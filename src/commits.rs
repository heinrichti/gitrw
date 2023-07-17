use core::panic;
use std::path::Path;

use bstr::ByteSlice;
use rustc_hash::FxHashSet;

use crate::{
    objs::{Commit, CommitHash, Tag, Tree},
    shared::ObjectHash,
};

use super::{
    compression::Decompression,
    objs::{GitObject, TagTargetType},
    packreader::PackReader,
    refs::GitRef,
};

pub struct CommitsFifoIter<'a> {
    pack_reader: &'a PackReader,
    compression: &'a mut Decompression,
    repository_path: &'a Path,
    commits: Vec<Commit>,
    processed_commits: FxHashSet<CommitHash>,
    parents_seen: FxHashSet<CommitHash>,
}

impl<'a> CommitsFifoIter<'a> {
    pub fn create(
        repository_path: &'a Path,
        pack_reader: &'a PackReader,
        compression: &'a mut Decompression,
    ) -> Self {
        let mut commits = Vec::new();
        let processed_commits = FxHashSet::default();
        let parents_seen = FxHashSet::default();

        let refs = GitRef::read_all(repository_path).unwrap();
        for r in refs {
            let commit = read_commit_from_ref(compression, repository_path, pack_reader, r);
            if let Some(x) = commit {
                commits.push(x);
            };
        }

        let commits = commits
            .into_iter()
            .map(|git_object| match git_object {
                GitObject::Commit(commit) => commit,
                _ => panic!("this should have been a commit, but wasn't"),
            })
            .collect();

        CommitsFifoIter {
            pack_reader,
            compression,
            repository_path,
            commits,
            processed_commits,
            parents_seen,
        }
    }
}

impl<'a> Iterator for CommitsFifoIter<'a> {
    type Item = Commit;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(commit) = self.commits.pop() {
            if self.processed_commits.contains(commit.hash()) {
                self.parents_seen.remove(commit.hash());
            } else if !self.parents_seen.insert(commit.hash().clone())
                || commit.parents().is_empty()
            {
                self.processed_commits.insert(commit.hash().clone());
                return Some(commit);
            } else {
                let parents = commit.parents();
                self.commits.push(commit);
                for parent in parents {
                    if !self.processed_commits.contains(&parent) {
                        let parent_commit = read_object_from_hash(
                            self.compression,
                            self.repository_path,
                            self.pack_reader,
                            parent.0,
                        )
                        .unwrap();

                        match parent_commit {
                            GitObject::Commit(pc) => self.commits.push(pc),
                            _ => panic!("Commit expected, got something else."),
                        };
                    }
                }
            }
        }

        None
    }
}

pub struct CommitsLifoIter<'a> {
    pack_reader: &'a PackReader,
    compression: &'a mut Decompression,
    repository_path: &'a Path,
    commits: Vec<Commit>,
    processed_commits: FxHashSet<CommitHash>,
}

impl<'a> CommitsLifoIter<'a> {
    pub fn create(
        repository_path: &'a Path,
        pack_reader: &'a PackReader,
        compression: &'a mut Decompression,
    ) -> CommitsLifoIter<'a> {
        let mut commits = Vec::new();
        let processed_commits = FxHashSet::default();

        let refs = GitRef::read_all(repository_path).unwrap();
        for r in refs {
            let commit = read_commit_from_ref(compression, repository_path, pack_reader, r);
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

        CommitsLifoIter {
            pack_reader,
            repository_path,
            commits,
            processed_commits,
            compression,
        }
    }
}

impl<'a> Iterator for CommitsLifoIter<'a> {
    type Item = Commit;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(commit) = self.commits.pop() {
            if self.processed_commits.insert(commit.hash().clone()) {
                for parent in commit.parents() {
                    if !self.processed_commits.contains(&parent) {
                        if let Some(parent_commit) = read_object_from_hash(
                            self.compression,
                            self.repository_path,
                            self.pack_reader,
                            parent.0,
                        ) {
                            match parent_commit {
                                GitObject::Commit(parent) => self.commits.push(parent),
                                _ => panic!("Expected a commit, but got something else"),
                            };
                        };
                    }
                }

                return Some(commit);
            }
        }

        None
    }
}

fn read_commit_from_ref(
    compression: &mut Decompression,
    repository_path: &Path,
    pack_reader: &PackReader,
    r: GitRef,
) -> Option<GitObject> {
    let hash = match r {
        GitRef::Simple(simple) => simple.hash,
        GitRef::Tag(tag) => tag.hash,
    };

    let hash: ObjectHash = hash.try_into().unwrap();
    let mut git_object =
        read_object_from_hash(compression, repository_path, pack_reader, hash).unwrap();
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

pub(crate) fn read_object_from_hash(
    compression: &mut Decompression,
    repository_path: &Path,
    pack_reader: &PackReader,
    hash: ObjectHash,
) -> Option<GitObject> {
    if let Some(obj) = pack_reader.read_git_object(compression, hash.clone()) {
        return Some(obj);
    }

    if let Ok(bytes) = compression.unpack_file(repository_path, &hash.to_string()) {
        if bytes.starts_with(b"commit ") {
            return Some(GitObject::Commit(Commit::create(
                Some(hash.into()),
                bytes,
                true,
            )));
        }

        if bytes.starts_with(b"tree ") {
            return Some(GitObject::Tree(Tree::create(hash.into(), bytes, true)));
        }

        if bytes.starts_with(b"tag ") {
            return Some(GitObject::Tag(Tag::create(hash.into(), bytes, true)));
        }

        if bytes.starts_with(b"blob ") {
            todo!("Not implemented yet")
            // return Some(GitObject::Blob(Blob::create(hash, bytes)));
        }

        dbg!(hash);
        dbg!(bytes.as_bstr());
        panic!("unknown loose git object type");
    }

    None
}
