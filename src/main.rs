use std::path::Path;

use clap::{Parser, Subcommand, ArgGroup};
use mimalloc::MiMalloc;

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
        directory: Option<Vec<String>>
    },

    /// Remove empty commits that are no merge commits
    PruneEmpty
}

#[derive(Subcommand)]
enum ContributorArgs {
    /// Lists all authors and committers
    List,
    /// Allows to rewrite contributors 
    Rewrite {
        /// Format inside file: Old User <old@user.mail> = New User <new@user.mail>
        mapping_file: String
    }
}

fn main() {
    let cli = Cli::parse();
    let repository_path = 
        if let Some(repository_path) = &cli.repository {
            Path::new(repository_path)
        } else {
            Path::new(".")
        };

    match cli.command {
        Commands::Contributor(args) => match args {
            ContributorArgs::List => gitrw::list_contributors(repository_path).unwrap(), 
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
        },

        Commands::PruneEmpty => {
            println!("Pruning empty commits");
            todo!();
        }
    };
}
