use std::path::Path;

use bstr::ByteSlice;
use rustc_hash::FxHashSet;

use crate::{
    git_objects::{GitObject, TagTargetType, Tree},
    hash_content::Compression,
    object_hash::ObjectHash,
    objs::{commit::Commit, tag::Tag},
    packreader::PackReader,
};

pub struct CommitWalker<'a> {
    pack_reader: PackReader,
    compression: Compression,
    repository_path: &'a Path,
    commits: Vec<Commit<'a>>,
    processed_commits: FxHashSet<ObjectHash>,
}

impl<'a> CommitWalker<'a> {
    pub fn create(repository_path: &Path) -> CommitWalker {
        let mut commits = Vec::new();
        let processed_commits = FxHashSet::default();

        let pack_reader = PackReader::create(repository_path).unwrap();
        let mut compression = Compression::new();

        let refs = crate::refs::GitRef::read_all(repository_path).unwrap();
        for r in refs {
            let commit = read_commit_from_ref(&mut compression, repository_path, &pack_reader, r);
            if let Some(x) = commit {
                commits.push(x)
            };
        }

        let commits = commits
            .into_iter()
            .map(|git_object| match git_object {
                GitObject::Commit(commit) => commit,
                _ => panic!("this should have been a commit, but wasn't"),
            })
            .collect();

        CommitWalker {
            pack_reader,
            repository_path,
            commits,
            processed_commits,
            compression,
        }
    }
}

impl<'a> Iterator for CommitWalker<'a> {
    type Item = Commit<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(commit) = self.commits.pop() {
            if self.processed_commits.insert(commit.object_hash.clone()) {
                for parent in commit.parents() {
                    if let Some(parent_commit) = read_object_from_hash(
                        &mut self.compression,
                        self.repository_path,
                        &self.pack_reader,
                        parent,
                    ) {
                        match parent_commit {
                            GitObject::Commit(parent) => self.commits.push(parent),
                            _ => panic!("Expected a commit, but got something else"),
                        };
                    };
                }

                return Some(commit);
            }
        }

        None
    }
}

fn read_commit_from_ref<'a>(
    compression: &mut Compression,
    repository_path: &Path,
    pack_reader: &PackReader,
    r: crate::refs::GitRef,
) -> Option<GitObject<'a>> {
    let hash = match r {
        crate::refs::GitRef::Simple(simple) => simple.hash,
        crate::refs::GitRef::Tag(tag) => tag.hash,
    };

    let mut git_object =
        read_object_from_hash(compression, repository_path, pack_reader, hash.into()).unwrap();
    while let GitObject::Tag(tag) = &git_object {
        if tag.target_type() == TagTargetType::Tree {
            break;
        }

        git_object =
            read_object_from_hash(compression, repository_path, pack_reader, tag.object()).unwrap();
    }

    if let GitObject::Commit(commit) = git_object {
        return Some(GitObject::Commit(commit));
    }

    None
}

fn read_object_from_hash<'a>(
    compression: &mut Compression,
    repository_path: &Path,
    pack_reader: &PackReader,
    hash: ObjectHash,
) -> Option<GitObject<'a>> {
    if let Some(obj) = pack_reader.read_git_object(compression, hash.clone()) {
        return Some(obj);
    }

    if let Ok(bytes) = compression.from_file(repository_path, &hash.to_string()) {
        if bytes.starts_with(b"commit ") {
            return Some(GitObject::Commit(Commit::create(hash, bytes, true)));
        }

        if bytes.starts_with(b"tree ") {
            return Some(GitObject::Tree(Tree::create(hash, bytes, true)));
        }

        if bytes.starts_with(b"tag ") {
            return Some(GitObject::Tag(Tag::create(hash, bytes, true)));
        }

        if bytes.starts_with(b"blob ") {
            panic!()
            // return Some(GitObject::Blob(Blob::create(hash, bytes)));
        }

        dbg!(hash);
        dbg!(bytes.as_bstr());
        panic!("unknown loose git object type");
    }

    None
}
