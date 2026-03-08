use bl_core::value::Value;
use bl_runtime::fs::call_fs_builtin;

// ── file_exists tests ───────────────────────────────────────────

#[test]
fn test_file_exists_true() {
    // Cargo.toml always exists in the project
    let result = call_fs_builtin("file_exists", vec![Value::Str("Cargo.toml".into())]).unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_file_exists_false() {
    let result = call_fs_builtin(
        "file_exists",
        vec![Value::Str("nonexistent_file_xyz.txt".into())],
    )
    .unwrap();
    assert_eq!(result, Value::Bool(false));
}

#[test]
fn test_file_exists_nonexistent_path() {
    let result = call_fs_builtin(
        "file_exists",
        vec![Value::Str("/some/deeply/nested/nonexistent/path.txt".into())],
    )
    .unwrap();
    assert_eq!(result, Value::Bool(false));
}

// ── read_text / write_text tests ────────────────────────────────

#[test]
fn test_read_write_text() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.txt");
    let path_str = path.to_string_lossy().to_string();

    call_fs_builtin(
        "write_text",
        vec![Value::Str(path_str.clone()), Value::Str("hello world".into())],
    )
    .unwrap();

    let result = call_fs_builtin("read_text", vec![Value::Str(path_str)]).unwrap();
    assert_eq!(result, Value::Str("hello world".into()));
}

#[test]
fn test_read_text_empty_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("empty.txt");
    std::fs::write(&path, "").unwrap();
    let path_str = path.to_string_lossy().to_string();

    let result = call_fs_builtin("read_text", vec![Value::Str(path_str)]).unwrap();
    assert_eq!(result, Value::Str("".into()));
}

#[test]
fn test_read_text_not_found() {
    assert!(
        call_fs_builtin("read_text", vec![Value::Str("/nonexistent/path.txt".into())]).is_err()
    );
}

#[test]
fn test_write_text_nested_nonexistent_dir_error() {
    let result = call_fs_builtin(
        "write_text",
        vec![
            Value::Str("/nonexistent/deeply/nested/dir/file.txt".into()),
            Value::Str("content".into()),
        ],
    );
    assert!(result.is_err());
}

// ── list_dir tests ──────────────────────────────────────────────

#[test]
fn test_list_dir() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.txt"), "aaa").unwrap();
    std::fs::create_dir(dir.path().join("subdir")).unwrap();

    let result = call_fs_builtin(
        "list_dir",
        vec![Value::Str(dir.path().to_string_lossy().to_string())],
    )
    .unwrap();

    if let Value::List(items) = result {
        assert_eq!(items.len(), 2);
        for item in &items {
            if let Value::Record(rec) = item {
                assert!(rec.contains_key("name"));
                assert!(rec.contains_key("size"));
                assert!(rec.contains_key("is_dir"));
            } else {
                panic!("expected Record");
            }
        }
    } else {
        panic!("expected List");
    }
}

#[test]
fn test_list_dir_nonexistent_error() {
    let result = call_fs_builtin(
        "list_dir",
        vec![Value::Str("/nonexistent/directory/path".into())],
    );
    assert!(result.is_err());
}

// ── mkdir tests ─────────────────────────────────────────────────

#[test]
fn test_mkdir() {
    let dir = tempfile::tempdir().unwrap();
    let nested = dir.path().join("a").join("b").join("c");
    let path_str = nested.to_string_lossy().to_string();

    call_fs_builtin("mkdir", vec![Value::Str(path_str)]).unwrap();
    assert!(nested.is_dir());
}

#[test]
fn test_mkdir_already_exists() {
    let dir = tempfile::tempdir().unwrap();
    let sub = dir.path().join("existing");
    std::fs::create_dir(&sub).unwrap();
    let path_str = sub.to_string_lossy().to_string();

    // create_dir_all should succeed even if it already exists
    let result = call_fs_builtin("mkdir", vec![Value::Str(path_str)]);
    assert!(result.is_ok());
    assert!(sub.is_dir());
}

// ── basename tests ──────────────────────────────────────────────

#[test]
fn test_basename() {
    let result =
        call_fs_builtin("basename", vec![Value::Str("/home/user/data/sample.fastq".into())])
            .unwrap();
    assert_eq!(result, Value::Str("sample.fastq".into()));
}

#[test]
fn test_basename_no_parent() {
    let result = call_fs_builtin("basename", vec![Value::Str("file.txt".into())]).unwrap();
    assert_eq!(result, Value::Str("file.txt".into()));
}

#[test]
fn test_basename_root_path() {
    let result = call_fs_builtin("basename", vec![Value::Str("/".into())]).unwrap();
    // Root path has no file_name, returns empty
    assert_eq!(result, Value::Str("".into()));
}

// ── dirname tests ───────────────────────────────────────────────

#[test]
fn test_dirname() {
    let result =
        call_fs_builtin("dirname", vec![Value::Str("/home/user/data/sample.fastq".into())])
            .unwrap();
    assert_eq!(result, Value::Str("/home/user/data".into()));
}

