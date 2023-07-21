use std::fmt::Display;

use bstr::{BStr, ByteSlice, ByteVec};

use crate::shared::{self, RefSlice};

use super::{ObjectHash, Tree, TreeHash};

impl Tree {
    pub fn create(object_hash: TreeHash, bytes: Box<[u8]>, skip_first_null: bool) -> Tree {
        let mut position = 0;

        if skip_first_null {
            position = bytes.iter().position(|x| *x == b'\0').unwrap() + 1;
        }

        let mut null_terminator_index_opt = bytes[position..].iter().position(|x| *x == b'\0');
        let mut lines = Vec::new();

        while let Some(null_terminator_index) = null_terminator_index_opt {
            let text = RefSlice::new(position, null_terminator_index);

            let tree_hash: TreeHash = bytes
                [position + null_terminator_index + 1..position + null_terminator_index + 21]
                .try_into()
                .unwrap();

            position += null_terminator_index + 21;

            lines.push(TreeLineIndex {
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

    pub fn lines(&self) -> impl Iterator<Item = TreeLine> {
        self.lines.iter().map(|tree_line_index| TreeLine {
            hash: &tree_line_index.hash,
            text: tree_line_index.text.get(&self._bytes).as_bstr(), // text: self._bytes.get(tree_line_index.text),
        })
    }
}

impl<'a> FromIterator<TreeLine<'a>> for Tree {
    fn from_iter<T: IntoIterator<Item = TreeLine<'a>>>(iter: T) -> Self {
        let mut buf: Vec<u8> = Vec::new();
        for line in iter {
            buf.push_str(line.text);
            buf.push(b'\0');
            for c in line.hash.0.bytes {
                buf.push(c);
            }
        }

        let object_hash = crate::calculate_hash(&buf, b"tree");

        Self::create(TreeHash(object_hash), buf.into_boxed_slice(), false)
    }
}

pub struct TreeLine<'a> {
    pub hash: &'a TreeHash,
    pub text: &'a BStr,
}

impl<'a> TreeLine<'a> {
    pub fn is_tree(&self) -> bool {
        self.text[0] != b'1'
    }

    pub fn filename(&self) -> &[u8] {
        let seperator_index = self.text.iter().position(|c| *c == b' ').unwrap();
        &self.text[seperator_index + 1..]
    }
}

impl<'a> Display for TreeLine<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let null_index = self.text.iter().position(|c| *c == b' ').unwrap();
        f.write_fmt(format_args!(
            "{}{} {} {}\t{}",
            if self.is_tree() {
                b"0".as_bstr()
            } else {
                b"".as_bstr()
            },
            &self.text[0..null_index],
            if self.is_tree() {
                b"tree".as_bstr()
            } else {
                b"blob".as_bstr()
            },
            self.hash,
            &self.text[null_index + 1..]
        ))
    }
}

#[derive(Debug)]
pub struct TreeLineIndex {
    hash: TreeHash,
    text: shared::RefSlice<u8>,
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

impl Display for Tree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for line in self.lines.iter() {
            let text = line.text.get(&self._bytes).as_bstr();
            writeln!(f, "{} {}", line.hash, text)?;
        }
        Ok(())
    }
}
