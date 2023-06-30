use std::error::Error;
use std::path::PathBuf;

use super::commit_walker::{CommitsFifoIter, CommitsLifoIter};
use super::hash_content::Compression;
use super::objs::commit::Commit;
use super::packreader::PackReader;
use super::refs::GitRef;

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
