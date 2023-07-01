use crate::shared::RefSlice;

use self::git_objects::TreeLine;

mod commit;
mod git_objects;
mod object_hash;
mod tag;

#[derive(Eq, PartialEq, Clone, Hash)]
pub struct ObjectHash {
    bytes: [u8; 20],
}

#[derive(Debug)]
pub struct Commit<'a> {
    pub object_hash: ObjectHash,
    _bytes: Box<[u8]>,
    tree_line: RefSlice<'a, u8>,
    parents: Vec<RefSlice<'a, u8>>,
    author_line: RefSlice<'a, u8>,
    committer_line: RefSlice<'a, u8>,
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
    _object_hash: ObjectHash,
    lines: Vec<TreeLine<'a>>,
    _bytes: Box<[u8]>,
}
