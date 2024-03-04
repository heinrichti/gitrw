use std::{borrow::Cow, collections::HashMap, hash::{BuildHasher, BuildHasherDefault}, ops::Deref, path::PathBuf, sync::mpsc::channel, thread::spawn};

use bstr::ByteSlice;
use libgitrw::{
    objs::{Commit, CommitHash, Tree, TreeHash},
    Repository,
};
use rustc_hash::{FxHashMap, FxHasher};

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

type DynFn<'a> = Box<dyn Fn(&'a [u8]) -> bool + 'a>;
type DynFn2<'a> = Box<dyn Fn(&[u8], &[u8]) -> bool + 'a>;

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

fn update_tree(
    tree_hash: TreeHash,
    path: &[u8],
    repository: &mut Repository,
    should_delete_file: &DynFn2,
    should_delete_folder: &DynFn,
    rewritten_trees: &mut HashMap<TreeHash, TreeHash, BuildHasherDefault<FxHasher>>,
) -> Option<TreeHash> {
    if should_delete_file(b"", b"") {}
    if should_delete_folder(b"") {}

    if let Some(rewritten_hash) = rewritten_trees.get(&tree_hash) {
        if *rewritten_hash == tree_hash {
            return None;
        }
        return Some(rewritten_hash.clone());
    }

    let tree: Tree = match repository.read_object(tree_hash.into()).unwrap() {
        libgitrw::objs::GitObject::Tree(tree) => tree,
        _ => panic!("Expected a tree, found something else"),
    };

    let old_hash = tree.hash();
    let tree: Tree = tree
        .lines()
        .filter(|line| !should_delete_file(path, line.filename()))
        .map(|mut line| {
            if line.is_tree() {
                if let Some(new_tree_hash) = update_tree(
                    line.hash.deref().clone(),
                    &[path, line.filename(), b"/"].concat(),
                    repository,
                    should_delete_file,
                    should_delete_folder,
                    rewritten_trees
                ) {            
                    line.hash = Cow::Owned(new_tree_hash);
                }
            }

            line
        })
        .collect();

    rewritten_trees.insert(old_hash.clone(), tree.hash().clone());

    if old_hash == tree.hash() {
        None
    } else {
        // TODO write out new tree
        Some(tree.hash().clone())
    }
}

pub fn remove(
    repository_path: PathBuf,
    files: Vec<String>,
    directories: Vec<String>,
    dry_run: bool,
) {
    let file_delete_patterns = build_file_delete_patterns(&files);
    let folder_delete_patterns = build_folder_delete_patterns(&directories);
    let mut rewritten_commits: HashMap<CommitHash, CommitHash, _> = FxHashMap::default();
    let mut rewritten_trees: HashMap<TreeHash, TreeHash, _> = FxHashMap::default();

    let (tx, rx) = channel();
    let write_path = repository_path.clone();

    let write_thread =
        spawn(move || Repository::write_commits(write_path, rx.into_iter(), dry_run));

    let mut repository = Repository::create(repository_path);
    for mut commit in repository.clone().commits_topo() {
        let old_hash = commit.hash().clone();

        update_parents(&mut commit, &rewritten_commits);

        // update tree
        // TODO write out new trees!
        if let Some(new_tree_hash) = update_tree(
            commit.tree(),
            b"/",
            &mut repository,
            &file_delete_patterns,
            &folder_delete_patterns,
            &mut rewritten_trees
        ) {
            commit.set_tree(new_tree_hash);
        }
        
        // write out changes if any
        if commit.has_changes() {
            let commit = Commit::create(None, commit.to_bytes(), false);
            rewritten_commits.insert(old_hash, commit.hash().clone());
            tx.send(commit).unwrap();    
        }
    }

    std::mem::drop(tx);
    write_thread.join().unwrap();

    if !dry_run {
        repository.update_refs(&rewritten_commits);
    }
}

fn update_parents(commit: &mut Commit, rewritten_commits: &HashMap<CommitHash, CommitHash, std::hash::BuildHasherDefault<rustc_hash::FxHasher>>) {
    for (i, parent) in commit.parents().iter().enumerate() {
        if let Some(new_parent) = rewritten_commits.get(parent) {
            if new_parent != parent {
                commit.set_parent(i, new_parent.clone());
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