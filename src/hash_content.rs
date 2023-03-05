use std::{
    error::Error,
    fs::File,
    io::{BufReader, Read},
    path::Path,
};

use memmap2::Mmap;

use crate::packreader::PackObject;

pub struct Compression {
    decompressor: flate2::Decompress,
}

impl Compression {
    pub fn new() -> Self {
        Compression {
            decompressor: flate2::Decompress::new(true),
        }
    }

    pub fn unpack(
        &mut self,
        mmap: &Mmap,
        pack_object: &PackObject,
        additional_offset: usize,
    ) -> Box<[u8]> {
        let slice = &mmap[pack_object.offset + pack_object.header_len + additional_offset + 2..];

        let mut buf: Vec<u8> = Vec::with_capacity(pack_object.data_size);
        unsafe { buf.set_len(pack_object.data_size) };

        self.decompressor.reset(false);
        self.decompressor
            .decompress(slice, &mut buf, flate2::FlushDecompress::Finish)
            .unwrap();

        buf.into_boxed_slice()
    }

    pub fn from_file(
        &mut self,
        base_path: &Path,
        hash_code: &str,
    ) -> Result<Box<[u8]>, Box<dyn Error>> {
        let (x, xs) = hash_code.split_at(2);
        let file_path = base_path.join("objects").join(x).join(xs);

        let file = File::open(file_path)?;
        let file_size: usize = File::metadata(&file).unwrap().len() as usize;
        let mut file_buffer = Vec::with_capacity(file_size - 2);
        let mut buf_reader = BufReader::new(file);
        buf_reader.seek_relative(2).unwrap();
        let bytes_read = buf_reader.read_to_end(&mut file_buffer).unwrap();
        if bytes_read != file_size - 2 {
            panic!("bytes_read[{bytes_read}] != file_size[{file_size}]");
        }

        let mut buf = Vec::with_capacity(file_size * 2);
        self.decompressor.reset(false);
        self.decompressor
            .decompress_vec(&file_buffer, &mut buf, flate2::FlushDecompress::Finish)
            .unwrap();

        Ok(buf.into_boxed_slice())
    }
}
