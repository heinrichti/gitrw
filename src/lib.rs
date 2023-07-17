use std::{
    error::Error,
    hash::Hasher,
    path::{Path, PathBuf},
};

use commits::{CommitsFifoIter, CommitsLifoIter};
use compression::Decompression;

use objs::{Commit, GitObject};
use packreader::PackReader;
use rayon::prelude::{ParallelBridge, ParallelIterator};
use refs::GitRef;
use rs_sha1::{HasherContext, Sha1Hasher};
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

pub mod prune;

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

pub fn calculate_hash(data: &[u8], prefix: &[u8]) -> ObjectHash {
    let mut hasher = Sha1Hasher::default();
    hasher.write(prefix);
    hasher.write(b" ");
    hasher.write(data.len().to_string().as_bytes());
    hasher.write(b"\0");
    hasher.write(data);
    let bytes = HasherContext::finish(&mut hasher);
    let bytes: [u8; 20] = bytes.into();
    ObjectHash::from(bytes)
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

    pub fn read_object(&mut self, hash: ObjectHash) -> Option<GitObject> {
        commits::read_object_from_hash(&mut self.decompression, &self.path, &self.pack_reader, hash)
    }

    pub fn write(mut file_path: PathBuf, object: &impl WriteObject) {
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

    pub fn write_commits(
        repository_path: PathBuf,
        commits: impl Iterator<Item = Commit> + Send,
        dry_run: bool,
    ) {
        commits
            .par_bridge()
            .filter(|_| !dry_run)
            .for_each(|commit| {
                Self::write(repository_path.clone(), &commit);
            });
    }

    pub fn commits_topo(&mut self) -> CommitsFifoIter {
        CommitsFifoIter::create(&self.path, &self.pack_reader, &mut self.decompression)
    }

    pub fn commits_lifo(&mut self) -> CommitsLifoIter {
        CommitsLifoIter::create(&self.path, &self.pack_reader, &mut self.decompression)
    }

    pub fn refs(&self) -> Result<Vec<GitRef>, Box<dyn Error>> {
        GitRef::read_all(&self.path)
    }
}
