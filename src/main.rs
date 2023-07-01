use std::{error::Error, fmt::Display, io::BufWriter, path::PathBuf};

use clap::{ArgGroup, Parser, Subcommand};
use gitrw::Repository;
use mimalloc::MiMalloc;
use rustc_hash::FxHashSet;
use std::io::Write;

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
    print_locked(repository.commits_ordered()
        .map(|commit| commit.tree()))?;

    Ok(())
}
