use std::{fmt::Display, hash::Hasher, ops::Deref, vec};

use bstr::{BStr, ByteSlice, ByteVec, Lines};
use rs_sha1::{HasherContext, Sha1Hasher};

use crate::{shared::RefSlice, WriteObject};

use super::{Commit, CommitHash, ObjectHash, TreeHash};

impl Display for CommitHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.0))
    }
}

impl From<ObjectHash> for CommitHash {
    fn from(value: ObjectHash) -> Self {
        CommitHash(value)
    }
}

impl TryFrom<&BStr> for CommitHash {
    type Error = &'static str;

    fn try_from(value: &BStr) -> Result<Self, Self::Error> {
        ObjectHash::try_from_bstr(value)
    }
}

impl<'a> Display for Commit<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.hash()))?;
        Ok(())
    }
}

impl<'a> Commit<'a> {
    pub fn create(hash: Option<CommitHash>, bytes: Box<[u8]>, skip_first_null: bool) -> Commit<'a> {
        let mut line_reader: Lines<'_>;
        let mut commit = Commit {
            hash: Some(hash.unwrap_or_else(|| CommitHash(Self::calculate_hash(&bytes)))),
            _bytes: bytes,
            tree_line: RefSlice::Owned(vec![]),
            parents: vec![],
            author: RefSlice::Owned(vec![]),
            author_time: RefSlice::Owned(vec![]),
            committer: RefSlice::Owned(vec![]),
            committer_time: RefSlice::Owned(vec![]),
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

        let author_line = &line[7..];
        let author_time = Self::time_index(author_line);
        let author = RefSlice::from_slice(&author_line[0..author_time]);
        let author_time = RefSlice::from_slice(&author_line[author_time + 1..]);

        let committer_line = line_reader.next().map(|line| &line[10..]).unwrap();
        let committer_time_index = Self::time_index(committer_line);
        let committer = RefSlice::from_slice(&committer_line[0..committer_time_index]);
        let committer_time = RefSlice::from_slice(&committer_line[committer_time_index + 1..]);

        let committer_line_start: usize =
            unsafe { committer_line.as_ptr().offset_from(bytes.as_ptr()) }
                .try_into()
                .unwrap();
        let remainder_start: usize = committer_line_start + committer_line.len() + 1;
        let remainder = RefSlice::from_slice(&bytes[remainder_start..]);

        commit.tree_line = tree_line;
        commit.parents = parents;
        commit.author = author;
        commit.author_time = author_time;
        commit.committer = committer;
        commit.committer_time = committer_time;
        commit._remainder = remainder;

        commit
    }

    pub fn calculate_hash(data: &[u8]) -> ObjectHash {
        let mut hasher = Sha1Hasher::default();
        hasher.write(b"commit ");
        hasher.write(data.len().to_string().as_bytes());
        hasher.write(b"\0");
        hasher.write(data);
        let bytes = HasherContext::finish(&mut hasher);
        let bytes: [u8; 20] = bytes.into();
        ObjectHash::from(bytes)
    }

    pub fn has_changes(&self) -> bool {
        self.hash.is_none()
    }

    pub fn hash(&self) -> &CommitHash {
        self.hash.as_ref().unwrap()
    }

    pub fn tree(&self) -> TreeHash {
        self.tree_line.as_bstr().try_into().unwrap()
    }

    pub fn set_tree(&mut self, value: TreeHash) {
        self.tree_line = RefSlice::from(value.to_string().as_bytes().to_vec());
        self.hash = None;
    }

    pub fn parents(&self) -> Vec<CommitHash> {
        let mut result = Vec::with_capacity(self.parents.len());
        for parent in self.parents.iter() {
            result.push(parent.as_bstr().try_into().unwrap());
        }

        result
    }

    pub fn set_parent(&mut self, index: usize, value: CommitHash) {
        self.parents[index] = RefSlice::Owned(value.0.to_string().bytes().collect());
        self.hash = None;
    }

    pub fn author(&'a self) -> &'a bstr::BStr {
        self.author.as_bstr()
    }

    pub fn author_bytes(&'a self) -> &'a [u8] {
        &self.author
    }

    pub fn set_author(&mut self, author: Vec<u8>) {
        self.author = RefSlice::from(author);
        self.hash = None;
    }

    fn time_index(line: &[u8]) -> usize {
        let mut spaces = 0;
        for (i, b) in line.iter().rev().enumerate() {
            let index_from_back = line.len() - i - 1;
            if *b == b' ' {
                spaces += 1;
            }

            if spaces == 2 {
                return index_from_back;
            }
        }

        line.len()
    }

    pub fn committer(&'a self) -> &'a bstr::BStr {
        self.committer.as_bstr()
    }

    pub fn committer_bytes(&'a self) -> &'a [u8] {
        &self.committer
    }

    pub fn set_committer(&mut self, committer: Vec<u8>) {
        self.committer = RefSlice::from(committer);
        self.hash = None;
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
                + b"author  \n".len()
                + self.committer.len()
                + self.committer_time.len()
                + b"committer  \n".len()
                + self.author.len()
                + self.author_time.len()
                + self._remainder.len(),
        );

        result.push_str(b"tree ");
        result.push_str(self.tree_line.deref());
        result.push_str(b"\n");

        for parent in &self.parents {
            result.push_str(b"parent ");
            result.push_str(parent.deref());
            result.push_str(b"\n");
        }

        result.push_str(b"author ");
        result.push_str(self.author.deref());
        result.push_str(b" ");
        result.push_str(self.author_time.deref());
        result.push_str(b"\n");

        result.push_str(b"committer ");
        result.push_str(self.committer.deref());
        result.push_str(b" ");
        result.push_str(self.committer_time.deref());
        result.push_str(b"\n");

        result.push_str(self._remainder.deref());

        result.into_boxed_slice()
    }
}

impl<'a> WriteObject for Commit<'a> {
    fn to_bytes(&self) -> &[u8] {
        &self._bytes
    }

    fn hash(&self) -> &ObjectHash {
        &self.hash().0
    }

    fn prefix(&self) -> &str {
        "commit"
    }
}
