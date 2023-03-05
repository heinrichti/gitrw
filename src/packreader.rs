use core::panic;
use std::error::Error;

use std::fs::{self, File};
use std::hash::BuildHasherDefault;
use std::path::Path;

use memmap2::Mmap;
use rustc_hash::FxHashMap;

use crate::git_objects::{GitObject, Tree};
use crate::hash_content::Compression;
use crate::idx_reader::get_pack_offsets;
use crate::object_hash::ObjectHash;
use crate::objs::commit::Commit;
use crate::objs::tag::Tag;
use crate::pack_diff::PackDiff;

#[derive(Debug)]
struct Pack {
    idx_file: String,
    pack_file: String,
}

struct PackWithObjects {
    pack: Mmap,
    objects: FxHashMap<ObjectHash, usize>,
}

pub struct PackReader {
    packs: Vec<PackWithObjects>,
}

impl PackReader {
    pub fn create(repository_path: &Path) -> Result<PackReader, Box<dyn Error>> {
        let mut packs_with_objects = Vec::new();

        for pack in get_packs(repository_path).into_iter() {
            let pack_file = File::open(pack.pack_file)?;
            let pack_map = unsafe { Mmap::map(&pack_file)? };

            let pack_offsets = get_pack_offsets(Path::new(&pack.idx_file)).unwrap();
            let mut offsets = FxHashMap::with_capacity_and_hasher(
                pack_offsets.len(),
                BuildHasherDefault::default(),
            );

            for offset in pack_offsets.into_iter() {
                offsets.insert(offset.hash, offset.offset);
            }

            packs_with_objects.push(PackWithObjects {
                pack: pack_map,
                objects: offsets,
            });
        }

        Ok(PackReader {
            packs: packs_with_objects,
        })
    }

    pub fn read_git_object(
        &self,
        compression: &mut Compression,
        object_hash: ObjectHash,
    ) -> Option<GitObject> {
        if let Some((mmap, offset)) = get_offset(self, &object_hash) {
            let bytes: Box<[u8]>;

            let mut pack_object = PackObject::create(mmap, offset);
            if pack_object.object_type == 6 {
                // diff
                (bytes, pack_object) = restore_diff_object_bytes(compression, mmap, pack_object);
            } else if pack_object.object_type == 7 {
                panic!("OBJ_REF_DELTA not implemented");
            } else {
                // plain object, should be easy to extract
                bytes = compression.unpack(mmap, &pack_object, 0);
            }

            let git_object = match pack_object.object_type {
                1u8 => GitObject::Commit(Commit::create(object_hash, bytes, false)),
                2u8 => GitObject::Tree(Tree::create(object_hash, bytes, false)),
                // 3u8 => GitObject::Blob(Blob::create(object_hash, bytes)),
                4u8 => GitObject::Tag(Tag::create(object_hash, bytes, false)),
                _ => panic!("unknown git object type"),
            };

            return Some(git_object);
        }

        None
    }
}

fn restore_diff_object_bytes(
    compression: &mut Compression,
    mmap: &Mmap,
    mut pack_object: PackObject,
) -> (Box<[u8]>, PackObject) {
    let mut pack_diff = PackDiff::create(compression, mmap, &pack_object);
    pack_object = PackObject::create(mmap, pack_object.offset - pack_diff.negative_offset);

    while pack_object.object_type == 6 {
        // OFS_DELTA
        let target_diff = PackDiff::create(compression, mmap, &pack_object);
        pack_diff = pack_diff.combine(&target_diff);
        pack_object = PackObject::create(mmap, pack_object.offset - pack_diff.negative_offset);
    }

    let content = compression.unpack(mmap, &pack_object, 0);
    (pack_diff.apply(&content), pack_object)
}

fn get_offset<'a>(
    pack_reader: &'a PackReader,
    object_hash: &ObjectHash,
) -> Option<(&'a Mmap, usize)> {
    for pack in pack_reader.packs.iter() {
        if let Some(result) = pack.objects.get(object_hash).map(|x| (&pack.pack, *x)) {
            return Some(result);
        }
    }

    None
}

const TYPE_MASK: u8 = 0b01110000;

#[derive(Debug)]
pub struct PackObject {
    pub object_type: u8,
    pub offset: usize,
    pub header_len: usize,
    pub data_size: usize,
}

impl PackObject {
    pub fn create(mmap: &Mmap, offset: usize) -> PackObject {
        let mut read_byte = mmap.get(offset).unwrap();
        let mut bytes_read = 1;
        let mut fsb_set = (read_byte & 0b10000000) != 0;
        let object_type = (read_byte & TYPE_MASK) >> 4;
        let mut data_size: usize = (read_byte & 0b00001111) as usize;
        let mut shift = 4;
        while fsb_set {
            read_byte = mmap.get(offset + bytes_read).unwrap();
            bytes_read += 1;
            fsb_set = (read_byte & 0b10000000) != 0;
            data_size |= ((read_byte & 0x7F) as usize) << shift;
            shift += 7;
        }

        PackObject {
            object_type,
            offset,
            header_len: bytes_read,
            data_size,
        }
    }
}

fn get_packs(repository_path: &Path) -> Vec<Pack> {
    let mut packs = Vec::new();

    let pack_dir = repository_path.join("objects/pack");

    for file in fs::read_dir(pack_dir)
        .unwrap()
        .map(|x| x.unwrap())
        .filter(|x| !x.file_type().unwrap().is_dir())
    {
        let path_buf = file.path();
        let path = path_buf.to_str().unwrap();
        if path.ends_with(".idx") {
            let mut pack_path = String::from(path.split_at(path.len() - 4).0);
            pack_path.push_str(".pack");

            packs.push(Pack {
                idx_file: String::from(path),
                pack_file: pack_path,
            });
        }
    }

    packs
}
