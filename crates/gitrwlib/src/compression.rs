use std::{
    error::Error,
    fs::File,
    io::{self, BufReader, BufWriter, Read, Write},
    path::Path,
};

use flate2::Status;
use libdeflater::Decompressor;
use memmap2::Mmap;
use once_cell::sync::Lazy;

use crate::{packreader::PackObject, WriteBytes};

pub struct Decompression {
    libdeflate_decompressor: Decompressor,
    flate2_decompressor: flate2::Decompress,
    file_buf: Lazy<[u8; 8192]>,
}

impl Default for Decompression {
    fn default() -> Self {
        Self {
            libdeflate_decompressor: Decompressor::new(),
            flate2_decompressor: flate2::Decompress::new(false),
            file_buf: Lazy::new(|| [0u8; 8192]),
        }
    }
}

pub fn pack_file(path: &Path, prefix: &str, write_bytes: &WriteBytes) -> Result<(), io::Error> {
    let file = File::options()
        .read(true)
        .write(true)
        .create_new(true)
        .open(path)?;
    let data = &write_bytes.bytes[write_bytes.start..];
    let mut buf_writer = BufWriter::new(file);
    let preamble: Vec<_> = format!("{} {}\0", prefix, data.len()).bytes().collect();

    let mut compress = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    compress.write_all(&preamble).unwrap();
    compress.write_all(data).unwrap();
    let data = compress.finish().unwrap();

    buf_writer.write_all(&data).unwrap();

    Ok(())
}

impl Decompression {
    #[allow(clippy::uninit_vec)]
    pub fn unpack(
        &mut self,
        mmap: &Mmap,
        pack_object: &PackObject,
        additional_offset: usize,
    ) -> Box<[u8]> {
        let slice = &mmap[pack_object.offset + pack_object.header_len + additional_offset + 2..];

        let mut buf: Vec<u8> = Vec::with_capacity(pack_object.data_size);
        unsafe { buf.set_len(pack_object.data_size) };

        self.libdeflate_decompressor
            .deflate_decompress(slice, &mut buf)
            .unwrap();

        buf.into_boxed_slice()
    }

    pub fn unpack_file(
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

        let buffer = &mut self.file_buf[..];

        let mut status = Status::Ok;
        while status == Status::Ok {
            let bytes_read = buf_reader.read(buffer.as_mut()).unwrap();
            if bytes_read == 0 {
                break;
            }

            output_buf.reserve(bytes_read * 2);

            status = self
                .flate2_decompressor
                .decompress_vec(
                    &buffer[0..bytes_read],
                    &mut output_buf,
                    flate2::FlushDecompress::None,
                )
                .unwrap();
        }

        Ok(output_buf.into_boxed_slice())
    }
}
