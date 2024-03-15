use core::panic;
use std::{
    borrow::Cow,
    cmp::Reverse,
    collections::{BinaryHeap, HashMap},
    hash::BuildHasher,
    ops::Deref,
    path::{Path, PathBuf},
    sync::{mpsc::channel, RwLock},
};

use bstr::ByteSlice;

use gitrwlib::{
    objs::{CommitBase, CommitEditable, CommitHash, Tree, TreeHash},
    Repository, WriteObject,
};
use rayon::prelude::*;
use regex::bytes::RegexSet;
use rustc_hash::FxHashMap;

macro_rules! b {
    ( $x:expr ) => {
        Box::new($x)
    };
}

fn last_index_of(path: &[u8], needle: u8) -> Option<usize> {
    for (i, c) in path.iter().rev().enumerate() {
        if *c == needle {
            return Some(path.len() - i - 1);
        }
    }
    None
}

type DynFn<'a> = Box<dyn Fn(&[u8]) -> bool + Sync + Send + 'a>;
type DynFn2<'a> = Box<dyn Fn(&[u8], &[u8]) -> bool + Sync + Send + 'a>;

fn build_folder_delete_patterns(folders: &[String]) -> DynFn {
    let mut delete_folder: DynFn = Box::new(|_path| false);

    for folder in folders.iter().map(|f| f.as_bytes()) {
        if folder[0] == b'*' {
            if folder[folder.len() - 1] == b'/' {
                delete_folder = b!(move |path| delete_folder(path) || path.ends_with(&folder[1..]));
            } else {
                // handles trailing slash
                delete_folder = b!(move |path| delete_folder(path)
                    || path[0..path.len() - 1].ends_with(&folder[1..]));
            }
        } else if folder[folder.len() - 1] == b'*' {
            delete_folder =
                b!(move |path| delete_folder(path)
                    || path.starts_with(&folder[0..folder.len() - 1]));
        } else if folder[0] == b'/' {
            // absolute path, no wildcard
            if folder[folder.len() - 1] == b'/' {
                delete_folder = b!(move |path| delete_folder(path) || path.eq(folder));
            } else {
                // handles missing trailing slash
                delete_folder = b!(move |path| delete_folder(path)
                    || path.len() == folder.len() + 1 && path[0..path.len() - 1].eq(folder));
            }
        } else {
            // relative path, no wildcard
            let mut folder: Vec<u8> = folder.to_owned();
            if folder[folder.len() - 1] != b'/' {
                folder.push(b'/');
            }
            if folder[0] != b'/' {
                folder.insert(0, b'/');
            }

            delete_folder = b!(move |path| delete_folder(path) || path.ends_with(&folder));
        }
    }

    delete_folder
}

fn build_regex_pattern(patterns: &[String]) -> DynFn2 {
    if patterns.is_empty() {
        return b!(|_, _| false);
    }

    let regexes = RegexSet::new(patterns).unwrap();
    b!(move |folder, file| {
        let path = [folder, file].concat();
        regexes.is_match(&path)
    })
}

