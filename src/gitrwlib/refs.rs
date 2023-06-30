use std::{error::Error, fs::File, io::BufReader, path::Path};

use bstr::{
    io::{BufReadExt, ByteLines},
    BString, ByteSlice,
};

trait RefName {
    fn get_name(&self) -> &BString;
}

#[derive(Debug)]
pub enum GitRef {
    Simple(SimpleRef),
    Tag(TagRef),
}

impl RefName for GitRef {
    fn get_name(&self) -> &BString {
        match self {
            GitRef::Simple(simple) => &simple.name,
            GitRef::Tag(tag) => &tag.name,
        }
    }
}

#[derive(Debug)]
pub struct SimpleRef {
    pub name: bstr::BString,
    pub hash: bstr::BString,
}

#[derive(Debug)]
pub struct TagRef {
    pub name: BString,
    pub hash: BString,
    pub obj_hash: BString,
}

impl GitRef {
    pub fn read_all(base_path: &std::path::Path) -> Result<Vec<GitRef>, Box<dyn Error>> {
        let packed_refs_path = base_path.join("packed-refs");

        let file = File::open(packed_refs_path);
        let packed_refs = match file {
            Ok(file) => {
                let reader = BufReader::new(file);

                let packed_refs = get_packed_refs(&mut reader.byte_lines())?;
                Some(packed_refs)
            }
            Err(_) => None,
        };

        let mut refs = get_loose_refs(base_path, "refs");

        if let Some(mut p) = packed_refs {
            refs.append(&mut p);
            refs.dedup_by(|x, y| x.get_name() == y.get_name());
        }

        Ok(refs)
    }
}

fn get_loose_refs(base_path: &Path, current_path: &str) -> Vec<GitRef> {
    let mut result: Vec<GitRef> = Vec::new();

    let full_path = base_path.join(current_path);
    for dir_entry in std::fs::read_dir(&full_path).unwrap().map(|x| x.unwrap()) {
        let file_type = dir_entry.file_type().unwrap();
        if file_type.is_dir() {
            let mut next_path = String::new();
            next_path.push_str(current_path);
            next_path.push('/');
            next_path.push_str(dir_entry.path().file_name().unwrap().to_str().unwrap());
            result.append(&mut get_loose_refs(base_path, &next_path));
        } else {
            let hash = BString::from(
                std::fs::read_to_string(&dir_entry.path())
                    .unwrap()
                    .trim_end(),
            );

            let mut name = String::new();
            name.push_str(current_path);
            name.push('/');
            name.push_str(dir_entry.file_name().to_str().unwrap());

            if !hash.starts_with(b"ref: ") {
                result.push(GitRef::Simple(SimpleRef {
                    name: BString::from(name),
                    hash,
                }))
            }
        }
    }

    result
}

fn get_packed_refs(lines: &mut ByteLines<BufReader<File>>) -> Result<Vec<GitRef>, Box<dyn Error>> {
    let mut result: Vec<GitRef> = Vec::new();

    let mut previous_line = Some(lines.next().unwrap().unwrap());
    let mut line_started = previous_line
        .as_ref()
        .map(|x| !x.starts_with(b"#"))
        .unwrap();

    for current_line in lines.by_ref().flatten() {
        if current_line.starts_with(b"^") {
            if let Some(x) = previous_line.take() {
                let split = x.split_at(41);
                result.push(GitRef::Tag(TagRef {
                    hash: split.0[0..split.0.len() - 1].as_bstr().to_owned(),
                    name: split.1.as_bstr().to_owned(),
                    obj_hash: current_line.split_at(1).1.as_bstr().to_owned(),
                }));
            };

            line_started = false;
        } else {
            if line_started {
                if let Some(x) = previous_line.take() {
                    let split = x.split_at(41);
                    result.push(GitRef::Simple(SimpleRef {
                        hash: split.0[0..split.0.len() - 1].as_bstr().to_owned(),
                        name: split.1.as_bstr().to_owned(),
                    }));
                };
            }

            line_started = !current_line.starts_with(b"#");
            previous_line = Some(current_line);
        }
    }

    if line_started {
        let previous_line = previous_line.unwrap();
        let split = previous_line.split_at(41);
        let hash = split.0[..split.0.len() - 1].as_bstr().to_owned();
        let name = split.1.as_bstr().to_owned();
        result.push(GitRef::Simple(SimpleRef { hash, name }));
    }

    Ok(result)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn read_packed_refs() {
        let test = GitRef::read_all(std::path::Path::new(".git")).expect("Cannot read file");
        dbg!(test);
    }
}
