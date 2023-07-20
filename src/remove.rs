use bstr::ByteSlice;

fn or<'a, T1, F1, F2>(f: F1, g: F2) -> Box<dyn Fn(T1, T1) -> bool + 'a>
where
    T1: Copy,
    F1: Fn(T1, T1) -> bool + 'a,
    F2: Fn(T1, T1) -> bool + 'a,
{
    Box::new(move |path, filename| f(path, filename) || g(path, filename))
}

fn single<'a, T1, F1>(f: F1) -> Box<dyn Fn(T1, T1) -> bool + 'a>
where
    T1: Copy,
    F1: Fn(T1, T1) -> bool + 'a,
{
    Box::new(f)
}

fn last_index_of(path: &[u8], needle: u8) -> Option<usize> {
    for (i, c) in path.iter().rev().enumerate() {
        if *c == needle {
            return Some(path.len() - i - 1);
        }
    }
    None
}

fn build_file_delete_patterns<'a>(
    files: &'a [String],
) -> Box<dyn Fn(&'a [u8], &'a [u8]) -> bool + 'a> {
    let mut delete_file = single(|_path: &[u8], _filename: &[u8]| false);
    for file in files.iter().map(|f| f.as_bytes()) {
        if file[0] == b'*' {
            match last_index_of(file, b'/') {
                // */bin/test.txt
                Some(last_slash) => {
                    delete_file = or(delete_file, move |path, filename| {
                        path.ends_with(&file[1..last_slash + 1])
                            && filename.eq(&file[last_slash + 1..])
                    })
                }
                // *mytest.txt
                None => {
                    delete_file = or(delete_file, |_path, filename| {
                        filename.ends_with(&file[1..])
                    })
                }
            }
        } else if file[file.len() - 1] == b'*' {
            match last_index_of(file, b'/') {
                // /some/folder/file_to_delete*
                Some(last_slash) => {
                    delete_file = or(delete_file, move |path, filename| {
                        path.eq(&file[0..last_slash + 1])
                            && filename.starts_with(&file[last_slash + 1..file.len() - 1])
                    })
                }
                // file_to_delete*
                None => {
                    delete_file = or(delete_file, move |_path, filename| {
                        filename.starts_with(&file[0..file.len() - 1])
                    })
                }
            }
        } else if file[0] == b'/' {
            // absolute path: /some/folder/file_to_delete.txt
            let last_slash = last_index_of(file, b'/').unwrap();
            delete_file = or(delete_file, move |path, filename| {
                path.len() + filename.len() == file.len()
                    && path.eq(&file[0..last_slash + 1])
                    && filename.eq(&file[last_slash + 1..])
            });
        } else {
            // simple file name, should not contain any slashes: file_to_delete.txt
            if last_index_of(file, b'/').is_some() {
                panic!("Unknown pattern: {}", file.as_bstr());
            }

            delete_file = or(delete_file, move |_path, filename| filename.eq(file));
        }
    }

    delete_file
}

pub fn remove(files: Vec<String>, _directories: Vec<String>) {
    let should_delete = build_file_delete_patterns(&files);
    should_delete(b"/", b"hello world");
}

#[cfg(test)]
mod test {

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
