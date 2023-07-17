use std::fmt::Display;

use bstr::{BStr, ByteSlice, ByteVec, Lines};

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

impl Display for Commit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.hash()))?;
        Ok(())
    }
}

impl Commit {
    pub fn create(hash: Option<CommitHash>, bytes: Box<[u8]>, skip_first_null: bool) -> Commit {
        let mut line_reader: Lines<'_>;

        let mut null_idx = 0;
        if skip_first_null {
            for i in 0..bytes.len() {
                if bytes[i] == b'\0' {
                    null_idx = i;
                    break;
                }
            }
            null_idx += 1;
            line_reader = bytes[null_idx..].lines();
        } else {
            line_reader = bytes.lines();
        }

        let mut line = line_reader.next().unwrap();
        let tree_line = RefSlice::from_slice(&bytes, line, 5);

        let mut parents = Vec::with_capacity(1);
        line = line_reader.next().unwrap();
        while line.starts_with(b"parent ") {
            parents.push(RefSlice::from_slice(&bytes, line, 7));
            line = line_reader.next().unwrap();
        }

        let author_line = &line[7..];
        let author_time_index = Self::time_index(author_line);
        let author = RefSlice::from_slice(&bytes, &author_line[0..author_time_index], 0);
        let author_time = RefSlice::from_slice(&bytes, author_line, author_time_index + 1);

        let committer_line = line_reader.next().map(|line| &line[10..]).unwrap();
        let committer_time_index = Self::time_index(committer_line);
        let committer = RefSlice::from_slice(&bytes, &committer_line[0..committer_time_index], 0);
        let committer_time = RefSlice::from_slice(&bytes, committer_line, committer_time_index + 1);

        let committer_line_start: usize =
            unsafe { committer_line.as_ptr().offset_from(bytes.as_ptr()) }
                .try_into()
                .unwrap();
        let remainder_start: usize = committer_line_start + committer_line.len() + 1;
        let remainder = RefSlice::new(remainder_start, bytes.len() - remainder_start);

        Commit {
            hash: hash.or_else(|| Some(CommitHash(crate::calculate_hash(&bytes, b"commit")))),
            bytes,
            bytes_start: null_idx,
            tree_line,
            parents,
            author,
            author_time,
            committer,
            committer_time,
            remainder,
        }
    }

    pub fn has_changes(&self) -> bool {
        self.hash.is_none()
    }

    pub fn hash(&self) -> &CommitHash {
        self.hash.as_ref().unwrap()
    }

    pub fn tree(&self) -> TreeHash {
        self.tree_line
            .get(&self.bytes)
            .as_bstr()
            .try_into()
            .unwrap()
    }

    pub fn set_tree(&mut self, value: TreeHash) {
        self.tree_line = RefSlice::from(value.to_string().as_bytes().to_vec());
        self.hash = None;
    }

    pub fn parents(&self) -> Vec<CommitHash> {
        let mut result = Vec::with_capacity(self.parents.len());
        for parent in self.parents.iter() {
            result.push(parent.get(&self.bytes).as_bstr().try_into().unwrap());
        }

        result
    }

    pub fn set_parent(&mut self, index: usize, value: CommitHash) {
        self.parents[index] = RefSlice::Owned(value.0.to_string().bytes().collect());
        self.hash = None;
    }

    pub fn author(&self) -> &bstr::BStr {
        self.author.get(&self.bytes).as_bstr()
    }

    pub fn author_bytes(&self) -> &[u8] {
        self.author.get(&self.bytes)
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

    pub fn committer(&self) -> &bstr::BStr {
        self.committer.get(&self.bytes).as_bstr()
    }

    pub fn committer_bytes(&self) -> &[u8] {
        self.committer.get(&self.bytes)
    }

    pub fn set_committer(&mut self, committer: Vec<u8>) {
        self.committer = RefSlice::from(committer);
        self.hash = None;
    }

    pub fn to_bytes(&self) -> Box<[u8]> {
        let mut result: Vec<u8> = Vec::with_capacity(
            b"tree \n".len()
                + self.tree_line.get(&self.bytes).len()
                + self
                    .parents
                    .iter()
                    .map(|parent| b"parent \n".len() + parent.get(&self.bytes).len())
                    .sum::<usize>()
                + b"author  \n".len()
                + self.committer.get(&self.bytes).len()
                + self.committer_time.get(&self.bytes).len()
                + b"committer  \n".len()
                + self.author.get(&self.bytes).len()
                + self.author_time.get(&self.bytes).len()
                + self.remainder.get(&self.bytes).len(),
        );

        result.push_str(b"tree ");
        result.push_str(self.tree_line.get(&self.bytes));
        result.push_str(b"\n");

        for parent in &self.parents {
            result.push_str(b"parent ");
            result.push_str(parent.get(&self.bytes));
            result.push_str(b"\n");
        }

        result.push_str(b"author ");
        result.push_str(self.author.get(&self.bytes));
        result.push_str(b" ");
        result.push_str(self.author_time.get(&self.bytes));
        result.push_str(b"\n");

        result.push_str(b"committer ");
        result.push_str(self.committer.get(&self.bytes));
        result.push_str(b" ");
        result.push_str(self.committer_time.get(&self.bytes));
        result.push_str(b"\n");

        result.push_str(self.remainder.get(&self.bytes));

        debug_assert_eq!(result.capacity(), result.len());

        result.into_boxed_slice()
    }
}

impl WriteObject for Commit {
    fn to_bytes(&self) -> &[u8] {
        &self.bytes[self.bytes_start..]
    }

    fn hash(&self) -> &ObjectHash {
        &self.hash().0
    }

    fn prefix(&self) -> &str {
        "commit"
    }
}
