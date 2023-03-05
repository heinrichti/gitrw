use memmap2::Mmap;

use crate::{hash_content::Compression, packreader::PackObject};

pub struct CopyInstruction {
    offset: usize,
    len: usize,
}

impl CopyInstruction {
    fn create(data: &[u8], current_offset: &mut usize) -> CopyInstruction {
        let copy_instruction = data[*current_offset];
        *current_offset += 1;

        let mut offset = 0;
        let mut len = 0;

        if (copy_instruction & 0b00000001) != 0 {
            offset |= data[*current_offset] as usize;
            *current_offset += 1;
        }

        if (copy_instruction & 0b00000010) != 0 {
            offset |= (data[*current_offset] as usize) << 8;
            *current_offset += 1;
        }

        if (copy_instruction & 0b00000100) != 0 {
            offset |= (data[*current_offset] as usize) << 16;
            *current_offset += 1;
        }

        if (copy_instruction & 0b00001000) != 0 {
            offset |= (data[*current_offset] as usize) << 24;
            *current_offset += 1;
        }

        if (copy_instruction & 0b00010000) != 0 {
            len |= data[*current_offset] as usize;
            *current_offset += 1;
        }

        if (copy_instruction & 0b00100000) != 0 {
            len |= (data[*current_offset] as usize) << 8;
            *current_offset += 1;
        }

        if (copy_instruction & 0b01000000) != 0 {
            len |= (data[*current_offset] as usize) << 16;
            *current_offset += 1;
        }

        if len == 0 {
            len = 0x10000;
        }

        CopyInstruction { offset, len }
    }
}

impl std::fmt::Debug for CopyInstruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "CopyInstruction. Offset: {} Len: {}",
            &self.offset, &self.len
        ))
    }
}

#[derive(Clone)]
pub struct AddInstruction {
    bytes: Box<[u8]>,
}

impl std::fmt::Debug for AddInstruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "AddInstruction. Start: {} End: {}",
            0,
            self.bytes.len()
        ))
    }
}

impl AddInstruction {
    fn create(data: &[u8], current_offset: &mut usize) -> AddInstruction {
        let bytes_to_copy = data[*current_offset] as usize;
        *current_offset += 1;
        let bytes = data[*current_offset..*current_offset + bytes_to_copy]
            .to_owned()
            .into_boxed_slice();
        let instruction = AddInstruction { bytes };
        *current_offset += bytes_to_copy;
        instruction
    }
}

pub enum DiffInstruction {
    Copy(CopyInstruction),
    Add(AddInstruction),
}

impl DiffInstruction {
    fn len(&self) -> usize {
        match &self {
            DiffInstruction::Copy(copy) => copy.len,
            DiffInstruction::Add(add) => add.bytes.len(),
        }
    }
}

pub struct PackDiff {
    pub target_len: usize,
    pub negative_offset: usize,
    pub instructions: Vec<DiffInstruction>,
}

impl PackDiff {
    pub fn create(
        compression: &mut Compression,
        mmap: &Mmap,
        pack_object: &PackObject,
    ) -> PackDiff {
        let (base_offset, bytes_read) = read_base_offset(mmap, pack_object);

        let diff_instruction_bytes = compression.unpack(mmap, pack_object, bytes_read);

        let (_, bytes_read) = read_varint(&diff_instruction_bytes, 0);
        let (target_len, bytes_read) = read_varint(&diff_instruction_bytes, bytes_read);

        let instructions =
            build_delta_instructions(diff_instruction_bytes, pack_object, bytes_read);

        PackDiff {
            instructions,
            target_len,
            negative_offset: base_offset,
        }
    }

    pub fn combine(self, other: &PackDiff) -> PackDiff {
        let mut instructions = Vec::new();

        for instruction in self.instructions.into_iter() {
            match instruction {
                DiffInstruction::Copy(copy) => {
                    let instructions_from_copy: Vec<DiffInstruction> =
                        get_instructions_from_copy(&copy, other);
                    for i in instructions_from_copy {
                        instructions.push(i);
                    }
                }
                DiffInstruction::Add(add) => {
                    let diff_instruction = DiffInstruction::Add(add);
                    instructions.push(diff_instruction);
                }
            };
        }

        PackDiff {
            target_len: self.target_len,
            negative_offset: other.negative_offset,
            instructions,
        }
    }

    pub fn apply(&self, bytes: &[u8]) -> Box<[u8]> {
        let mut target = Vec::with_capacity(self.target_len);
        unsafe { target.set_len(self.target_len) };
        let mut target_offset = 0;

        for instruction in self.instructions.iter() {
            match instruction {
                DiffInstruction::Add(add) => {
                    let len = add.bytes.len();
                    target[target_offset..target_offset + len].copy_from_slice(&add.bytes);
                    target_offset += len;
                }
                DiffInstruction::Copy(copy) => {
                    target[target_offset..target_offset + copy.len]
                        .copy_from_slice(&bytes[copy.offset..copy.offset + copy.len]);
                    target_offset += copy.len;
                }
            }
        }

        target.into_boxed_slice()
    }
}

