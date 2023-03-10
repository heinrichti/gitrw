use std::{slice, marker::PhantomData};

use bstr::ByteSlice;

use crate::object_hash::ObjectHash;

#[derive(Debug)]
pub struct Commit<'a> {
    pub object_hash: ObjectHash,
    _bytes: Box<[u8]>,
    // tree: Range<usize>,
    // parents: Box<[Range<usize>]>,
    // author_line: Range<usize>,
    // committer_line: Range<usize>,
    // message: Range<usize>,
    parents: Vec<(*const u8, usize)>,
    author_line: (*const u8, usize),
    committer_line: (*const u8, usize),
    _phantom: PhantomData<&'a [u8]>
}

impl<'a> Commit<'a> {
    pub fn create(object_hash: ObjectHash, bytes: Box<[u8]>, skip_first_null: bool) -> Commit<'a> {
        let mut line_reader = bytes.lines();
        if skip_first_null {
            line_reader.next();
        };
        line_reader.next();
        // let tree = line_reader.next().map(|x| x.start + 5..x.end).unwrap();
        let mut parents = Vec::with_capacity(1);
        let mut line = line_reader.next().unwrap();
        while line.starts_with(b"parent ") {
            parents.push((unsafe { line.as_ptr().add(7) }, line.len() - 7));
            line = line_reader.next().unwrap();
        }
        let author_line = (unsafe { line.as_ptr().add(7) }, line.len() - 7);
        let committer_line = line_reader
            .next()
            .map(|c| (unsafe { c.as_ptr().add(10) }, c.len() - 10))
            .unwrap();

        // while {
        //     line = line_reader.next().unwrap();
        //     line.count() != 0
        // } {}

        // let message = if let Some(line) = line_reader.next() {
        //     line.start..bytes.len()
        // } else {
        //     0..0
        // };

        Commit {
            object_hash,
            _bytes: bytes,
            // tree,
            parents,
            author_line,
            committer_line,
            _phantom: PhantomData
            // message,
        }
    }

    // pub fn tree(&self) -> &bstr::BStr {
    //     &self.bytes[self.tree.clone()].as_bstr()
    // }

    pub fn parents(&self) -> Vec<ObjectHash> {
        let mut result = Vec::with_capacity(self.parents.len());
        for parent in self.parents.iter() {
            let a = unsafe { slice::from_raw_parts(parent.0, parent.1) };
            result.push(a.as_bstr().into());
        }

        result
    }

    // pub fn message(&self) -> &bstr::BStr {
    //     &self.bytes[self.message.clone()].as_bstr()
    // }

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
