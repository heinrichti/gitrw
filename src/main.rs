use std::{
    collections::HashMap,
    error::Error,
    fmt::Display,
    hash::BuildHasher,
    io::BufWriter,
    path::PathBuf,
    sync::mpsc::{channel, Receiver, Sender},
    thread,
};

use clap::{ArgGroup, Parser, Subcommand};
use gitrw::{
    objs::{Commit, CommitHash, TreeHash},
    Repository,
};
#[cfg(not(test))]
use mimalloc::MiMalloc;
use rustc_hash::{FxHashMap, FxHashSet};
use std::io::Write;

#[cfg(not(test))]
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[derive(clap::Parser)]
struct Cli {
    /// Path to the mirrored/bare repository (do not use on a repository with a working copy)
    repository: Option<String>,

    #[command(subcommand)]
    command: Commands,

    /// Do not change the repository.
    #[arg(short, long)]
    dry_run: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Contributor related actions like list and rewrite
    #[command(subcommand)]
    Contributor(ContributorArgs),

    /// Remove files and whole directories from the repository
    #[command(group(ArgGroup::new("input")
                        .required(true)
                        .multiple(true)))]
    Remove {
        /// File to remove. Argument can be specified multiple times
        #[arg(short, long, group = "input")]
        file: Option<Vec<String>>,

        /// Directory to remove. Argument can be specified multiple times
        #[arg(short, long, group = "input")]
        directory: Option<Vec<String>>,
    },

    /// Remove empty commits that are no merge commits
    PruneEmpty,
}

#[derive(Subcommand)]
enum ContributorArgs {
    /// Lists all authors and committers
    List,
    /// Allows to rewrite contributors
    Rewrite {
        /// Format inside file: Old User <old@user.mail> = New User <new@user.mail>
        mapping_file: String,
    },
}

fn main() {
    let cli = Cli::parse();

    let repository_path = if let Some(repository_path) = &cli.repository {
        PathBuf::from(repository_path)
    } else {
        PathBuf::from(".")
    };

    match cli.command {
        Commands::Contributor(args) => match args {
            ContributorArgs::List => list_contributors(repository_path).unwrap(),
            ContributorArgs::Rewrite { mapping_file: file } => {
                println!("rewrite from file: {file}");
                todo!();
            }
        },
        Commands::Remove { file, directory } => {
            if let Some(dir) = directory {
                for d in dir {
                    println!("Deleting directory: {d}");
                }
            }

            if let Some(file) = file {
                for f in file {
                    println!("Deleting file: {f}");
                }
            }

            todo!();
        }

        Commands::PruneEmpty => {
            println!("Pruning empty commits");

            remove_empty_commits(repository_path, cli.dry_run).unwrap();
        }
    };
}

fn print_locked<T: Display>(items: impl Iterator<Item = T>) -> Result<(), Box<dyn Error>> {
    let lock = std::io::stdout().lock();
    let mut handle = BufWriter::new(lock);

    for item in items {
        writeln!(handle, "{item}")?;
    }

    Ok(())
}

pub fn list_contributors(repository_path: PathBuf) -> Result<(), Box<dyn Error>> {
    let mut repository = Repository::create(repository_path);
    let mut committers = FxHashSet::default();

    for commit in repository.commits_lifo() {
        committers.insert(commit.committer().to_owned());
        committers.insert(commit.author().to_owned());
    }

    let mut committers: Vec<_> = committers.iter().collect();
    committers.sort();

    print_locked(committers.iter())?;

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

fn parent_if_empty<'a, T: BuildHasher>(
    commit: &'a Commit,
    rewritten_commits: &'a HashMap<CommitHash, CommitHash, T>,
    commit_trees: &'a HashMap<CommitHash, TreeHash, T>,
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

fn find_empty_commits(repository_path: PathBuf, tx: Sender<Commit>) {
    let mut repository = Repository::create(repository_path);
    let mut rewritten_commits: FxHashMap<CommitHash, CommitHash> = FxHashMap::default();
    let mut commit_trees: FxHashMap<CommitHash, TreeHash> = FxHashMap::default();

    for mut commit in repository.commits_topo() {
        if let Some(parent) = parent_if_empty(&commit, &rewritten_commits, &commit_trees) {
            println!("Empty commit {} -> {}", commit.hash(), parent);
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
}

pub fn remove_empty_commits(repository_path: PathBuf, dry_run: bool) -> Result<(), Box<dyn Error>> {
    let write_path = repository_path.clone();
    let (tx, rx) = channel();

    let thread = thread::spawn(move || find_empty_commits(repository_path.clone(), tx));
    write_commits(rx, dry_run, write_path);

    thread.join().unwrap();

    Ok(())
}

use rayon::prelude::*;

fn write_commits(rx: Receiver<Commit<'_>>, dry_run: bool, repository_path: PathBuf) {
    rx.into_iter()
        .filter(|_| !dry_run)
        .par_bridge()
        .for_each(|commit| {
            Repository::write(repository_path.clone(), commit);
        });
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc::channel;

    use bstr::ByteSlice;
    use gitrw::objs::{Commit, CommitHash};

    const BYTES: &[u8] = b"tree 31aa860596f003d69b896943677e9fe5ff208233\nparent 5eec99927bb6058c8180e5dac871c89c7d01b0ab\nauthor Tim Heinrich <2929650+TimHeinrich@users.noreply.github.com> 1688207675 +0200\ncommitter Tim Heinrich <2929650+TimHeinrich@users.noreply.github.com> 1688209149 +0200\n\nChanging of commit data\n";

    #[test]
    fn miri_commit() {
        let object_hash: CommitHash = b"53dd2e51161a4eebd8baacd17383c9af35a8283e"
            .as_bstr()
            .try_into()
            .unwrap();

        let mut commit = Commit::create(Some(object_hash), BYTES.into(), false);

        let author = commit.author().to_owned();
        commit.set_author(b"Test user".to_vec());

        let (sender, receiver) = channel();

        let thread = std::thread::spawn(move || {
            sender.send(commit).unwrap();
        });

        for mut commit in receiver {
            assert_eq!("Test user", commit.author());
            commit.set_author(author.clone().bytes().collect());
            let b = commit.to_bytes();
            assert_eq!(BYTES.to_vec().into_boxed_slice(), b);
        }

        thread.join().unwrap();
    }
}
