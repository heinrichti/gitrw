use std::fmt::Display;

use bstr::{BStr, BString, ByteSlice, ByteVec};

use crate::shared::SliceIndexes;

use super::{CommitEditable, CommitBase, CommitHash, ObjectHash, TreeHash, WriteBytes};
use memchr::memchr;

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

impl Display for CommitBase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.hash))?;
        Ok(())
    }
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

impl CommitBase {
    pub fn create(hash: CommitHash, bytes: Box<[u8]>, skip_first_null: bool) -> Self {
        let mut bytes_start = 0;
        let mut line_reader = if skip_first_null {
            bytes_start = memchr(b'\0', &bytes).unwrap();
            bytes_start += 1;
            bytes[bytes_start..].lines()
        } else {
            bytes.lines()
        };

        let mut line = line_reader.next().unwrap();
        let tree_line = SliceIndexes::from_slice(&bytes, line, 5);

        let mut parents = Vec::with_capacity(1);
        line = line_reader.next().unwrap();
        while line.starts_with(b"parent ") {
            parents.push(SliceIndexes::from_slice(&bytes, line, 7));
            line = line_reader.next().unwrap();
        }

        let author_line = &line[7..];
        let author_time_index = time_index(author_line);
        let author = SliceIndexes::from_slice(&bytes, &author_line[0..author_time_index], 0);
        let author_time = SliceIndexes::from_slice(&bytes, author_line, author_time_index + 1);

        let committer_line = line_reader.next().map(|line| &line[10..]).unwrap();
        let committer_time_index = time_index(committer_line);
        let committer = SliceIndexes::from_slice(&bytes, &committer_line[0..committer_time_index], 0);
        let committer_time = SliceIndexes::from_slice(&bytes, committer_line, committer_time_index + 1);

        let committer_line_start: usize =
            unsafe { committer_line.as_ptr().offset_from(bytes.as_ptr()) }
                .try_into()
                .unwrap();
        let remainder_start: usize = committer_line_start + committer_line.len() + 1;
        let remainder = SliceIndexes::new(remainder_start, bytes.len() - remainder_start);

        Self {
            hash,
            bytes: WriteBytes {
                bytes,
                start: bytes_start,
            },
            tree_line,
            parents,
            author,
            author_time,
            committer,
            committer_time,
            remainder,
        }
    }

    pub(crate) fn get_str(&self, f: impl Fn(&CommitBase) -> &SliceIndexes) -> &BStr {
        f(&self).get(&self.bytes.bytes).as_bstr()
    }

    pub fn parents(&self) -> Vec<CommitHash> {
        self.parents.iter().enumerate().map(|(i, _)| {
            self.get_str(|c| &c.parents[i]).try_into().unwrap()
        }).collect()
    }

    pub fn author(&self) -> &bstr::BStr {
        self.get_str(|c| &c.author)
    }
    
    pub fn committer(&self) -> &bstr::BStr {
        self.get_str(|c| &c.committer)
    }

    pub fn tree(&self) -> TreeHash {
        self.get_str(|c| &c.tree_line).try_into().unwrap()
    }
}

impl CommitEditable {
    pub fn create(base: CommitBase) -> Self {
        let parents = vec![None; base.parents.len()];
        CommitEditable {
            base,
            tree: None,
            author: None,
            committer: None,
            parents,
        }
    }

    pub fn has_changes(&self) -> bool {
        self.tree.is_some() 
            || self.author.is_some() 
            || self.committer.is_some() 
            || self.parents.iter().any(|p| p.is_some())
    }

    pub fn parents(&self) -> Vec<CommitHash> {
        self.parents.iter().enumerate().map(|(i, p)| {
            if let Some(p) = p {
                p.clone()
            } else {
                self.base.parents[i].get(&self.base.bytes.bytes).as_bstr().try_into().unwrap()
            }
        }).collect()
    }

    pub fn base_hash(&self) -> &CommitHash {
        &self.base.hash
    }

