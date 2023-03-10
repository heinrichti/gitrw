use std::{
    error::Error,
    fs::File,
    io::{BufReader, Read},
    path::Path,
};

use flate2::Status;
use libdeflater::Decompressor;
use memmap2::Mmap;

use crate::packreader::PackObject;

pub struct Compression {
    libdeflate_decompressor: Decompressor,
    flate2_decompressor: flate2::Decompress
}

static mut FILE_BUF: [u8; 8192] = [0u8; 8192];

impl Compression {
    pub fn new() -> Self {
        Compression {
            libdeflate_decompressor: Decompressor::new(),
            flate2_decompressor: flate2::Decompress::new(false)
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

        self.libdeflate_decompressor.deflate_decompress(slice, &mut buf).unwrap();

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
        let mut buf_reader = BufReader::new(file);
        buf_reader.seek_relative(2).unwrap();

        let mut output_buf = Vec::new();

        self.flate2_decompressor.reset(false);

        let mut status = Status::Ok;
        while status == Status::Ok {
            let bytes_read = buf_reader.read(unsafe { &mut FILE_BUF }).unwrap();
            output_buf.reserve(bytes_read*2);

            status = self.flate2_decompressor
                .decompress_vec(unsafe { &FILE_BUF[0..bytes_read] }, &mut output_buf, flate2::FlushDecompress::None)
                .unwrap();
        }

        Ok(output_buf.into_boxed_slice())
    }
}
