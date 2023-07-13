use bstr::{ByteSlice, ByteVec};

use crate::{objs::TagTargetType, shared::RefSlice};

use super::{ObjectHash, Tag};

impl Tag {
    pub fn create(_object_hash: ObjectHash, bytes: Box<[u8]>, skip_first_null: bool) -> Tag {
        let mut tag = Tag {
            bytes,
            object: RefSlice::Owned(vec![]),
            obj_type: RefSlice::Owned(vec![]),
            remainder: RefSlice::Owned(vec![]),
        };

        let bytes = &tag.bytes;

        let mut line_reader = bytes.lines();
        if skip_first_null {
            line_reader.next();
        };

        let line = line_reader.next().unwrap();
        let object = RefSlice::from_slice(bytes, line, 7);

        let line = line_reader.next().unwrap();
        let obj_type = RefSlice::from_slice(bytes, line, 5);

        let line_start: usize = unsafe { line.as_ptr().offset_from(bytes.as_ptr()) }
            .try_into()
            .unwrap();
        let remainder_start = line_start + line.len() + 1;
        let remainder = RefSlice::new(remainder_start, bytes.len() - remainder_start);

        tag.object = object;
        tag.obj_type = obj_type;
        tag.remainder = remainder;

        tag
    }

    pub fn object(&self) -> ObjectHash {
        self.object.get(&self.bytes).as_bstr().try_into().unwrap()
    }

    pub fn target_type(&self) -> TagTargetType {
        let target = self.obj_type.get(&self.bytes);

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
            + self.object.get(&self.bytes).len()
            + b"type \n".len()
            + self.obj_type.get(&self.bytes).len()
            + self.remainder.get(&self.bytes).len();

        let mut result: Vec<u8> = Vec::with_capacity(byte_size);

        result.push_str(b"object ");
        result.push_str(self.object.get(&self.bytes));
        result.push_str(b"\n");

        result.push_str(b"type ");
        result.push_str(self.obj_type.get(&self.bytes));
        result.push_str(b"\n");

        result.push_str(self.remainder.get(&self.bytes));

        result.into_boxed_slice()
    }
}
