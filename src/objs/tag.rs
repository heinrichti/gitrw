use std::slice;

use bstr::ByteSlice;

use crate::{git_objects::TagTargetType, object_hash::ObjectHash};

#[derive(Debug)]
pub struct Tag {
    // object_hash: ObjectHash,
    _bytes: Box<[u8]>,
    // object: Range<usize>,
    // obj_type: Range<usize>,
    object: (*const u8, usize),
    obj_type: (*const u8, usize), // tag: Range<usize>,
                                  // tagger: Range<usize>,
                                  // message: Range<usize>
}

impl Tag {
    pub fn create(_object_hash: ObjectHash, bytes: Box<[u8]>, skip_first_null: bool) -> Tag {
        let mut line_reader = bytes.lines();
        if skip_first_null {
            line_reader.next();
        };

        let line = line_reader.next().unwrap();
        let object = (unsafe { line.as_ptr().add(7) }, line.len() - 7);

        let line = line_reader.next().unwrap();
        let obj_type = (unsafe { line.as_ptr().add(5) }, line.len() - 5);

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

        // Tag { object_hash, bytes, object, obj_type, tag, tagger, message }
        Tag {
            _bytes: bytes,
            object,
            obj_type,
        }
    }

    pub fn object(&self) -> ObjectHash {
        unsafe { slice::from_raw_parts(self.object.0, self.object.1) }
            .as_bstr()
            .into()
    }

    pub fn target_type(&self) -> TagTargetType {
        let target = unsafe { slice::from_raw_parts(self.obj_type.0, self.obj_type.1) };

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
