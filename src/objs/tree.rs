use std::fmt::Display;

use bstr::{BStr, ByteSlice};

use crate::shared::{self, RefSlice};

use super::{ObjectHash, Tree, TreeHash};

impl<'a> Tree<'a> {
    pub fn create(object_hash: TreeHash, bytes: Box<[u8]>, skip_first_null: bool) -> Tree<'a> {
        let mut position = 0;

        if skip_first_null {
            position = bytes.iter().position(|x| *x == b'\0').unwrap() + 1;
        }

        let mut null_terminator_index_opt = bytes[position..].iter().position(|x| *x == b'\0');
        let mut lines = Vec::new();

        while let Some(null_terminator_index) = null_terminator_index_opt {
            let text = RefSlice::from_slice(&bytes[position..position + null_terminator_index]);

            let tree_hash: TreeHash = bytes
                [position + null_terminator_index + 1..position + null_terminator_index + 21]
                .try_into()
                .unwrap();

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

#[derive(Debug)]
pub struct TreeLine<'a> {
    hash: TreeHash,
    text: shared::RefSlice<'a, u8>,
}

impl Display for TreeHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.0))
    }
}

impl TryFrom<&[u8]> for TreeHash {
    type Error = &'static str;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        Ok(ObjectHash::try_from(value)?.into())
    }
}

impl TryFrom<&BStr> for TreeHash {
    type Error = &'static str;

    fn try_from(value: &BStr) -> Result<Self, Self::Error> {
        ObjectHash::try_from_bstr(value)
    }
}

impl From<ObjectHash> for TreeHash {
    fn from(value: ObjectHash) -> Self {
        TreeHash(value)
    }
}

impl Display for Tree<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for line in self.lines.iter() {
            writeln!(f, "{} {}", line.hash, line.text.as_bstr())?;
        }
        Ok(())
    }
}
