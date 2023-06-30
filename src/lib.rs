use std::{error::Error, io::{Write, BufWriter}, path::Path, fmt::Display};

use gitrwlib::repository::Repository;
use rustc_hash::FxHashSet;

mod gitrwlib;

fn print_locked<T: Display>(items: impl Iterator<Item = T>) -> Result<(), Box<dyn Error>> {
    let lock = std::io::stdout().lock();
    let mut handle = BufWriter::new(lock);

    for item in items {
        writeln!(handle, "{item}")?;
    }

    Ok(())
}

pub fn list_contributors(repository_path: &Path) -> Result<(), Box<dyn Error>> {
    let mut repository = Repository::create(repository_path);
    let mut committers = FxHashSet::default();

    for commit in repository.commits_lifo() {
        committers.insert(commit.committer().to_owned());
        committers.insert(commit.author().to_owned());
    }

    let mut committers: Vec<_> = committers.iter().collect();
    committers.sort();

    print_locked(committers.into_iter())?;

    Ok(())
}

// pub fn print_tree(repository_path: &Path, object_hash: ObjectHash) -> Result<(), Box<dyn Error>> {
//     let pack_reader = PackReader::create(repository_path)?;
//     let mut compression = Compression::new();

//     let obj = pack_reader
//         .read_git_object(&mut compression, object_hash.clone());

//     if obj.is_some() {
//         match obj.unwrap() {
//             GitObject::Tree(tree) => println!("{tree}"),
//             _ => panic!(),
//         };
//     }
//     else {
//         if let Ok(bytes) = compression.from_file(repository_path, &object_hash.to_string()) {
//             let tree = Tree::create(object_hash, bytes, true);
//             println!("{tree}");
//         } else { panic!() };
//     }

//     Ok(())
// }

pub fn remove_empty_commits(repository_path: &Path) -> Result<(), Box<dyn Error>> {
    let mut repository = Repository::create(repository_path);
    print_locked(repository.commits_ordered())?;

    Ok(())
}