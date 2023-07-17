use crate::shared::{ObjectHash, RefSlice};

use self::tree::TreeLine;

mod commit;
mod tag;
mod tree;

#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub struct TreeHash(pub(crate) ObjectHash);

#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub struct CommitHash(pub(crate) ObjectHash);

#[derive(Debug)]
pub struct Commit {
    hash: Option<CommitHash>,
    bytes: Box<[u8]>,
    bytes_start: usize,
    tree_line: RefSlice<u8>,
    parents: Vec<RefSlice<u8>>,
    author: RefSlice<u8>,
    author_time: RefSlice<u8>,
    committer: RefSlice<u8>,
    committer_time: RefSlice<u8>,
    remainder: RefSlice<u8>,
}

#[derive(Debug)]
pub struct Tag {
    hash: Option<ObjectHash>,
    bytes: Box<[u8]>,
    bytes_start: usize,
    object: RefSlice<u8>,
    obj_type: RefSlice<u8>,
    tag_name: RefSlice<u8>,
    remainder: RefSlice<u8>,
}

#[derive(Debug)]
pub enum GitObject {
    Commit(Commit),
    Tree(Tree),
    // Blob(Blob),
    Tag(Tag),
}

#[derive(PartialEq, Eq)]
pub enum TagTargetType {
    Tag,
    Commit,
    Tree,
}

#[derive(Debug)]
pub struct Tree {
    _object_hash: TreeHash,
    lines: Vec<TreeLine>,
    _bytes: Box<[u8]>,
}

impl Tree {
    pub fn hash(&self) -> &TreeHash {
        &self._object_hash
    }
}
