use std::{
    collections::HashMap,
    error::Error,
    hash::{BuildHasher, Hasher},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

use commits::{CommitsFifoIter, CommitsLifoIter};
use compression::Decompression;

use objs::{CommitEditable, CommitBase, CommitHash, GitObject, Tag, Tree};
use packreader::PackReader;
use rayon::prelude::{ParallelBridge, ParallelIterator};
use refs::GitRef;
use rs_sha1::{HasherContext, Sha1Hasher};
use shared::ObjectHash;

mod commits;
mod compression;
// pub mod ffi;
mod idx_reader;
mod pack_diff;
mod packreader;
mod refs;
mod shared;

pub mod objs;

pub struct Repository {
    path: PathBuf,
    pack_reader: PackReader,
}

impl Clone for Repository {
    fn clone(&self) -> Self {
        Self {
            path: self.path.clone(),
            pack_reader: self.pack_reader.clone(),
        }
    }
}

#[derive(Debug)]
pub struct WriteBytes {
    bytes: Box<[u8]>,
    start: usize,
}

pub struct WriteObject {
    pub hash: ObjectHash,
    prefix: String,
    bytes: WriteBytes,
}

impl From<CommitEditable> for WriteObject {
    fn from(value: CommitEditable) -> Self {
        let wb = value.to_bytes();
        Self {
            hash: calculate_hash(&wb.bytes, b"commit"),
            prefix: String::from("commit"),
            bytes: wb,
        }
    }
}

impl From<Tag> for WriteObject {
    fn from(value: Tag) -> Self {
        Self {
            hash: value.hash().clone(),
            prefix: String::from("tag"),
            bytes: value.bytes(),
        }
    }
}

impl From<Tree> for WriteObject {
    fn from(value: Tree) -> Self {
        Self {
            hash: value.hash().0.clone(),
            prefix: String::from("tree"),
            bytes: value.bytes(),
        }
    }
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

        Repository {
            path,
            pack_reader,
        }
    }

    pub fn read_object(&self, hash: ObjectHash) -> Option<GitObject> {
        let mut compression = Decompression::default();
        commits::read_object_from_hash(&mut compression, &self.path, &self.pack_reader, hash)
    }

    pub fn write(mut repo_path: PathBuf, object: WriteObject, dry_run: bool) {
        if dry_run {
            return;
        }

        let hash = object.hash.to_string();
        let data = object.bytes;
        let prefix = object.prefix;

        repo_path.push("objects");
        repo_path.push(&hash[0..2]);

        std::fs::create_dir_all(&repo_path).unwrap();

        repo_path.push(&hash[2..]);
        if !Path::new(&repo_path).exists() {
            compression::pack_file(&repo_path, prefix.as_str(), &data);
        }
    }

    pub fn write_commits(
        repository_path: PathBuf,
        commits: impl Iterator<Item = WriteObject> + Send,
        dry_run: bool,
    ) {
        commits
            .par_bridge()
            .for_each(|commit| {
                Self::write(repository_path.clone(), commit, dry_run);
            });
    }

    pub fn write_trees(
        repository_path: PathBuf,
        trees: impl Iterator<Item = objs::Tree> + Send,
        dry_run: bool,
    ) {
        trees
            .par_bridge()
            .for_each(|tree| {
                Self::write(repository_path.clone(), tree.into(), dry_run);
            });
    }

    pub fn commits_topo(&self) -> impl Iterator<Item = CommitBase> + '_ {
        CommitsFifoIter::create(&self.path, &self.pack_reader, Decompression::default())
    }

    pub fn commits_lifo(&self) -> impl Iterator<Item = CommitBase> + '_ {
        CommitsLifoIter::create(&self.path, &self.pack_reader, Decompression::default())
    }

    pub fn refs(&self) -> Result<Vec<GitRef>, Box<dyn Error>> {
        GitRef::read_all(&self.path)
    }

    pub fn update_refs<T: BuildHasher>(
        &self,
        rewritten_commits: &HashMap<CommitHash, CommitHash, T>,
        dry_run: bool
    ) {
        if !dry_run {
            refs::GitRef::update(self, rewritten_commits, dry_run);
        }
    }

    pub fn write_rewritten_commits_file(
        rewritten_commits: HashMap<
            CommitHash,
            CommitHash,
            std::hash::BuildHasherDefault<rustc_hash::FxHasher>,
        >,
        dry_run: bool
    ) {
        if dry_run {
            return;
        }

        let file = std::fs::File::create("object-id-map.old-new.txt").unwrap();
        let mut writer = BufWriter::new(file);
        for (old, new) in rewritten_commits.iter() {
            writer.write_fmt(format_args!("{old} {new}\n")).unwrap();
        }

        println!("object-id-map.old-new.txt written");
    }
}
