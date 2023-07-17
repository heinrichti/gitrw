use std::{
    error::Error,
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
};

use bstr::{
    io::{BufReadExt, ByteLines},
    BStr, BString, ByteSlice,
};
use rustc_hash::FxHashMap;

use crate::{
    objs::{CommitHash, Tag, TagTargetType},
    shared::ObjectHash,
    Repository, WriteObject,
};

trait RefName {
    fn get_name(&self) -> &BStr;
    fn get_target(&self) -> &BStr;
}

#[derive(Debug)]
pub enum GitRef {
    Simple(SimpleRef),
    Tag(TagRef),
}

impl RefName for GitRef {
    fn get_name(&self) -> &BStr {
        match self {
            GitRef::Simple(simple) => simple.name[..].as_bstr(),
            GitRef::Tag(tag) => tag.name[..].as_bstr(),
        }
    }

    fn get_target(&self) -> &BStr {
        match self {
            GitRef::Simple(simple) => simple.hash[..].as_bstr(),
            GitRef::Tag(tag) => tag.hash[..].as_bstr(),
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

    pub fn update(
        repository: &mut Repository,
        rewritten_commits: &FxHashMap<CommitHash, CommitHash>,
    ) {
        for r in repository.refs().unwrap() {
            Self::rewrite_ref(repository, r.get_name(), r.get_target(), rewritten_commits);
        }

        // TODO delete packed refs
        // todo!()
    }

    fn rewrite_ref(
        repository: &mut Repository,
        ref_name: &BStr,
        ref_target: &BStr,
        rewritten_commits: &FxHashMap<CommitHash, CommitHash>,
    ) -> ObjectHash {
        let tag_target_obj = repository
            .read_object(ref_target.try_into().unwrap())
            .unwrap();
        match tag_target_obj {
            crate::objs::GitObject::Commit(_) => {
                let mut ref_path_buf = repository.path.clone();
                ref_path_buf.push(ref_name.to_string());
                let file_name = ref_path_buf.file_name().unwrap();
                let ref_path = ref_path_buf.to_str().unwrap();
                let dir_path = &ref_path[0..ref_path.len() - file_name.len()];

                std::fs::create_dir_all(dir_path).unwrap();

                let tag_target: CommitHash = ref_target.try_into().unwrap();
                let rewritten_target = rewritten_commits.get(&tag_target);
                match rewritten_target {
                    Some(new_target) => {
                        std::fs::write(ref_path, new_target.to_string()).unwrap();
                        new_target.clone().0
                    }
                    None => tag_target.0,
                }
            }
            crate::objs::GitObject::Tree(tree) => {
                println!(
                    "Skipping tag pointing to tree (not supported yet): {}",
                    ref_name
                );
                tree.hash().clone().0
            }
            crate::objs::GitObject::Tag(mut target_tag) => match target_tag.target_type() {
                TagTargetType::Commit => {
                    let target_tag_object = rewritten_commits.get(&CommitHash(target_tag.object()));
                    let target_hash = target_tag_object.map(|target_tag_object| {
                        target_tag.set_object(target_tag_object.clone().0);
                        let tag = Tag::create(None, target_tag.to_bytes(), false);
                        Repository::write(repository.path.clone(), &tag);
                        tag.hash().clone()
                    });

                    match target_hash {
                        Some(target_hash) => {
                            let path: PathBuf = [
                                repository.path.to_str().unwrap(),
                                ref_name.to_str().unwrap(),
                            ]
                            .iter()
                            .collect();

                            let file_name = path.file_name().unwrap();
                            let ref_path = path.to_str().unwrap();
                            let dir_path = &ref_path[0..ref_path.len() - file_name.len()];
                            std::fs::create_dir_all(dir_path).unwrap();

                            std::fs::write(path, target_hash.to_string()).unwrap();
                            target_hash.clone()
                        }
                        None => target_tag.hash().clone(),
                    }
                }
                TagTargetType::Tree => {
                    println!(
                        "Skipping tag pointing to tree (not supported yet): {}",
                        target_tag.name()
                    );
                    target_tag.hash().clone()
                }
                TagTargetType::Tag => {
                    panic!("Did not expect a tag to point to another tag");
                }
            },
        }
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
