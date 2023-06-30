use std::{path::Path};
use std::error::Error;

use crate::commit_walker::CommitsLifoIter;
use crate::{packreader::PackReader, hash_content::Compression, commit_walker::CommitsFifoIter, objs::commit::Commit};

pub struct Repository<'a> {
    path: &'a Path,
    pack_reader: PackReader,
    compression: Compression
}

impl<'a> Repository<'a> {
    pub fn create(path: &'a Path) -> Self {
        let pack_reader = PackReader::create(path).unwrap();
        let compression = Compression::new();

        Repository { path, pack_reader, compression }
    }

    pub fn commits_ordered(&mut self) -> impl Iterator<Item = Commit> {
        CommitsFifoIter::create(self.path, &self.pack_reader, &mut self.compression)
    }

    pub fn commits_lifo(&mut self) -> impl Iterator<Item = Commit> {
        CommitsLifoIter::create(self.path, &self.pack_reader, &mut self.compression)
    }

    pub fn _refs(&self) -> Result<Vec<crate::refs::GitRef>, Box<dyn Error>> {
        crate::refs::GitRef::read_all(self.path)
    }
}