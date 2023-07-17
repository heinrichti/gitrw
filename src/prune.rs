use std::{
    collections::HashMap,
    error::Error,
    hash::BuildHasher,
    io::{BufWriter, Write},
    path::PathBuf,
    sync::mpsc::{channel, Sender},
    thread,
};

use rustc_hash::FxHashMap;

use crate::{
    objs::{Commit, CommitHash, TreeHash},
    refs, Repository,
};

fn parent_if_empty<T: BuildHasher>(
    commit: &Commit,
    rewritten_commits: &HashMap<CommitHash, CommitHash, T>,
    commit_trees: &HashMap<CommitHash, TreeHash, T>,
) -> Option<CommitHash> {
    let parents = commit.parents();
    if parents.len() == 1 {
        let commit_tree = commit.tree();
        let parent = parents.first().unwrap();
        let parent = rewritten_commits.get(parent).unwrap_or(parent).clone();

        let parent_tree = &commit_trees[&parent];
        if parent_tree == &commit_tree {
            Some(parent)
        } else {
            None
        }
    } else {
        None
    }
}

fn find_empty_commits(
    repository: &mut Repository,
    tx: Sender<Commit>,
) -> FxHashMap<CommitHash, CommitHash> {
    let mut rewritten_commits: FxHashMap<CommitHash, CommitHash> = FxHashMap::default();
    let mut commit_trees: FxHashMap<CommitHash, TreeHash> = FxHashMap::default();

    for mut commit in repository.commits_topo() {
        if let Some(parent) = parent_if_empty(&commit, &rewritten_commits, &commit_trees) {
            rewritten_commits.insert(commit.hash().clone(), parent);
            continue;
        }

        let commit_hash = commit.hash().clone();
        commit
            .parents()
            .iter()
            .map(|parent| rewritten_commits.get(parent).unwrap_or(parent).clone())
            .enumerate()
            .for_each(|(i, parent)| commit.set_parent(i, parent));

        let commit = Commit::create(None, commit.to_bytes(), false);
        commit_trees.insert(commit.hash().clone(), commit.tree());

        if &commit_hash != commit.hash() {
            rewritten_commits.insert(commit_hash, commit.hash().clone());
            tx.send(commit).unwrap();
        }
    }

    rewritten_commits
}

pub fn remove_empty_commits(repository_path: PathBuf, dry_run: bool) -> Result<(), Box<dyn Error>> {
    let write_path = repository_path.clone();
    let (tx, rx) = channel();

    let thread =
        thread::spawn(move || Repository::write_commits(write_path, rx.into_iter(), dry_run));

    let mut repository = Repository::create(repository_path);
    let rewritten_commits = find_empty_commits(&mut repository, tx);

    thread.join().unwrap();

    refs::GitRef::update(&mut repository, &rewritten_commits);

    write_rewritten_commits(rewritten_commits);

    Ok(())
}

fn write_rewritten_commits(
    rewritten_commits: HashMap<
        CommitHash,
        CommitHash,
        std::hash::BuildHasherDefault<rustc_hash::FxHasher>,
    >,
) {
    let file = std::fs::File::create("object-id-map.old-new.txt").unwrap();
    let mut writer = BufWriter::new(file);
    for (old, new) in rewritten_commits.iter() {
        writer.write_fmt(format_args!("{old} {new}\n")).unwrap();
    }

    println!("object-id-map.old-new.txt written");
}
