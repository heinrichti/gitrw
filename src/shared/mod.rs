use std::{marker::PhantomData, ops::Deref};

pub(crate) mod object_hash;

#[derive(Eq, PartialEq, Clone, Hash)]
pub struct ObjectHash {
    bytes: [u8; 20],
}

#[derive(Debug)]
pub enum RefSlice<'a, T> {
    Referenced(UnsafeSlice<'a, T>),
    Owned(Vec<T>),
}

#[derive(Debug)]
pub struct UnsafeSlice<'a, T> {
    ptr: *const T,
    len: usize,
    _phantom: PhantomData<&'a T>,
}

unsafe impl<'a, T> Send for UnsafeSlice<'a, T> {}

impl<'a, T> RefSlice<'a, T> {
    pub fn from_slice(slice: &[T]) -> RefSlice<'a, T> {
        RefSlice::Referenced(UnsafeSlice {
            ptr: slice.as_ptr(),
            len: slice.len(),
            _phantom: PhantomData,
        })
    }
}

impl<'a, T> From<Vec<T>> for RefSlice<'a, T> {
    fn from(value: Vec<T>) -> Self {
        RefSlice::Owned(value)
    }
}

impl<'a, T> Deref for RefSlice<'a, T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Referenced(r) => unsafe { std::slice::from_raw_parts(r.ptr, r.len) },
            Self::Owned(owned) => owned,
        }
    }
}