#[test]
fn test_dirname_file_without_directory() {
    let result = call_fs_builtin("dirname", vec![Value::Str("file.txt".into())]).unwrap();
    // "file.txt" has no parent directory component, returns empty
    assert_eq!(result, Value::Str("".into()));
}

// ── extension tests ─────────────────────────────────────────────

#[test]
fn test_extension() {
    let result =
        call_fs_builtin("extension", vec![Value::Str("data/sample.fastq.gz".into())]).unwrap();
    assert_eq!(result, Value::Str("gz".into()));
}

#[test]
fn test_extension_no_extension() {
    let result = call_fs_builtin("extension", vec![Value::Str("Makefile".into())]).unwrap();
    assert_eq!(result, Value::Str("".into()));
}

// ── path_join tests ─────────────────────────────────────────────

#[test]
fn test_path_join() {
    let result = call_fs_builtin(
        "path_join",
        vec![Value::Str("/home/user".into()), Value::Str("data".into())],
    )
    .unwrap();
    if let Value::Str(s) = result {
        assert!(s.contains("data"));
    } else {
        panic!("expected Str");
    }
}

#[test]
fn test_path_join_absolute_second_arg() {
    let result = call_fs_builtin(
        "path_join",
        vec![
            Value::Str("/home/user".into()),
            Value::Str("/absolute/path".into()),
        ],
    )
    .unwrap();
    if let Value::Str(s) = result {
        // On Unix, joining with an absolute path replaces the base
        // On Windows, behavior differs. Just verify it produces a result.
        assert!(s.contains("absolute"));
    } else {
        panic!("expected Str");
    }
}

// ── glob tests ──────────────────────────────────────────────────

#[test]
fn test_glob_pattern() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.txt"), "").unwrap();
    std::fs::write(dir.path().join("b.txt"), "").unwrap();
    std::fs::write(dir.path().join("c.csv"), "").unwrap();

    let pattern = dir.path().join("*.txt").to_string_lossy().to_string();
    let result = call_fs_builtin("glob", vec![Value::Str(pattern)]).unwrap();
    if let Value::List(items) = result {
        assert_eq!(items.len(), 2);
    } else {
        panic!("expected List");
    }
}

#[test]
fn test_glob_no_match() {
    let dir = tempfile::tempdir().unwrap();
    let pattern = dir.path().join("*.zzz").to_string_lossy().to_string();
    let result = call_fs_builtin("glob", vec![Value::Str(pattern)]).unwrap();
    assert_eq!(result, Value::List(vec![]));
}

// ── read_lines / write_lines tests ──────────────────────────────

#[test]
fn test_read_write_lines() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("lines.txt");
    let path_str = path.to_string_lossy().to_string();

    let lines = vec![
        Value::Str("line1".into()),
        Value::Str("line2".into()),
        Value::Str("line3".into()),
    ];
    call_fs_builtin(
        "write_lines",
        vec![Value::Str(path_str.clone()), Value::List(lines)],
    )
    .unwrap();

    let result = call_fs_builtin("read_lines", vec![Value::Str(path_str)]).unwrap();
    if let Value::List(items) = result {
        assert_eq!(items.len(), 3);
        assert_eq!(items[0], Value::Str("line1".into()));
    } else {
        panic!("expected List");
    }
}

// ── append_text tests ───────────────────────────────────────────

#[test]
fn test_append_text() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("append.txt");
    let path_str = path.to_string_lossy().to_string();

    call_fs_builtin(
        "write_text",
        vec![Value::Str(path_str.clone()), Value::Str("hello".into())],
    )
    .unwrap();
    call_fs_builtin(
        "append_text",
        vec![Value::Str(path_str.clone()), Value::Str(" world".into())],
    )
    .unwrap();

    let result = call_fs_builtin("read_text", vec![Value::Str(path_str)]).unwrap();
    assert_eq!(result, Value::Str("hello world".into()));
}

// ── is_dir / is_file tests ──────────────────────────────────────

#[test]
fn test_is_dir_and_is_file() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.txt");
    std::fs::write(&file, "content").unwrap();

    let dir_str = dir.path().to_string_lossy().to_string();
    let file_str = file.to_string_lossy().to_string();

    assert_eq!(
        call_fs_builtin("is_dir", vec![Value::Str(dir_str.clone())]).unwrap(),
        Value::Bool(true)
    );
    assert_eq!(
        call_fs_builtin("is_file", vec![Value::Str(file_str.clone())]).unwrap(),
        Value::Bool(true)
    );
    assert_eq!(
        call_fs_builtin("is_dir", vec![Value::Str(file_str)]).unwrap(),
        Value::Bool(false)
    );
}

#[test]
fn test_is_file_on_directory_returns_false() {
    let dir = tempfile::tempdir().unwrap();
    let dir_str = dir.path().to_string_lossy().to_string();
    assert_eq!(
        call_fs_builtin("is_file", vec![Value::Str(dir_str)]).unwrap(),
        Value::Bool(false)
    );
}

