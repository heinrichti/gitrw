#![feature(file_create_new)]
use std::{
    error::Error,
    path::{Path, PathBuf},
};

use commits::{CommitsFifoIter, CommitsLifoIter};
use compression::{Compression, Decompression};
use objs::Commit;
use packreader::PackReader;
use refs::GitRef;
use shared::ObjectHash;

mod commits;
mod compression;
mod idx_reader;
mod pack_diff;
mod packreader;
mod refs;
mod shared;

pub mod objs;

pub struct Repository {
    path: PathBuf,
    pack_reader: PackReader,
    decompression: Decompression,
    compression: Compression,
}

pub trait WriteObject {
    fn hash(&self) -> &ObjectHash;
    fn to_bytes(&self) -> &[u8];
    fn prefix(&self) -> &str;
}

impl Repository {
    pub fn create(path: PathBuf) -> Self {
        let pack_reader = PackReader::create(&path).unwrap();
        let decompression = Decompression::default();
        let compression = Compression::default();

        Repository {
            path,
            pack_reader,
            decompression,
            compression,
        }
    }

    pub fn write(&mut self, object: impl WriteObject) {
        let hash = object.hash().to_string();
        let data = object.to_bytes();
        let prefix = object.prefix();

        let mut file_path = self.path.clone();
        file_path.push("objects");
        file_path.push(&hash[0..2]);

        std::fs::create_dir_all(&file_path).unwrap();

        file_path.push(&hash[2..]);
        if !Path::new(&file_path).exists() {
            self.compression.pack_file(&file_path, prefix, &data);
        }
    }

    pub fn commits_topo<'a, 'b>(&'a mut self) -> CommitsFifoIter<'a, 'b> {
        CommitsFifoIter::<'a, 'b>::create(&self.path, &self.pack_reader, &mut self.decompression)
    }

    pub fn commits_lifo(&mut self) -> impl Iterator<Item = Commit> {
        CommitsLifoIter::create(&self.path, &self.pack_reader, &mut self.decompression)
    }

    pub fn _refs(&self) -> Result<Vec<GitRef>, Box<dyn Error>> {
        GitRef::read_all(&self.path)
    }
}
