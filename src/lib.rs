#![feature(file_create_new)]
use std::{
    error::Error,
    path::{Path, PathBuf},
};

use commits::{CommitsFifoIter, CommitsLifoIter};
use compression::Decompression;

use packreader::PackReader;
use refs::GitRef;
use shared::ObjectHash;

mod commits;
mod compression;
pub mod ffi;
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

        Repository {
            path,
            pack_reader,
            decompression,
        }
    }

    pub fn write(mut file_path: PathBuf, object: impl WriteObject) {
        let hash = object.hash().to_string();
        let data = object.to_bytes();
        let prefix = object.prefix();

        file_path.push("objects");
        file_path.push(&hash[0..2]);

        std::fs::create_dir_all(&file_path).unwrap();

        file_path.push(&hash[2..]);
        if !Path::new(&file_path).exists() {
            compression::pack_file(&file_path, prefix, data);
        }
    }

    pub fn commits_topo(&mut self) -> CommitsFifoIter {
        CommitsFifoIter::create(&self.path, &self.pack_reader, &mut self.decompression)
    }

    pub fn commits_lifo(&mut self) -> CommitsLifoIter {
        CommitsLifoIter::create(&self.path, &self.pack_reader, &mut self.decompression)
    }

    pub fn _refs(&self) -> Result<Vec<GitRef>, Box<dyn Error>> {
        GitRef::read_all(&self.path)
    }
}
