use std::fmt::Display;

use bstr::{BStr, BString, ByteSlice};

use super::ObjectHash;

impl Display for ObjectHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(hex::encode(self.bytes).as_str())?;
        Ok(())
    }
}

impl std::fmt::Debug for ObjectHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.write_str(hex::encode(self.bytes).as_str())?;
        Ok(())
    }
}

impl ObjectHash {
    pub(crate) fn try_from_bstr<T: From<ObjectHash>>(hash: &BStr) -> Result<T, &'static str> {
        if hash.len() != 40 {
            return Err("ObjectHash has to be 40 characters");
        }

        let bytes = std::array::from_fn(|i| {
            HASH_VALUE[hash[2 * i] as usize] << 4 | HASH_VALUE[hash[2 * i + 1] as usize]
        });

        Ok(ObjectHash::from(bytes).into())
    }
}

impl TryFrom<&[u8]> for ObjectHash {
    type Error = &'static str;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() != 20 {
            Err("ObjectHash has to be 20 bytes")
        } else {
            let mut buf = [0u8; 20];
            buf.copy_from_slice(value);
            Ok(ObjectHash::from(buf))
        }
    }
}

impl TryFrom<&BStr> for ObjectHash {
    type Error = &'static str;

    fn try_from(value: &BStr) -> Result<Self, Self::Error> {
        ObjectHash::try_from_bstr(value)
    }
}

impl TryFrom<BString> for ObjectHash {
    type Error = &'static str;

    fn try_from(value: BString) -> Result<Self, Self::Error> {
        ObjectHash::try_from_bstr(value.as_bstr())
    }
}

impl From<[u8; 20]> for ObjectHash {
    fn from(value: [u8; 20]) -> Self {
        ObjectHash { bytes: value }
    }
}

impl From<ObjectHash> for Vec<u8> {
    fn from(val: ObjectHash) -> Self {
        val.bytes.to_vec()
    }
}

const HASH_VALUE: &[u8] = &[
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 2, 3, 4, 5, 6, 7,
    8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31,
    32,
];
