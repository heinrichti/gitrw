use std::{
    error::Error,
    fs::File,
    io::{BufReader, Read, Seek, SeekFrom},
    path::Path,
};

use crate::object_hash::ObjectHash;

const HEADER_LEN: usize = 8;
const HASH_LEN: usize = 20;
const FANOUT_LEN: usize = 4;
const HASHES_TABLE_START: usize = HEADER_LEN + 256 * FANOUT_LEN;

pub struct PackOffset {
    pub hash: ObjectHash,
    pub offset: usize,
}

pub fn get_pack_offsets(idx_path: &Path) -> Result<Vec<PackOffset>, Box<dyn Error>> {
    let file = File::open(idx_path)?;
    let mut reader = BufReader::new(file);

    let mut buffer = Vec::with_capacity(HASHES_TABLE_START);
    unsafe { buffer.set_len(HASHES_TABLE_START) };

    reader.read_exact(&mut buffer)?;
    verify_header(&buffer)?;

    let object_count = get_file_count_from_fanout(&buffer[HEADER_LEN + 255 * FANOUT_LEN..]);
    let mut result = Vec::with_capacity(object_count);
    if object_count == 0 {
        return Ok(result);
    }

    let mut hashes = Vec::with_capacity(object_count);
    for _ in 0..object_count {
        let mut hash = [0u8; 20];
        reader.read_exact(&mut hash)?;
        hashes.push(hash);
    }

    let offset: u64 =
        HASHES_TABLE_START as u64 + HASH_LEN as u64 * object_count as u64 + 4 * object_count as u64;
    reader.seek(SeekFrom::Start(offset))?;

    let mut pack_offset = [0u8; 4];
    let mut large_offsets = Vec::new();
    for hash in hashes {
        reader.read_exact(&mut pack_offset)?;
        let mut offset: usize = pack_offset[3] as usize;
        offset += (pack_offset[2] as usize) << 8;
        offset += (pack_offset[1] as usize) << 16;
        offset += ((pack_offset[0] & 0b01111111) as usize) << 24;

        if msb_set(&pack_offset) {
            large_offsets.push(hash);
        } else {
            result.push(PackOffset {
                hash: ObjectHash::new(hash),
                offset,
            });
        }
    }

    let offset: u64 = HASHES_TABLE_START as u64
        + HASH_LEN as u64 * object_count as u64
        + 4 * object_count as u64
        + 4 * object_count as u64;
    reader.seek(SeekFrom::Start(offset))?;

    let mut pack_offset = [0u8; 8];
    for large_offset in large_offsets {
        reader.read_exact(&mut pack_offset)?;
        if cfg!(target_endian = "little") {
            pack_offset.reverse();
        }

        result.push(PackOffset {
            hash: ObjectHash::new(large_offset),
            offset: usize::from_be_bytes(pack_offset),
        });
    }

    Ok(result)
}

#[inline]
fn msb_set(pack_offset: &[u8]) -> bool {
    (pack_offset[0] & 0b10000000) != 0
}

fn get_file_count_from_fanout(bytes: &[u8]) -> usize {
    assert!(bytes.len() >= 4);
    let mut result: usize = bytes[3] as usize;
    result += (bytes[2] as usize) << 8;
    result += (bytes[1] as usize) << 16;
    result += (bytes[0] as usize) << 24;

    result
}

#[derive(Debug)]
pub enum IdxError {
    InvalidHeader,
}

impl std::error::Error for IdxError {}

impl std::fmt::Display for IdxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            &IdxError::InvalidHeader => f.write_str("IDX file has invalid header."),
        }
    }
}

fn verify_header(buffer: &[u8]) -> Result<(), IdxError> {
    if buffer[0] == 255
        && buffer[1] == b't'
        && buffer[2] == b'O'
        && buffer[3] == b'c'
        && buffer[4] == 0
        && buffer[5] == 0
        && buffer[6] == 0
        && buffer[7] == 2
    {
        return Ok(());
    }

    Err(IdxError::InvalidHeader)
}

#[cfg(test)]
mod test {

    use super::verify_header;

    #[test]
    pub fn header_test() {
        let buf = [0u8; 1024];

        let r = verify_header(&buf);
        assert!(r.is_err());
    }
}