    pub fn tree(&self) -> TreeHash {
        if let Some(t) = &self.tree {
            t.clone()
        } else {
            self.base.get_str(|c| &c.tree_line).try_into().unwrap()
        }
    }

    pub fn set_tree(&mut self, value: TreeHash) {
        self.tree = Some(value);
    }

    pub fn set_parent(&mut self, index: usize, value: CommitHash) {
        self.parents[index] = Some(value);
    }

    // pub fn author(&self) -> &bstr::BStr {
    //     self.author.get(&self.bytes).as_bstr()
    // }

    pub fn author_bytes(&self) -> &[u8] {
        if let Some(author) = &self.author {
            author
        } else {
            self.base.get_str(|c| &c.author).as_bytes()
        }
    }

    pub fn set_author(&mut self, author: Vec<u8>) {
        self.author = Some(author);
    }

    // pub fn committer(&self) -> &bstr::BStr {
    //     self.committer.get(&self.bytes).as_bstr()
    // }

    pub fn committer_bytes(&self) -> &[u8] {
        if let Some(committer) = &self.committer {
            committer
        } else {
            self.base.get_str(|c| &c.committer).as_bytes()
        }
        // self.committer.get(&self.bytes)
    }

    pub fn set_committer(&mut self, committer: Vec<u8>) {
        self.committer = Some(committer);
    }

    // pub fn tree_str(&self) -> &BStr {
    //     if let Some(t) = self.tree {
    //         format!("{}", t).as_bytes().as_bstr()
    //     } else {
    //         self.get_base_str(|commit_base| &commit_base.tree_line).as_bstr()
    //     }
    // }

    fn get_str(&self, 
        self_getter: impl Fn(&Self) -> &Option<Vec<u8>>,
        base_getter: impl Fn(&CommitBase) -> &SliceIndexes) -> &BStr {
            if let Some(v) = self_getter(self) {
                v.as_bstr()
            } else {
                self.base.get_str(base_getter)
            }
    }

    pub fn to_bytes(self) -> WriteBytes {
        // let bytes = self.base.bytes();

        let has_changes = self.has_changes();
        if !has_changes {
            return self.base.bytes;
        }

        let tree: BString = //self.get_str(|c| &c.tree, |c| &c.tree_line);
            if let Some(tree) = &self.tree {
                tree.to_string().as_bytes().as_bstr().to_owned()
            } else {
                self.base.get_str(|c| &c.tree_line).to_owned()
            };

        let parents: Vec<_> = self.parents().iter().map(|p| format!("{}", p)).collect();

        let author = self.get_str(|c| &c.author, |c| &c.author);
        let author_time = self.base.get_str(|c| &c.author_time);
        let committer = self.get_str(|c| &c.committer, |c| &c.committer);
        let committer_time = self.base.get_str(|c| &c.committer_time);
        let remainder = self.base.get_str(|c| &c.remainder);
        
        let mut result: Vec<u8> = Vec::with_capacity(
            b"tree \n".len() + tree.len()
                + parents
                    .iter()
                    .map(|parent| b"parent \n".len() + parent.len())
                    .sum::<usize>()
                + b"author  \n".len() + author.len()
                + committer_time.len()
                + b"committer  \n".len() + committer.len()
                + author_time.len()
                + remainder.len(),
        );

        result.push_str(b"tree ");
        result.push_str(tree);
        result.push_str(b"\n");

        for parent in parents {
            result.push_str(b"parent ");
            result.push_str(parent);
            result.push_str(b"\n");
        }

        result.push_str(b"author ");
        result.push_str(author);
        result.push_str(b" ");
        result.push_str(author_time);
        result.push_str(b"\n");

        result.push_str(b"committer ");
        result.push_str(committer);
        result.push_str(b" ");
        result.push_str(committer_time);
        result.push_str(b"\n");

        result.push_str(remainder);

        debug_assert_eq!(result.capacity(), result.len());

        WriteBytes {
            bytes: result.into_boxed_slice(),
            start: 0,
        }
    }
}
