use std::{error::Error, path::PathBuf};

use commits::{CommitsFifoIter, CommitsLifoIter};
use hash_content::Compression;
use objs::commit::Commit;
use packreader::PackReader;
use refs::GitRef;

mod commits;
mod hash_content;
mod idx_reader;
mod pack_diff;
mod packreader;
mod refs;

pub mod objs;

pub struct Repository {
    path: PathBuf,
    pack_reader: PackReader,
    compression: Compression,
}

impl Repository {
    pub fn create(path: PathBuf) -> Self {
        let pack_reader = PackReader::create(&path).unwrap();
        let compression = Compression::new();

        Repository {
            path,
            pack_reader,
            compression,
        }
    }

    pub fn commits_ordered(&mut self) -> impl Iterator<Item = Commit> {
        CommitsFifoIter::create(&self.path, &self.pack_reader, &mut self.compression)
    }

    pub fn commits_lifo(&mut self) -> impl Iterator<Item = Commit> {
        CommitsLifoIter::create(&self.path, &self.pack_reader, &mut self.compression)
    }

    pub fn _refs(&self) -> Result<Vec<GitRef>, Box<dyn Error>> {
        GitRef::read_all(&self.path)
    }
}