fn build_file_delete_patterns(files: &[String]) -> DynFn2 {
    let mut delete_file: DynFn2 = b!(|_path, _filename| false);
    for file in files.iter().map(|f| f.as_bytes()) {
        if file[0] == b'*' {
            match last_index_of(file, b'/') {
                // */bin/test.txt
                Some(last_slash) => {
                    delete_file = b!(move |path, filename| delete_file(path, filename)
                        || (path.ends_with(&file[1..last_slash + 1])
                            && filename.eq(&file[last_slash + 1..])));
                }
                // *mytest.txt
                None => {
                    delete_file = b!(move |path, filename| delete_file(path, filename)
                        || filename.ends_with(&file[1..]));
                }
            }
        } else if file[file.len() - 1] == b'*' {
            match last_index_of(file, b'/') {
                // /some/folder/file_to_delete*
                Some(last_slash) => {
                    delete_file = b!(move |path, filename| delete_file(path, filename)
                        || (path.eq(&file[0..last_slash + 1])
                            && filename.starts_with(&file[last_slash + 1..file.len() - 1])));
                }
                // file_to_delete*
                None => {
                    delete_file = b!(move |path, filename| delete_file(path, filename)
                        || filename.starts_with(&file[0..file.len() - 1]));
                }
            }
        } else if file[0] == b'/' {
            // absolute path: /some/folder/file_to_delete.txt
            let last_slash = last_index_of(file, b'/').unwrap();
            delete_file = b!(move |path, filename| delete_file(path, filename)
                || (path.len() + filename.len() == file.len()
                    && path.eq(&file[0..last_slash + 1])
                    && filename.eq(&file[last_slash + 1..])));
        } else {
            // simple file name, should not contain any slashes: file_to_delete.txt
            if last_index_of(file, b'/').is_some() {
                panic!("Unknown pattern: {}", file.as_bstr());
            }

            delete_file =
                b!(move |path, filename| delete_file(path, filename) || filename.eq(file));
        }
    }

    delete_file
}

fn update_tree<T: BuildHasher + Sync + Send>(
    tree_hash: TreeHash,
    path: &[u8],
    repository: &mut Repository,
    should_delete_file: &DynFn2,
    should_delete_folder: &DynFn,
    should_remove: &DynFn2,
    rewritten_trees: &RwLock<HashMap<TreeHash, Option<TreeHash>, T>>,
    write_tree: &(impl Fn(Tree) + Sync + Send),
) -> Option<TreeHash> {
    if let Some(rewritten_hash_option) = rewritten_trees.read().unwrap().get(&tree_hash) {
        return rewritten_hash_option.clone();
    }

    let tree: Tree = match repository.read_object(tree_hash.into()).unwrap() {
        gitrwlib::objs::GitObject::Tree(tree) => tree,
        _ => panic!("Expected a tree, found something else"),
    };

    let old_hash = tree.hash();

    let mut filtered_lines = vec![];
    let mut tree_changed = false;
    for mut line in tree.lines() {
        if line.is_tree() {
            let full_path = [path, line.filename(), b"/"].concat();

            if should_delete_folder(&full_path) {
                tree_changed = true;
                continue;
            }

            if let Some(new_tree_hash) = update_tree(
                line.hash.deref().clone(),
                &full_path,
                repository,
                should_delete_file,
                should_delete_folder,
                should_remove,
                rewritten_trees,
                write_tree,
            ) {
                tree_changed = true;
                line.hash = Cow::Owned(new_tree_hash);
            }
        } else {
            if should_delete_file(path, line.filename()) {
                tree_changed = true;
                continue;
            }
            if should_remove(path, line.filename()) {
                tree_changed = true;
                continue;
            }
        }

        filtered_lines.push(line);
    }

    if !tree_changed {
        rewritten_trees
            .write()
            .unwrap()
            .insert(old_hash.clone(), None);
        None
    } else {
        let tree: Tree = filtered_lines.into_iter().collect();
        let new_hash = tree.hash().clone();
        rewritten_trees
            .write()
            .unwrap()
            .insert(old_hash.clone(), Some(new_hash.clone()));
        write_tree(tree);
        Some(new_hash)
    }
}

struct OrderedCommit {
    commit: CommitBase,
    index: usize,
}

impl Eq for OrderedCommit {}

impl PartialEq for OrderedCommit {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index
    }
}

impl PartialOrd for OrderedCommit {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.index.cmp(&other.index))
    }
}

impl Ord for OrderedCommit {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.index.cmp(&other.index)
    }
}

