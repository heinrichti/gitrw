use crate::shared::{ObjectHash, RefSlice};

use self::tree::TreeLine;

mod commit;
mod tag;
mod tree;

#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub struct TreeHash(ObjectHash);

#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub struct CommitHash(pub(crate) ObjectHash);

#[derive(Debug)]
pub struct Commit {
    hash: Option<CommitHash>,
    bytes: Box<[u8]>,
    tree_line: RefSlice<u8>,
    parents: Vec<RefSlice<u8>>,
    author: RefSlice<u8>,
    author_time: RefSlice<u8>,
    committer: RefSlice<u8>,
    committer_time: RefSlice<u8>,
    _remainder: RefSlice<u8>,
}

#[derive(Debug)]
pub struct Tag {
    bytes: Box<[u8]>,
    object: RefSlice<u8>,
    obj_type: RefSlice<u8>,
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
    Commit,
    Tree,
}

#[derive(Debug)]
pub struct Tree {
    _object_hash: TreeHash,
    lines: Vec<TreeLine>,
    _bytes: Box<[u8]>,
}