#[test]
fn test_is_dir_on_file_returns_false() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("afile.txt");
    std::fs::write(&file, "data").unwrap();
    let file_str = file.to_string_lossy().to_string();
    assert_eq!(
        call_fs_builtin("is_dir", vec![Value::Str(file_str)]).unwrap(),
        Value::Bool(false)
    );
}

// ── file_size tests ─────────────────────────────────────────────

#[test]
fn test_file_size() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("sized.txt");
    std::fs::write(&file, "12345").unwrap();
    let path_str = file.to_string_lossy().to_string();

    let result = call_fs_builtin("file_size", vec![Value::Str(path_str)]).unwrap();
    assert_eq!(result, Value::Int(5));
}

#[test]
fn test_file_size_empty_file() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("empty.txt");
    std::fs::write(&file, "").unwrap();
    let path_str = file.to_string_lossy().to_string();

    let result = call_fs_builtin("file_size", vec![Value::Str(path_str)]).unwrap();
    assert_eq!(result, Value::Int(0));
}

// ── copy_file / remove tests ────────────────────────────────────

#[test]
fn test_copy_and_remove() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("src.txt");
    let dst = dir.path().join("dst.txt");
    std::fs::write(&src, "data").unwrap();

    let src_str = src.to_string_lossy().to_string();
    let dst_str = dst.to_string_lossy().to_string();

    call_fs_builtin(
        "copy_file",
        vec![Value::Str(src_str.clone()), Value::Str(dst_str.clone())],
    )
    .unwrap();
    assert!(dst.exists());

    call_fs_builtin("remove", vec![Value::Str(dst_str)]).unwrap();
    assert!(!dst.exists());
}

#[test]
fn test_copy_nonexistent_source_error() {
    let dir = tempfile::tempdir().unwrap();
    let dst = dir.path().join("dst.txt");
    let result = call_fs_builtin(
        "copy_file",
        vec![
            Value::Str("/nonexistent/source.txt".into()),
            Value::Str(dst.to_string_lossy().to_string()),
        ],
    );
    assert!(result.is_err());
}

#[test]
fn test_remove_nonexistent_error() {
    let result = call_fs_builtin("remove", vec![Value::Str("/nonexistent/file.txt".into())]);
    assert!(result.is_err());
}

// ── rename_file tests ───────────────────────────────────────────

#[test]
fn test_rename_file() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("old.txt");
    let dst = dir.path().join("new.txt");
    std::fs::write(&src, "data").unwrap();

    call_fs_builtin(
        "rename_file",
        vec![
            Value::Str(src.to_string_lossy().to_string()),
            Value::Str(dst.to_string_lossy().to_string()),
        ],
    )
    .unwrap();
    assert!(!src.exists());
    assert!(dst.exists());
}

#[test]
fn test_rename_nonexistent_source_error() {
    let dir = tempfile::tempdir().unwrap();
    let dst = dir.path().join("dst.txt");
    let result = call_fs_builtin(
        "rename_file",
        vec![
            Value::Str("/nonexistent/source.txt".into()),
            Value::Str(dst.to_string_lossy().to_string()),
        ],
    );
    assert!(result.is_err());
}

// ── temp_file / temp_dir tests ──────────────────────────────────

#[test]
fn test_temp_file_and_dir() {
    let result = call_fs_builtin("temp_file", vec![]).unwrap();
    if let Value::Str(path) = result {
        assert!(std::path::Path::new(&path).exists());
        std::fs::remove_file(&path).ok();
    } else {
        panic!("expected Str");
    }

    let result = call_fs_builtin("temp_dir", vec![]).unwrap();
    if let Value::Str(path) = result {
        assert!(std::path::Path::new(&path).is_dir());
        std::fs::remove_dir(&path).ok();
    } else {
        panic!("expected Str");
    }
}

#[test]
fn test_temp_dir_returns_existing_directory() {
    let result = call_fs_builtin("temp_dir", vec![]).unwrap();
    if let Value::Str(path) = result {
        let p = std::path::Path::new(&path);
        assert!(p.exists());
        assert!(p.is_dir());
        std::fs::remove_dir(&path).ok();
    } else {
        panic!("expected Str");
    }
}

// ── Wrong type errors ───────────────────────────────────────────

#[test]
fn test_file_exists_wrong_type() {
    let result = call_fs_builtin("file_exists", vec![Value::Int(42)]);
    assert!(result.is_err());
}

#[test]
fn test_read_text_wrong_type() {
    let result = call_fs_builtin("read_text", vec![Value::Int(42)]);
    assert!(result.is_err());
}

#[test]
fn test_write_text_wrong_type() {
    let result = call_fs_builtin("write_text", vec![Value::Int(42), Value::Str("x".into())]);
    assert!(result.is_err());
}

#[test]
fn test_list_dir_wrong_type() {
    let result = call_fs_builtin("list_dir", vec![Value::Int(42)]);
    assert!(result.is_err());
}

#[test]
fn test_unknown_fs_builtin() {
    let result = call_fs_builtin("nonexistent", vec![]);
    assert!(result.is_err());
}
