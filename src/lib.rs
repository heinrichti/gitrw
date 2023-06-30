use std::{error::Error, io::{Write, BufWriter}, path::Path};

use hash_content::Compression;
use object_hash::ObjectHash;
use objs::git_objects::{GitObject, Tree};
use packreader::PackReader;
use repository::Repository;
use rustc_hash::FxHashSet;

use crate::objs::git_objects;

mod commit_walker;
mod hash_content;
mod idx_reader;
pub mod object_hash;
mod objs;
mod pack_diff;
mod packreader;
mod refs;
mod repository;

pub fn list_contributors(repository_path: &Path) -> Result<(), Box<dyn Error>> {
    let mut repository = Repository::create(repository_path);
    let mut committers = FxHashSet::default();

    for commit in repository.commits_lifo() {
        committers.insert(commit.committer().to_owned());
        committers.insert(commit.author().to_owned());
    }

    let mut committers: Vec<_> = committers.iter().collect();
    committers.sort();

    let lock = std::io::stdout().lock();
    let mut handle = BufWriter::new(lock);
    for committer in committers {
        writeln!(handle, "{committer}")?;
    }

    Ok(())
}

pub fn print_tree(repository_path: &Path, object_hash: ObjectHash) -> Result<(), Box<dyn Error>> {
    let pack_reader = PackReader::create(repository_path)?;
    let mut compression = Compression::new();

    let obj = pack_reader
        .read_git_object(&mut compression, object_hash.clone());

    if obj.is_some() {
        match obj.unwrap() {
            GitObject::Tree(tree) => println!("{tree}"),
            _ => panic!(),
        };
    }
    else {
        if let Ok(bytes) = compression.from_file(repository_path, &object_hash.to_string()) {
            let tree = Tree::create(object_hash, bytes, true);
            println!("{tree}");
        } else { panic!() };
    }

    Ok(())
}

pub fn remove_empty_commits(repository_path: &Path) -> Result<(), Box<dyn Error>> {
    let mut repository = Repository::create(repository_path);
    let commits = repository.commits_ordered();

    let lock = std::io::stdout().lock();
    let mut handle = BufWriter::new(lock);
    for commit in commits.into_iter() {
        writeln!(handle, "{0}", commit.object_hash)?;
    }

    Ok(())
}