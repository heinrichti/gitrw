use std::{fmt::Display, ops::Deref, vec};

use bstr::{ByteSlice, ByteVec, Lines};

use crate::shared::RefSlice;

use super::{Commit, ObjectHash};

impl<'a> Display for Commit<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.object_hash))?;
        Ok(())
    }
}

impl<'a> Commit<'a> {
    pub fn create(object_hash: ObjectHash, bytes: Box<[u8]>, skip_first_null: bool) -> Commit<'a> {
        let mut line_reader: Lines<'_>;
        let mut commit = Commit {
            object_hash,
            _bytes: bytes,
            tree_line: RefSlice::Owned(vec![]),
            parents: vec![],
            author_line: RefSlice::Owned(vec![]),
            committer_line: RefSlice::Owned(vec![]),
            _remainder: RefSlice::Owned(vec![]),
        };

        let bytes = &commit._bytes;

        if skip_first_null {
            let mut null_idx = 0;
            for i in 0..bytes.len() {
                if bytes[i] == b'\0' {
                    null_idx = i;
                    break;
                }
            }
            line_reader = bytes[null_idx + 1..].lines();
        } else {
            line_reader = bytes.lines();
        }

        let tree_line = line_reader
            .next()
            .map(|line| RefSlice::<'a>::from_slice(&line[5..]))
            .unwrap();

        let mut parents = Vec::with_capacity(1);
        let mut line = line_reader.next().unwrap();
        while line.starts_with(b"parent ") {
            parents.push(RefSlice::from_slice(&line[7..]));
            line = line_reader.next().unwrap();
        }

        let author_line = RefSlice::from_slice(&line[7..]);

        let committer_line = line_reader
            .next()
            .map(|line| RefSlice::from_slice(&line[10..]))
            .unwrap();

        let committer_line_start: usize =
            unsafe { committer_line.as_ptr().offset_from(bytes.as_ptr()) }
                .try_into()
                .unwrap();
        let remainder_start: usize = committer_line_start + committer_line.len() + 1;
        let remainder = RefSlice::from_slice(&bytes[remainder_start..]);

        commit.tree_line = tree_line;
        commit.parents = parents;
        commit.author_line = author_line;
        commit.committer_line = committer_line;
        commit._remainder = remainder;

        commit
    }

    pub fn tree(&self) -> ObjectHash {
        self.tree_line.as_bstr().try_into().unwrap()
    }

    pub fn set_tree(&mut self, value: ObjectHash) {
        self.tree_line = RefSlice::from(value.to_string().as_bytes().to_vec());
    }

    pub fn parents(&self) -> Vec<ObjectHash> {
        let mut result = Vec::with_capacity(self.parents.len());
        for parent in self.parents.iter() {
            result.push(parent.as_bstr().try_into().unwrap());
        }

        result
    }

    pub fn author(&'a self) -> &'a bstr::BStr {
        Commit::contributor(&self.author_line)
    }

    pub fn set_author(&mut self, author: Vec<u8>) {
        self.author_line = RefSlice::from(author);
    }

    fn contributor(line: &'a [u8]) -> &'a bstr::BStr {
        let mut spaces = 0;
        for (i, b) in line.iter().rev().enumerate() {
            let index_from_back = line.len() - i - 1;
            if *b == b' ' {
                spaces += 1;
            }

            if spaces == 2 {
                return line[0..index_from_back].as_bstr();
            }
        }

        return (b"").as_bstr();
    }

    pub fn committer(&'a self) -> &'a bstr::BStr {
        Commit::contributor(&self.committer_line)
    }

    pub fn set_committer(&mut self, committer: Vec<u8>) {
        self.committer_line = RefSlice::from(committer);
    }

    pub fn to_bytes(&self) -> Box<[u8]> {
        let mut result: Vec<u8> = Vec::with_capacity(
            b"tree \n".len()
                + self.tree_line.len()
                + self
                    .parents
                    .iter()
                    .map(|parent| b"parent \n".len() + parent.len())
                    .sum::<usize>()
                + b"author \n".len()
                + self.committer_line.len()
                + b"committer \n".len()
                + self.author_line.len()
                + self._remainder.len(),
        );

        dbg!(result.capacity());

        result.push_str(b"tree ");
        result.push_str(self.tree_line.deref());
        result.push_str(b"\n");

        for parent in &self.parents {
            result.push_str(b"parent ");
            result.push_str(parent.deref());
            result.push_str(b"\n");
        }

        result.push_str(b"author ");
        result.push_str(self.author_line.deref());
        result.push_str(b"\n");

        result.push_str(b"committer ");
        result.push_str(self.committer_line.deref());
        result.push_str(b"\n");

        result.push_str(self._remainder.deref());

        result.into_boxed_slice()
    }
}
