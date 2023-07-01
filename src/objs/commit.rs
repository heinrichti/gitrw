use std::{fmt::Display, marker::PhantomData, slice};

use bstr::{ByteSlice, Lines};

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

        if skip_first_null {
            let mut null_idx = 0;
            for i in 0..bytes.len() {
                if bytes[i] == b'\0' {
                    null_idx = i;
                    break;
                }
            }
            line_reader = bytes[null_idx+1..].lines();
        } else {
            line_reader = bytes.lines();
        }

        let tree_line = line_reader.next().map(|line| 
            (unsafe { line.as_ptr().add(5) }, line.len() - 5)).unwrap();

        let mut parents = Vec::with_capacity(1);
        let mut line = line_reader.next().unwrap();
        while line.starts_with(b"parent ") {
            parents.push((unsafe { line.as_ptr().add(7) }, line.len() - 7));
            line = line_reader.next().unwrap();
        }
        let author_line = (unsafe { line.as_ptr().add(7) }, line.len() - 7);
        let committer_line = line_reader
            .next()
            .map(|line| (unsafe { line.as_ptr().add(10) }, line.len() - 10))
            .unwrap();

        dbg!(&object_hash);
        let remainder = unsafe { committer_line.0.add(committer_line.1 + 1) };
        let tmp = unsafe { bytes.as_ptr().add(bytes.len()).offset_from(remainder) };
        dbg!(tmp);
        let remainder_len: usize = unsafe { bytes.as_ptr().add(bytes.len()).offset_from(remainder) }
            .try_into().unwrap();

        Commit {
            object_hash,
            _bytes: bytes,
            tree_line,
            parents,
            author_line,
            committer_line,
            remainder: (remainder, remainder_len),
            _phantom: PhantomData,
        }
    }

    pub fn tree(&self) -> ObjectHash {
        unsafe { std::slice::from_raw_parts(self.tree_line.0, self.tree_line.1) }
            .as_bstr().try_into().unwrap()
    }

    pub fn parents(&self) -> Vec<ObjectHash> {
        let mut result = Vec::with_capacity(self.parents.len());
        for parent in self.parents.iter() {
            let a = unsafe { slice::from_raw_parts(parent.0, parent.1) };
            result.push(a.as_bstr().try_into().unwrap());
        }

        result
    }

    pub fn author(&self) -> &'a bstr::BStr {
        Commit::contributor(unsafe {
            std::slice::from_raw_parts(self.author_line.0, self.author_line.1)
        })
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

    pub fn committer(&self) -> &'a bstr::BStr {
        Commit::contributor(unsafe {
            std::slice::from_raw_parts(self.committer_line.0, self.committer_line.1)
        })
    }
}
