use super::*;

fn rejection_message(raw: &str) -> String {
    match sanitize_relative_path(raw) {
        Err(KeynearError::Validation { field, message }) => {
            assert_eq!(field, "path");
            message
        }
        Err(other) => panic!("expected a path validation error, got {other:?}"),
        Ok(path) => panic!("expected {raw:?} to be rejected, got {}", path.display()),
    }
}

#[test]
fn sanitize_relative_path_accepts_normal_nested_and_unicode_paths() {
    assert_eq!(
        sanitize_relative_path("docs/notes.txt").expect("nested path"),
        PathBuf::from("docs/notes.txt")
    );
    assert_eq!(
        sanitize_relative_path("a//b").expect("repeated separators"),
        PathBuf::from("a/b")
    );
    assert_eq!(
        sanitize_relative_path("a/./b.txt").expect("interior dot segment"),
        PathBuf::from("a/b.txt")
    );
    assert_eq!(
        sanitize_relative_path("dir/").expect("trailing separator"),
        PathBuf::from("dir")
    );
    assert_eq!(
        sanitize_relative_path("файлы/данные.txt").expect("unicode path"),
        PathBuf::from("файлы/данные.txt")
    );
}

#[test]
fn sanitize_relative_path_rejects_empty_nul_and_backslash_inputs() {
    for raw in ["", "a\0b", "dir\\file.txt", "\\\\server\\share"] {
        assert_eq!(
            rejection_message(raw),
            "relative path contains forbidden characters"
        );
    }
}

#[test]
fn sanitize_relative_path_rejects_unix_and_windows_absolute_paths() {
    for raw in ["/etc/passwd", "/", "C:", "C:/secret.txt"] {
        assert_eq!(rejection_message(raw), "absolute path is forbidden");
    }
}

#[test]
fn sanitize_relative_path_rejects_traversal_components() {
    for raw in [".", "..", "./a.txt", "a/../b.txt"] {
        assert_eq!(
            rejection_message(raw),
            "path traversal or root component is forbidden"
        );
    }
}
