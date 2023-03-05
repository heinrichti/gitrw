use std::{
    fmt::Display,
    hash::Hash,
    mem::{self, MaybeUninit},
};

use bstr::{BStr, BString, ByteSlice};

#[derive(Eq, PartialEq, Clone, Hash)]
pub struct ObjectHash {
    _bytes: [u8; 20],
}

impl Display for ObjectHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(hex::encode(self._bytes).as_str())?;
        Ok(())
    }
}

impl std::fmt::Debug for ObjectHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.write_str(hex::encode(self._bytes).as_str())?;
        Ok(())
    }
}

impl ObjectHash {
    pub fn new(bytes: [u8; 20]) -> ObjectHash {
        ObjectHash { _bytes: bytes }
    }

    fn from_bstr(hash: &BStr) -> ObjectHash {
        assert_eq!(hash.len(), 40);

        let mut bytes: [MaybeUninit<u8>; 20] = [MaybeUninit::uninit(); 20];
        for i in 0..bytes.len() {
            bytes[i].write(
                HASH_VALUE[hash[2 * i] as usize] << 4 | HASH_VALUE[hash[2 * i + 1] as usize],
            );
        }

        ObjectHash::new(unsafe { mem::transmute(bytes) })
    }
}

impl From<&BStr> for ObjectHash {
    fn from(value: &BStr) -> Self {
        ObjectHash::from_bstr(value)
    }
}

impl From<BString> for ObjectHash {
    fn from(value: BString) -> Self {
        ObjectHash::from_bstr(value.as_bstr())
    }
}

const HASH_VALUE: &[u8] = &[
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 2, 3, 4, 5, 6, 7,
    8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31,
    32,
];
