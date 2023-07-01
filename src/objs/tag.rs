use std::ops::Deref;

use bstr::ByteSlice;

use crate::{objs::TagTargetType, shared::RefSlice};

use super::{ObjectHash, Tag};

impl<'a> Tag<'a> {
    pub fn create(_object_hash: ObjectHash, bytes: Box<[u8]>, skip_first_null: bool) -> Tag<'a> {
        let mut tag = Tag {
            _bytes: bytes,
            object: RefSlice::Owned(vec![]),
            obj_type: RefSlice::Owned(vec![]),
        };

        let bytes = &tag._bytes;

        let mut line_reader = bytes.lines();
        if skip_first_null {
            line_reader.next();
        };

        let line = line_reader.next().unwrap();
        let object = RefSlice::from_slice(&line[7..]);

        let line = line_reader.next().unwrap();
        let obj_type = RefSlice::from_slice(&line[5..]);

        // let line = line_reader.next().unwrap();

        // for line in line_reader {
        //     // if bytes[line.clone()].starts_with(b"object ") {
        //     //     object = line.start + b"object ".len()..line.end;
        //     // } else if bytes[line.clone()].starts_with(b"type ") {
        //         // obj_type = line.start + b"type ".len()..line.end;
        //     // } else
        //     if bytes[line.clone()].starts_with(b"tag ") {
        //         tag = line.start + b"tag ".len()..line.end;
        //     } else if bytes[line.clone()].starts_with(b"tagger ") {
        //         tagger = line.start + b"tagger ".len()..line.end;
        //     } else {
        //         message = line;
        //         break;
        //     }
        // }

        tag.object = object;
        tag.obj_type = obj_type;
        tag
    }

    pub fn object(&self) -> ObjectHash {
        self.object.as_bstr().try_into().unwrap()
    }

    pub fn target_type(&self) -> TagTargetType {
        let target = self.obj_type.deref();

        if target == b"commit" {
            return TagTargetType::Commit;
        } else if target == b"tree" {
            return TagTargetType::Tree;
        }

        panic!(
            "{}",
            format_args!("unknown target type: {}", target.as_bstr())
        );
    }
}
