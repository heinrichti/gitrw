use std::{collections::HashMap, error::Error, path::PathBuf, sync::mpsc::channel, thread::spawn};

use bstr::{io::BufReadExt, ByteSlice};
use libgitrw::{
    objs::{Commit, CommitHash},
    Repository,
};
use rustc_hash::FxHashMap;

fn split_index(line: &[u8]) -> Option<usize> {
    for (pos, c) in line.iter().enumerate() {
        if *c == b'=' {
            return Some(pos);
        }
    }

    None
}

fn get_mappings(file_path: &str) -> Result<FxHashMap<Vec<u8>, Vec<u8>>, Box<dyn Error>> {
    let file = std::io::BufReader::new(std::fs::File::open(file_path).unwrap());

    let mut mappings = FxHashMap::default();

    for line in file.byte_lines() {
        let line = line?;
        let split_pos = split_index(&line).ok_or("test")?;

        let old = line[0..split_pos].trim().to_owned();
        let new = line[split_pos + 1..].trim().to_owned();
        let _ = mappings.insert(old, new);
    }

    Ok(mappings)
}

pub fn rewrite(
    repository_path: PathBuf,
    file_path: &str,
    dry_run: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mappings = get_mappings(file_path)?;

    let (tx, rx) = channel();
    let write_path = repository_path.clone();
    let write_thread =
        spawn(move || Repository::write_commits(write_path, rx.into_iter(), dry_run));

    let mut repository = Repository::create(repository_path);
    let mut rewritten_commits: HashMap<CommitHash, CommitHash, _> = FxHashMap::default();
    for mut commit in repository.commits_topo() {
        let old_hash = commit.hash().clone();
        let mut changed = false;

        if let Some(new_author) = mappings.get(commit.author_bytes()) {
            commit.set_author(new_author.clone());
            changed = true;
        }

        if let Some(new_committer) = mappings.get(commit.committer_bytes()) {
            commit.set_author(new_committer.clone());
            changed = true;
        }

        for (i, parent) in commit.parents().iter().enumerate() {
            if let Some(new_commit_hash) = rewritten_commits.get(parent) {
                commit.set_parent(i, new_commit_hash.clone());
                changed = true;
            }
        }

        if changed {
            commit = Commit::create(None, commit.to_bytes(), false);
            rewritten_commits.insert(old_hash, commit.hash().clone());
            tx.send(commit).unwrap();
        }
    }

    drop(tx);
    write_thread.join().expect("Failed to write commits");

    if !rewritten_commits.is_empty() && !dry_run {
        repository.update_refs(&rewritten_commits);
        Repository::write_rewritten_commits_file(rewritten_commits);
    }

    Ok(())
}
