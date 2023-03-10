use std::slice;
use std::{fmt::Display, ops::Range};

use bstr::ByteSlice;

use crate::object_hash::ObjectHash;
use crate::objs::tag::Tag;

use super::commit::Commit;

#[derive(Debug)]
pub enum GitObject {
    Commit(Commit),
    Tree(Tree),
    // Blob(Blob),
    Tag(Tag),
}

// pub struct Blob { object_hash: ObjectHash }

// impl Blob {
//     pub fn create(_object_hash: ObjectHash, _bytes: Vec<u8>) -> Blob {
//         todo!()
//     }
// }

// impl GitObject {
//     pub fn hash(&self) -> &ObjectHash {
//         match self {
//             GitObject::Commit(commit) => &commit.object_hash,
//             GitObject::Tag(tag) => &tag.object_hash,
//             GitObject::Tree(tree) => &tree.object_hash,
//             GitObject::Blob(blob) => &blob.object_hash,
//         }
//     }
// }

#[derive(PartialEq, Eq)]
pub enum TagTargetType {
    Commit,
    Tree,
}

#[derive(Debug)]
pub struct Tree {
    _object_hash: ObjectHash,
    lines: Vec<TreeLine>,
    _bytes: Box<[u8]>,
}

impl Display for Tree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for line in self.lines.iter() {
            writeln!(
                f,
                "{} {}",
                line.hash,
                unsafe { slice::from_raw_parts(line.text.0, line.text.1) }.as_bstr(),
                // self.bytes[line.text.clone()].as_bstr()
            )?;
        }
        Ok(())
    }
}

#[derive(Debug)]
struct TreeLine {
    hash: ObjectHash,
    text: (*const u8, usize)
}

impl Tree {
    pub fn create(object_hash: ObjectHash, bytes: Box<[u8]>, skip_first_null: bool) -> Tree {
        let mut position = 0;

        if skip_first_null {
            position = bytes.iter().position(|x| *x == b'\0').unwrap() + 1;
        }

        let mut null_terminator_index_opt = bytes[position..].iter().position(|x| *x == b'\0');
        let mut lines = Vec::new();

        while let Some(null_terminator_index) = null_terminator_index_opt {
            let text = (
                unsafe { bytes.as_ptr().add(position) },
                null_terminator_index);
                
            let tree_hash: ObjectHash = 
                bytes[position + null_terminator_index + 1..position + null_terminator_index + 21].into();

            position += null_terminator_index + 21;

            lines.push(TreeLine {
                hash: tree_hash,
                text,
            });

            null_terminator_index_opt = bytes[position..].iter().position(|x| *x == b'\0');
        }

        Tree {
            _object_hash: object_hash,
            lines,
            _bytes: bytes,
        }
    }
}