fn get_instructions_from_copy(
    copy_instruction: &CopyInstruction,
    source: &PackDiff,
) -> Vec<DiffInstruction> {
    let mut result = Vec::<DiffInstruction>::new();
    let mut current_source_offset = 0;
    let mut copy_instruction_consumed = 0;

    let end_offset = copy_instruction.offset + copy_instruction.len;

    for source_instruction in source.instructions.iter() {
        let source_instruction_len = source_instruction.len();
        if copy_instruction.offset < current_source_offset + source_instruction.len()
            && end_offset > current_source_offset
        {
            let source_instruction_offset =
                copy_instruction.offset + copy_instruction_consumed - current_source_offset;
            let bytes_to_take = if source_instruction.len() - source_instruction_offset
                <= copy_instruction.len - copy_instruction_consumed
            {
                source_instruction.len() - source_instruction_offset
            } else {
                copy_instruction.len - copy_instruction_consumed
            };

            result.push(match source_instruction {
                DiffInstruction::Copy(copy) => DiffInstruction::Copy(CopyInstruction {
                    offset: copy.offset + source_instruction_offset,
                    len: bytes_to_take,
                }),
                DiffInstruction::Add(add) => DiffInstruction::Add(AddInstruction {
                    bytes: add.bytes
                        [source_instruction_offset..source_instruction_offset + bytes_to_take]
                        .to_vec()
                        .into_boxed_slice(),
                }),
            });

            copy_instruction_consumed += bytes_to_take;
        } else if end_offset < current_source_offset {
            break;
        }

        current_source_offset += source_instruction_len;
    }

    result
}

fn read_varint(delta_data: &[u8], mut offset: usize) -> (usize, usize) {
    let mut byte = delta_data[offset];
    offset += 1;
    let mut len = (byte & 0b01111111) as usize;
    let mut fsb_set = (byte & 0b10000000) != 0;
    let mut shift = 7;
    while fsb_set {
        byte = delta_data[offset];
        offset += 1;
        fsb_set = (byte & 0b10000000) != 0;
        len |= ((byte & 0b01111111) as usize) << shift;
        shift += 7;
    }

    (len, offset)
}

fn build_delta_instructions(
    diff_data: Box<[u8]>,
    pack_object: &PackObject,
    mut bytes_read: usize,
) -> Vec<DiffInstruction> {
    let mut result: Vec<DiffInstruction> = Vec::new();
    while bytes_read < pack_object.data_size {
        let instruction = diff_data[bytes_read];

        if (instruction & 0b10000000) != 0 {
            let copy_instruction = CopyInstruction::create(&diff_data, &mut bytes_read);
            result.push(DiffInstruction::Copy(copy_instruction));
        } else {
            let add_instruction = AddInstruction::create(&diff_data, &mut bytes_read);
            result.push(DiffInstruction::Add(add_instruction));
        }
    }

    result
}

fn read_base_offset(mmap: &Mmap, pack_object: &PackObject) -> (usize, usize) {
    let mut byte = mmap
        .get(pack_object.offset + pack_object.header_len)
        .unwrap();
    let mut bytes_read = 1;
    let mut offset = (byte & 127) as usize;

    while (byte & 128) != 0 {
        offset += 1;
        byte = mmap
            .get(pack_object.offset + pack_object.header_len + bytes_read)
            .unwrap();
        bytes_read += 1;
        offset <<= 7;
        offset += (byte & 127) as usize;
    }

    (offset, bytes_read)
}

#[cfg(test)]
mod test {
    use std::vec;

    use super::{AddInstruction, CopyInstruction, DiffInstruction, PackDiff};

    #[test]
    pub fn patch_diff() {
        let base = Vec::from("hello world");
        let add_text = Vec::from(", this is a test");

        let base_diff = PackDiff {
            negative_offset: 1000,
            target_len: base.len() + add_text.len(),
            instructions: vec![
                DiffInstruction::Copy(CopyInstruction {
                    offset: 0,
                    len: base.len(),
                }),
                DiffInstruction::Add(AddInstruction {
                    bytes: add_text.into(),
                }),
            ],
        };

        // hello world, this is a test
        // huhu world, is a test good?
        let target_text = Vec::from("huhu world, is a test good?");
        let huhu_text = Vec::from("huhu");
        let q_text = Vec::from("is a test good?");

        let next_diff = PackDiff {
            negative_offset: 50,
            target_len: target_text.len(),
            instructions: vec![
                DiffInstruction::Add(AddInstruction {
                    bytes: huhu_text.into(),
                }),
                DiffInstruction::Copy(CopyInstruction { offset: 5, len: 8 }),
                DiffInstruction::Add(AddInstruction {
                    bytes: q_text.into(),
                }),
            ],
        };

        let diff = next_diff.combine(&base_diff);
        let bytes = diff.apply(&base);

        assert_eq!(target_text.len(), diff.target_len);
        assert_eq!(*bytes, target_text);
        // println!("Text: {}", bytes.to_str().unwrap());
    }
}
