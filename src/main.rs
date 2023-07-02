use std::{error::Error, fmt::Display, io::BufWriter, path::PathBuf};

use clap::{ArgGroup, Parser, Subcommand};
use gitrw::{Repository, objs::{CommitHash, TreeHash}};
#[cfg(not(test))]
use mimalloc::MiMalloc;
use rustc_hash::{FxHashSet, FxHashMap};
use std::io::Write;

#[cfg(not(test))]
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[derive(clap::Parser)]
struct Cli {
    repository: Option<String>,

    #[command(subcommand)]
    command: Commands,
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

            remove_empty_commits(repository_path).unwrap();
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

    print_locked(committers.iter().into_iter())?;

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

pub fn remove_empty_commits(repository_path: PathBuf) -> Result<(), Box<dyn Error>> {
    let mut repository = Repository::create(repository_path);

    // let mut commits_to_rewrite: FxHashMap<CommitHash, CommitHash> = FxHashMap::default();
    let mut commit_trees: FxHashMap<CommitHash, TreeHash> = FxHashMap::default();

    for commit in repository.commits_ordered() {
        let parents = commit.parents();
        if parents.len() == 1 {
            let commit_tree = commit.tree();
            commit_trees.get(parents.first().unwrap()).map(|parent_tree| {
                if parent_tree == &commit_tree {
                    // let parent_hash = get_rewritten_commit()
                    println!("Empty commit: {}", commit.object_hash);
                    // commits_to_rewrite.insert(commit.object_hash, parent_hash)
                }
            });

            commit_trees.insert(commit.object_hash, commit_tree);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc::channel;

    use bstr::ByteSlice;
    use gitrw::objs::Commit;

    const BYTES: &[u8] = b"tree 31aa860596f003d69b896943677e9fe5ff208233\nparent 5eec99927bb6058c8180e5dac871c89c7d01b0ab\nauthor Tim Heinrich <2929650+TimHeinrich@users.noreply.github.com> 1688207675 +0200\ncommitter Tim Heinrich <2929650+TimHeinrich@users.noreply.github.com> 1688209149 +0200\n\nChanging of commit data";

    #[test]
    fn miri_commit() {
        let commit = Commit::create(
            b"53dd2e51161a4eebd8baacd17383c9af35a8283e"
                .as_bstr()
                .try_into()
                .unwrap(),
            BYTES.into(),
            false,
        );

        let (sender, receiver) = channel();

        let thread = std::thread::spawn(move || {
            sender.send(commit).unwrap();
        });

        for commit in receiver {
            println!("{}: {}", commit.object_hash, commit.author());
            let b = commit.to_bytes();
            assert_eq!(BYTES.to_vec().into_boxed_slice(), b);
        }

        thread.join().unwrap();
    }
}
