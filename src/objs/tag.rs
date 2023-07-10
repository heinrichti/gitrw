use std::ops::Deref;

use bstr::{ByteSlice, ByteVec};

use crate::{objs::TagTargetType, shared::RefSlice};

use super::{ObjectHash, Tag};

impl<'a> Tag<'a> {
    pub fn create(_object_hash: ObjectHash, bytes: Box<[u8]>, skip_first_null: bool) -> Tag<'a> {
        let mut tag = Tag {
            _bytes: bytes,
            object: RefSlice::Owned(vec![]),
            obj_type: RefSlice::Owned(vec![]),
            remainder: RefSlice::Owned(vec![]),
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

        let line_start: usize = unsafe { line.as_ptr().offset_from(bytes.as_ptr()) }
            .try_into()
            .unwrap();
        let remainder_start = line_start + line.len() + 1;
        let remainder = RefSlice::from_slice(&bytes[remainder_start..]);

        tag.object = object;
        tag.obj_type = obj_type;
        tag.remainder = remainder;

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
            format_args!(
                "unknown target type: {} for object {}",
                target.as_bstr(),
                self.object()
            )
        );
    }

    pub fn to_bytes(&self) -> Box<[u8]> {
        let byte_size: usize = b"object \n".len()
            + self.object.len()
            + b"type \n".len()
            + self.obj_type.len()
            + self.remainder.len();

        let mut result: Vec<u8> = Vec::with_capacity(byte_size);

        result.push_str(b"object ");
        result.push_str(self.object.deref());
        result.push_str(b"\n");

        result.push_str(b"type ");
        result.push_str(self.obj_type.deref());
        result.push_str(b"\n");

        result.push_str(self.remainder.deref());

        result.into_boxed_slice()
    }
}