pub fn remove(
    repository_path: PathBuf,
    files: Vec<String>,
    directories: Vec<String>,
    regexes: Vec<String>,
    dry_run: bool,
) {
    let mut rewritten_commits: HashMap<CommitHash, CommitHash, _> = FxHashMap::default();
    let rewritten_trees: RwLock<HashMap<TreeHash, Option<TreeHash>, _>> =
        RwLock::new(FxHashMap::default());

    let mut repository = rayon::scope(|scope| {
        let (tx, rx) = channel::<OrderedCommit>();
        scope.spawn(|_| {
            let mut heap: BinaryHeap<Reverse<OrderedCommit>> = BinaryHeap::new();
            let mut commit_index = 0usize;
            for ordered_commit in rx.into_iter() {
                if ordered_commit.index == commit_index {
                    commit_index += 1;

                    let commit = CommitEditable::create(ordered_commit.commit);
                    let (old_hash, new_hash) = update_commit(
                        &repository_path,
                        commit,
                        &rewritten_commits,
                        &rewritten_trees,
                        dry_run,
                    );
                    if old_hash != new_hash {
                        rewritten_commits.insert(old_hash, new_hash);
                    }

                    while let Some(commit) = heap.pop() {
                        if commit.0.index == commit_index {
                            commit_index += 1;

                            let commit = CommitEditable::create(commit.0.commit);
                            let (old_hash, new_hash) = update_commit(
                                &repository_path,
                                commit,
                                &rewritten_commits,
                                &rewritten_trees,
                                dry_run,
                            );
                            if old_hash != new_hash {
                                rewritten_commits.insert(old_hash, new_hash);
                            }
                        } else {
                            heap.push(commit);
                            break;
                        }
                    }
                } else {
                    heap.push(Reverse(ordered_commit));
                }
            }
        });

        let repository = Repository::create(repository_path.clone());
        let file_delete_patterns = build_file_delete_patterns(&files);
        let folder_delete_patterns = build_folder_delete_patterns(&directories);
        let should_remove_line = build_regex_pattern(&regexes);
        repository
            .commits_topo()
            .enumerate()
            .map(|(index, commit)| OrderedCommit { index, commit })
            .par_bridge()
            .for_each_with(repository.clone(), |repository, commit| {
                let old_tree_hash = commit.commit.tree();
                update_tree(
                    old_tree_hash,
                    b"/",
                    repository,
                    &file_delete_patterns,
                    &folder_delete_patterns,
                    &should_remove_line,
                    &rewritten_trees,
                    &|tree| {
                        if !dry_run {
                            // TODO write out on different thread
                            Repository::write(repository_path.clone(), tree.into(), dry_run);
                        }
                    },
                );

                tx.send(commit).unwrap();
            });

        std::mem::drop(tx);

        repository
    });

    repository.update_refs(&rewritten_commits, dry_run);
    Repository::write_rewritten_commits_file(rewritten_commits, dry_run);
}

fn update_commit(
    repo_path: &Path,
    mut commit: CommitEditable,
    rewritten_commits: &HashMap<
        CommitHash,
        CommitHash,
        std::hash::BuildHasherDefault<rustc_hash::FxHasher>,
    >,
    rewritten_trees: &RwLock<
        HashMap<TreeHash, Option<TreeHash>, std::hash::BuildHasherDefault<rustc_hash::FxHasher>>,
    >,
    dry_run: bool,
) -> (CommitHash, CommitHash) {
    let old_hash = commit.base_hash().clone();

    update_parents(&mut commit, rewritten_commits);
    // update tree
    if let Some(Some(new_tree_hash)) = rewritten_trees.read().unwrap().get(&commit.tree()) {
        commit.set_tree(new_tree_hash.clone());
    }

    if commit.has_changes() {
        let write_object: WriteObject = commit.into();
        let new_hash = write_object.hash.clone();
        Repository::write(repo_path.into(), write_object, dry_run);
        return (old_hash, new_hash.into());
    }

    (old_hash.clone(), old_hash)
}

