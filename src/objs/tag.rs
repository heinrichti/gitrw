use bstr::{BStr, ByteSlice, ByteVec, Lines};

use crate::{objs::TagTargetType, shared::RefSlice};

use super::{ObjectHash, Tag};

impl Tag {
    pub fn create(hash: Option<ObjectHash>, bytes: Box<[u8]>, skip_first_null: bool) -> Tag {
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

        let line = line_reader.next().unwrap();
        let object = RefSlice::from_slice(&bytes, line, 7);

        let line = line_reader.next().unwrap();
        let obj_type = RefSlice::from_slice(&bytes, line, 5);

        let line = line_reader.next().unwrap();
        let tag_name = RefSlice::from_slice(&bytes, line, 4);

        let line_start: usize = unsafe { line.as_ptr().offset_from(bytes.as_ptr()) }
            .try_into()
            .unwrap();
        let remainder_start = line_start + line.len() + 1;
        let remainder = RefSlice::new(remainder_start, bytes.len() - remainder_start);

        Tag {
            hash: hash.or_else(|| Some(crate::calculate_hash(&bytes, b"tag"))),
            bytes,
            bytes_start: null_idx,
            object,
            obj_type,
            tag_name,
            remainder,
        }
    }

    pub fn hash(&self) -> &ObjectHash {
        self.hash.as_ref().unwrap()
    }

    pub fn object(&self) -> ObjectHash {
        self.object.get(&self.bytes).as_bstr().try_into().unwrap()
    }

    pub fn set_object(&mut self, object: ObjectHash) {
        self.hash = None;
        self.object = RefSlice::Owned(object.to_string().bytes().collect());
    }

    pub fn name(&self) -> &BStr {
        self.tag_name.get(&self.bytes).as_bstr()
    }

    pub fn target_type(&self) -> TagTargetType {
        let target = self.obj_type.get(&self.bytes);

        if target == b"tag" {
            return TagTargetType::Tag;
        }
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

    pub(crate) fn bytes(&self) -> &[u8] {
        &self.bytes[self.bytes_start..]
    }

    pub fn to_bytes(&self) -> Box<[u8]> {
        let byte_size: usize = b"object \n".len()
            + self.object.get(&self.bytes).len()
            + b"type \n".len()
            + self.obj_type.get(&self.bytes).len()
            + b"tag \n".len()
            + self.tag_name.get(&self.bytes).len()
            + self.remainder.get(&self.bytes).len();

        let mut result: Vec<u8> = Vec::with_capacity(byte_size);

        result.push_str(b"object ");
        result.push_str(self.object.get(&self.bytes));
        result.push_str(b"\n");

        result.push_str(b"type ");
        result.push_str(self.obj_type.get(&self.bytes));
        result.push_str(b"\n");

        result.push_str(b"tag ");
        result.push_str(self.tag_name.get(&self.bytes));
        result.push_str(b"\n");

        result.push_str(self.remainder.get(&self.bytes));

        result.into_boxed_slice()
    }
}
