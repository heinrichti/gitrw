use std::path::Path;

use clap::Parser;
use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[derive(clap::Parser)]
struct Cli {
    repository: String,
}

fn main() {
    let cli = Cli::parse();
    let repository_path = Path::new(&cli.repository);
    gitrw::list_contributors(repository_path).unwrap();
}
