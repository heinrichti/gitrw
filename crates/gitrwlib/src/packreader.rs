use core::panic;
use std::error::Error;

use std::fs::{self, File};
use std::hash::BuildHasherDefault;
use std::path::Path;
use std::sync::{Arc, RwLock};

use memmap2::Mmap;
use rustc_hash::FxHashMap;

use crate::compression::Decompression;
use crate::idx_reader::get_pack_offsets;
use crate::objs::{CommitBase, Tag};
use crate::objs::{GitObject, Tree};
use crate::pack_diff::PackDiff;
use crate::shared::ObjectHash;

#[derive(Debug)]
struct Pack {
    idx_file: String,
    pack_file: String,
}

struct PackWithObjects {
    pack: Mmap,
    objects: Arc<RwLock<FxHashMap<ObjectHash, usize>>>,
    pack_file: String,
}

#[derive(Clone)]
pub struct PackReader {
    packs: Vec<PackWithObjects>,
}

impl Clone for PackWithObjects {
    fn clone(&self) -> Self {
        let pack_file = File::open(self.pack_file.clone()).unwrap();
        let pack_map = unsafe { Mmap::map(&pack_file).unwrap() };

        Self {
            pack: pack_map,
            objects: self.objects.clone(),
            pack_file: self.pack_file.clone(),
        }
    }
}

impl PackReader {
    pub fn create(repository_path: &Path) -> Result<PackReader, Box<dyn Error>> {
        let mut packs_with_objects = Vec::new();

        for pack in get_packs(repository_path).into_iter() {
            let pack_file = File::open(pack.pack_file.clone())?;
            let pack_map = unsafe { Mmap::map(&pack_file)? };

            let pack_offsets = get_pack_offsets(Path::new(&pack.idx_file)).unwrap();
            let offsets = Arc::new(RwLock::new(FxHashMap::with_capacity_and_hasher(
                pack_offsets.len(),
                BuildHasherDefault::default(),
            )));

            for offset in pack_offsets.into_iter() {
                offsets.write().unwrap().insert(offset.hash, offset.offset);
            }

            packs_with_objects.push(PackWithObjects {
                pack: pack_map,
                objects: offsets,
                pack_file: pack.pack_file,
            });
        }

        Ok(PackReader {
            packs: packs_with_objects,
        })
    }

    pub fn read_git_object(
        &self,
        decompression: &mut Decompression,
        object_hash: ObjectHash,
    ) -> Option<GitObject> {
        if let Some(r) = self.read_git_object_bytes(decompression, &object_hash) {
            let git_object = match r.1.object_type {
                1u8 => GitObject::Commit(CommitBase::create(object_hash.into(), r.0, false)),
                2u8 => GitObject::Tree(Tree::create(object_hash.into(), r.0, false)),
                // 3u8 => GitObject::Blob(Blob::create(object_hash, bytes)),
                4u8 => GitObject::Tag(Tag::create(object_hash.into(), r.0, false)),
                _ => panic!("unknown git object type"),
            };

            Some(git_object)
        } else {
            None
        }
    }

    pub fn read_git_object_bytes(
        &self,
        decompression: &mut Decompression,
        object_hash: &ObjectHash,
    ) -> Option<(Box<[u8]>, PackObject)> {
        if let Some((mmap, offset)) = get_offset(self, object_hash) {
            let bytes: Box<[u8]>;

            let mut pack_object = PackObject::create(mmap, offset);
            if pack_object.object_type == 6 {
                // diff
                (bytes, pack_object) = restore_diff_object_bytes(decompression, mmap, pack_object);
            } else if pack_object.object_type == 7 {
                // OBJ_REF_DELTA: 20 bytes for the base object hash, then the instructions
                let slice_start = pack_object.offset + pack_object.header_len;
                let base_object_hash: ObjectHash =
                    mmap[slice_start..slice_start + 20].try_into().unwrap();

                let base = self
                    .read_git_object_bytes(decompression, &base_object_hash)
                    .unwrap();

                let pack_diff = PackDiff::create_for_ref(decompression, mmap, &pack_object);
                bytes = pack_diff.apply(&base.0);
                pack_object = base.1;
            } else {
                // plain object, should be easy to extract
                bytes = decompression.unpack(mmap, &pack_object, 0);
            }

            return Some((bytes, pack_object));
        }

        None
    }
}

fn restore_diff_object_bytes(
    compression: &mut Decompression,
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
        if let Some(result) = pack
            .objects
            .read()
            .unwrap()
            .get(object_hash)
            .map(|x| (&pack.pack, *x))
        {
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
