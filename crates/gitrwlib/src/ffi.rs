use core::slice;
use std::path::PathBuf;

use crate::{objs::CommitHash, Repository};
use bstr::ByteSlice;

use crate::{
    commits::{CommitsFifoIter, CommitsLifoIter},
    objs::Commit,
};

#[repr(C)]
pub struct FfiRepository<'a> {
    repository: Repository,
    commits_topo: Option<CommitsFifoIter<'a>>,
    commits_lifo: Option<CommitsLifoIter<'a>>,
}

#[repr(C)]
pub struct CommitFfi {
    commit: Commit,
}

#[no_mangle]
pub unsafe extern "C" fn repo_new(slice_ptr: &mut u8, len: u64) -> *mut FfiRepository<'static> {
    let x = slice::from_raw_parts(slice_ptr, len.try_into().unwrap());
    let mut path = PathBuf::new();
    path.push(x.as_bstr().to_os_str().unwrap());

    Box::into_raw(Box::new(FfiRepository {
        repository: Repository::create(path),
        commits_topo: None,
        commits_lifo: None,
    }))
}

#[no_mangle]
pub unsafe extern "C" fn repo_destroy(handle: *mut FfiRepository) {
    unsafe {
        let _ = Box::from_raw(handle);
    };
}

#[no_mangle]
pub unsafe extern "C" fn repo_commits_topo_init(handle: *mut FfiRepository) {
    let repo: &mut FfiRepository = unsafe { handle.as_mut().unwrap() };
    repo.commits_topo = Some(repo.repository.commits_topo());
}

#[no_mangle]
pub unsafe extern "C" fn repo_commits_lifo_init(handle: *mut FfiRepository) {
    let repo: &mut FfiRepository = unsafe { handle.as_mut().unwrap() };
    repo.commits_lifo = Some(repo.repository.commits_lifo());
}

#[no_mangle]
pub unsafe extern "C" fn repo_commits_topo_next(
    handle: *mut FfiRepository<'static>,
    commit_out: *mut *const CommitFfi,
) -> u8 {
    let repo = unsafe { handle.as_mut().unwrap() };
    let next = repo.commits_topo.as_mut().unwrap().next();

    if let Some(commit) = next {
        let result = Box::into_raw(Box::new(CommitFfi { commit }));
        unsafe { *commit_out = result };
        1
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn repo_commits_lifo_next(
    handle: *mut FfiRepository<'static>,
    commit_out: *mut *const CommitFfi,
) -> u8 {
    let repo = unsafe { handle.as_mut().unwrap() };
    let next = repo.commits_lifo.as_mut().unwrap().next();

    if let Some(commit) = next {
        let result = Box::into_raw(Box::new(CommitFfi { commit }));
        unsafe { *commit_out = result };
        1
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn commit_destroy(handle: *mut CommitFfi) {
    unsafe {
        let _ = Box::from_raw(handle);
    }
}

#[no_mangle]
pub unsafe extern "C" fn commit_author(handle: *const CommitFfi, len: *mut u32) -> *const u8 {
    let commit = &unsafe { handle.as_ref() }.unwrap().commit;
    unsafe { *len = commit.author_bytes().len().try_into().unwrap() };
    commit.author_bytes().as_ptr()
}

#[no_mangle]
pub unsafe extern "C" fn commit_committer(handle: *const CommitFfi, len: *mut u32) -> *const u8 {
    let commit = &unsafe { handle.as_ref() }.unwrap().commit;
    unsafe { *len = commit.committer_bytes().len().try_into().unwrap() };
    commit.committer_bytes().as_ptr()
}

#[no_mangle]
pub unsafe extern "C" fn commit_hash(handle: *const CommitFfi) -> *const [u8; 20] {
    let commit = &unsafe { handle.as_ref() }.unwrap().commit;

    let x: *const CommitHash = commit.hash();
    unsafe { std::mem::transmute(x) }
}
