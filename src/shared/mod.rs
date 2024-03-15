pub(crate) mod object_hash;

#[derive(Eq, PartialEq, Clone, Hash)]
pub struct ObjectHash {
    pub(crate) bytes: [u8; 20],
}

#[derive(Debug)]
pub enum RefSlice<T> {
    Referenced(SliceIndexes),
    Owned(Vec<T>),
}

#[derive(Debug)]
pub struct SliceIndexes {
    position: usize,
    len: usize,
}

impl SliceIndexes {
    pub fn new(position: usize, len: usize) -> Self {
        Self { position, len }
    }

    pub fn from_slice<T>(data: &[T], slice: &[T], slice_pos: usize) -> Self {
        let slice_start: usize = unsafe { slice.as_ptr().offset_from(data.as_ptr()) }
            .try_into()
            .unwrap();
        let position = slice_start + slice_pos;
        let len = slice.len() - slice_pos;
        SliceIndexes { position, len }
    }

    pub fn get<T>(&self, data: &[T]) -> &[T] {
        unsafe { std::slice::from_raw_parts(data.as_ptr().add(self.position), self.len) }
    }
}

impl<T> RefSlice<T> {
    pub fn new(position: usize, len: usize) -> Self {
        Self::Referenced(SliceIndexes::new(position, len))
    }

    pub fn from_slice(data: &[T], slice: &[T], slice_pos: usize) -> RefSlice<T> {
        RefSlice::Referenced(SliceIndexes::from_slice(data, slice, slice_pos))
    }

    pub fn get(&self, data: &[T]) -> &[T] {
        match self {
            Self::Owned(o) => o,
            Self::Referenced(r) => r.get(data),
        }
    }
}

impl<T> From<Vec<T>> for RefSlice<T> {
    fn from(value: Vec<T>) -> Self {
        RefSlice::Owned(value)
    }
}
