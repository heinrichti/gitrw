use std::{
    collections::HashMap,
    error::Error,
    hash::BuildHasher,
    path::PathBuf,
    sync::mpsc::{channel, Sender},
    thread,
};

use rustc_hash::FxHashMap;

use libgitrw::{
    objs::{CommitEditable, CommitHash, TreeHash},
    Repository, WriteObject,
};

fn get_parent_if_empty_commit<T: BuildHasher>(
    commit: &CommitEditable,
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
    tx: Sender<WriteObject>,
) -> FxHashMap<CommitHash, CommitHash> {
    let mut rewritten_commits: FxHashMap<CommitHash, CommitHash> = FxHashMap::default();
    let mut commit_trees: FxHashMap<CommitHash, TreeHash> = FxHashMap::default();

    for mut commit in repository.commits_topo().map(CommitEditable::create) {
        if let Some(parent) = get_parent_if_empty_commit(&commit, &rewritten_commits, &commit_trees)
        {
            rewritten_commits.insert(commit.base_hash().clone(), parent);
            continue;
        }

        let base_hash = commit.base_hash().clone();
        commit
            .parents()
            .iter()
            .map(|parent| rewritten_commits.get(parent).unwrap_or(parent).clone())
            .enumerate()
            .for_each(|(i, parent)| commit.set_parent(i, parent));

        let commit_tree = commit.tree();
        let w: WriteObject = commit.into();

        let new_hash: CommitHash = w.hash.clone().into();
        commit_trees.insert(new_hash.clone(), commit_tree);

        if base_hash != new_hash {
            rewritten_commits.insert(base_hash, new_hash.clone());
            tx.send(w).unwrap();
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

    if !rewritten_commits.is_empty() {
        repository.update_refs(&rewritten_commits, dry_run);
        Repository::write_rewritten_commits_file(rewritten_commits, dry_run);
    }

    Ok(())
}
