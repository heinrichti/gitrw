use crate::{shared::{ObjectHash, RefSlice, SliceIndexes}, WriteBytes};

use self::tree::TreeLineIndex;

mod commit;
mod tag;
mod tree;

#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub struct TreeHash(pub(crate) ObjectHash);

impl From<TreeHash> for ObjectHash {
    fn from(val: TreeHash) -> Self {
        val.0
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub struct CommitHash(pub(crate) ObjectHash);

#[derive(Debug)]
pub struct CommitEditable {
    base: CommitBase,
    tree: Option<TreeHash>,
    pub parents: Vec<Option<CommitHash>>,
    author: Option<Vec<u8>>,
    committer: Option<Vec<u8>>,
}

#[derive(Debug)]
pub struct CommitBase {
    pub hash: CommitHash,
    bytes: WriteBytes,
    pub tree_line: SliceIndexes,
    pub parents: Vec<SliceIndexes>,
    pub author: SliceIndexes,
    pub author_time: SliceIndexes,
    pub committer: SliceIndexes,
    pub committer_time: SliceIndexes,
    pub remainder: SliceIndexes,
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
    Commit(CommitBase),
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
    object_hash: TreeHash,
    lines: Vec<TreeLineIndex>,
    bytes: Box<[u8]>,
    bytes_start: usize,
}

impl Tree {
    pub fn hash(&self) -> &TreeHash {
        &self.object_hash
    }
}
