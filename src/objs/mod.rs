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
pub struct Commit<'a> {
    pub object_hash: CommitHash,
    _bytes: Box<[u8]>,
    tree_line: RefSlice<'a, u8>,
    parents: Vec<RefSlice<'a, u8>>,
    author: RefSlice<'a, u8>,
    author_time: RefSlice<'a, u8>,
    committer: RefSlice<'a, u8>,
    committer_time: RefSlice<'a, u8>,
    _remainder: RefSlice<'a, u8>,
}

#[derive(Debug)]
pub struct Tag<'a> {
    // object_hash: ObjectHash,
    _bytes: Box<[u8]>,
    // object: Range<usize>,
    // obj_type: Range<usize>,
    object: RefSlice<'a, u8>,
    obj_type: RefSlice<'a, u8>, // tag: Range<usize>,
                                // tagger: Range<usize>,
                                // message: Range<usize>
}

#[derive(Debug)]
pub enum GitObject<'a> {
    Commit(Commit<'a>),
    Tree(Tree<'a>),
    // Blob(Blob),
    Tag(Tag<'a>),
}

#[derive(PartialEq, Eq)]
pub enum TagTargetType {
    Commit,
    Tree,
}

#[derive(Debug)]
pub struct Tree<'a> {
    _object_hash: TreeHash,
    lines: Vec<TreeLine<'a>>,
    _bytes: Box<[u8]>,
}
