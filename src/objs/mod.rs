use std::marker::PhantomData;

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
    tree_line: (*const u8, usize),
    parents: Vec<(*const u8, usize)>,
    author_line: (*const u8, usize),
    committer_line: (*const u8, usize),
    remainder: (*const u8, usize),
    _phantom: PhantomData<&'a [u8]>,
}

#[derive(Debug)]
pub struct Tag {
    // object_hash: ObjectHash,
    _bytes: Box<[u8]>,
    // object: Range<usize>,
    // obj_type: Range<usize>,
    object: (*const u8, usize),
    obj_type: (*const u8, usize), // tag: Range<usize>,
                                  // tagger: Range<usize>,
                                  // message: Range<usize>
}

#[derive(Debug)]
pub enum GitObject<'a> {
    Commit(Commit<'a>),
    Tree(Tree<'a>),
    // Blob(Blob),
    Tag(Tag),
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