fn update_parents(
    commit: &mut CommitEditable,
    rewritten_commits: &HashMap<
        CommitHash,
        CommitHash,
        std::hash::BuildHasherDefault<rustc_hash::FxHasher>,
    >,
) {
    for (i, parent) in commit.parents().iter().enumerate() {
        if let Some(new_parent) = rewritten_commits.get(parent) {
            if new_parent != parent {
                commit.parents[i] = Some(new_parent.clone());
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::build_folder_delete_patterns;

    #[test]
    pub fn folder_deletion_patterns() {
        let patterns: Vec<String> = vec![
            "/some/folder".into(),
            "/another/folder/".into(),
            "*some_folder".into(),
            "*my/directory".into(),
            "/x/y*".into(),
            "bin/debug".into(),
            "foo/bar/".into(),
        ];

        let matches = build_folder_delete_patterns(&patterns);

        assert!(matches(b"/some/folder/"));
        assert!(matches(b"/another/folder/"));
        assert!(matches(b"/this/is_some_folder/"));
        assert!(matches(b"/this/is/some_folder/"));
        assert!(matches(b"/my/directory/"));
        assert!(matches(b"/_my/directory/"));
        assert!(matches(b"/x/y/"));
        assert!(matches(b"/x/y/z/"));
        assert!(matches(b"/src/bin/debug/"));
        assert!(matches(b"/bin/debug/"));
        assert!(matches(b"/baz/foo/bar/"));
        assert!(matches(b"/foo/bar/"));

        assert!(!matches(b"/_bin/debug/"));
        assert!(!matches(b"/bin/debug_/"));
        assert!(!matches(b"/a/some/folder/"));
        assert!(!matches(b"/this/is_some_folder/b/"));
        assert!(!matches(b"/my/directory/b/"));
    }

    #[test]
    pub fn file_deletion_patterns() {
        let patterns = vec![
            "/some/folder/removeme.txt".into(),
            "test.txt".into(),
            "*/bin/test_with_folder.txt".into(),
            "*test1.txt".into(),
            "/var/opt/myfile*".into(),
            "thisfile*".into(),
        ];
        let should_delete = super::build_file_delete_patterns(&patterns);

        assert!(should_delete(b"/some/folder/", b"removeme.txt"));
        assert!(!should_delete(b"/some/folder/", b"1removeme.txt"));
        assert!(!should_delete(b"/some/folder/", b"removeme.txt1"));
        assert!(!should_delete(b"/some/folder/", b"removeme.tx"));
        assert!(!should_delete(b"/some/folder_/", b"removeme.txt"));

        assert!(should_delete(b"/", b"test.txt"));
        assert!(should_delete(b"/hello/world/", b"test.txt"));

        assert!(should_delete(b"/test/bin/", b"test_with_folder.txt"));
        assert!(!should_delete(
            b"/test/bin/another_folder",
            b"test_with_folder.txt"
        ));

        assert!(should_delete(b"/some/folder/", b"test1.txt"));
        assert!(should_delete(b"/", b"test1.txt"));
        assert!(should_delete(b"/some/folder/", b"more_to_this_test1.txt"));
        assert!(should_delete(b"/", b"more_to_this_test1.txt"));

        assert!(should_delete(b"/var/opt/", b"myfile.txt"));
        assert!(should_delete(b"/var/opt/", b"myfile"));
        assert!(!should_delete(b"/var/opt/", b"_myfile.txt"));

        assert!(should_delete(b"/some/folder/", b"thisfile.txt"));
        assert!(should_delete(b"/another/folder/", b"thisfile.txt"));
        assert!(should_delete(b"/some/folder/", b"thisfile"));
        assert!(should_delete(b"/", b"thisfile"));

        assert!(!should_delete(b"/", b"_thisfile"));
        assert!(!should_delete(b"/", b"test.txt1"));
        assert!(!should_delete(b"/hello/world", b"1test.txt"));
    }
}
