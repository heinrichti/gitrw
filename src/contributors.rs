use std::{
    collections::HashMap, error::Error, io::stdin, path::PathBuf, sync::mpsc::channel,
    thread::spawn,
};

use bstr::{io::BufReadExt, BString, ByteSlice};
use libgitrw::{
    objs::{CommitEditable, CommitHash},
    Repository, WriteObject,
};
use rustc_hash::{FxHashMap, FxHashSet};

fn split_index(line: &[u8]) -> Option<usize> {
    for (pos, c) in line.iter().enumerate() {
        if *c == b'=' {
            return Some(pos);
        }
    }

    None
}

fn get_mappings() -> Result<FxHashMap<Vec<u8>, Vec<u8>>, Box<dyn Error>> {
    let mut mappings = FxHashMap::default();

    for line in stdin().lock().byte_lines() {
        let line = line?;
        let split_pos = split_index(&line).ok_or("Line is malformed. Pattern: old = new")?;

        let old = line[0..split_pos].trim().to_owned();
        let new = line[split_pos + 1..].trim().to_owned();

        if old != new {
            mappings.insert(old, new);
        }
    }

    Ok(mappings)
}

pub fn rewrite(repository_path: PathBuf, dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
    let mappings = get_mappings()?;

    let (tx, rx) = channel();
    let write_path = repository_path.clone();
    let write_thread =
        spawn(move || Repository::write_commits(write_path, rx.into_iter(), dry_run));

    let mut repository = Repository::create(repository_path);
    let mut rewritten_commits: HashMap<CommitHash, CommitHash, _> = FxHashMap::default();
    for mut commit in repository.commits_topo().map(CommitEditable::create) {
        if let Some(new_author) = mappings.get(commit.author_bytes()) {
            commit.set_author(new_author.clone());
        }

        if let Some(new_committer) = mappings.get(commit.committer_bytes()) {
            commit.set_committer(new_committer.clone());
        }

        for (i, parent) in commit.parents().iter().enumerate() {
            if let Some(new_commit_hash) = rewritten_commits.get(parent) {
                commit.set_parent(i, new_commit_hash.clone());
            }
        }

        if commit.has_changes() {
            let old_hash = commit.base_hash().clone();
            let w: WriteObject = commit.into();
            rewritten_commits.insert(old_hash, CommitHash::from(w.hash.clone()));
            tx.send(w).unwrap();
        }
    }

    drop(tx);
    write_thread.join().expect("Failed to write commits");

    if !rewritten_commits.is_empty() {
        repository.update_refs(&rewritten_commits, dry_run);
        Repository::write_rewritten_commits_file(rewritten_commits, dry_run);
    }

    Ok(())
}

pub fn get_contributors(repository_path: PathBuf) -> Result<Vec<BString>, Box<dyn Error>> {
    let mut committers = FxHashSet::default();
    let repository = Repository::create(repository_path);

    for commit in repository.commits_lifo() {
        committers.insert(commit.committer().to_owned());
        committers.insert(commit.author().to_owned());
    }

    let mut committers: Vec<_> = committers.into_iter().collect();
    committers.sort();

    Ok(committers)
}
