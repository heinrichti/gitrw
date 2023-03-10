use std::{error::Error, io::{Write, BufWriter}, path::Path};

use commit_walker::CommitWalker;
use hash_content::Compression;
use object_hash::ObjectHash;
use objs::git_objects::GitObject;
use packreader::PackReader;
use rustc_hash::FxHashSet;

use crate::objs::git_objects;

mod commit_walker;
mod hash_content;
mod idx_reader;
mod object_hash;
mod objs;
mod pack_diff;
mod packreader;
mod refs;

pub fn list_contributors(repository_path: &Path) -> Result<(), Box<dyn Error>> {
    let commit_walker = CommitWalker::create(repository_path);

    let mut committers = FxHashSet::default();

    for commit in commit_walker {
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

    let obj = pack_reader.read_git_object(&mut compression, object_hash);

    match obj.unwrap() {
        GitObject::Tree(tree) => println!("{tree}"),
        _ => panic!(),
    };

    Ok(())
}
