use super::*;

pub(crate) fn expand_tilde(path: &Path) -> PathBuf {
    let raw = path.to_string_lossy();
    if let Some(rest) = raw.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join(rest);
        }
    }
    path.to_path_buf()
}

pub(super) fn unique_target_path(base_dir: &Path, rel: &Path, is_dir: bool) -> PathBuf {
    let candidate = base_dir.join(rel);
    if !candidate.exists() {
        return candidate;
    }
    let (stem, ext) = if is_dir {
        let name = candidate
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("folder");
        (name, None)
    } else {
        let stem = candidate
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("file");
        let ext = candidate.extension().and_then(|s| s.to_str());
        (stem, ext)
    };
    let parent = candidate.parent().unwrap_or(base_dir);
    for idx in 1..10_000 {
        let name = match ext {
            Some(e) => format!("{stem} ({idx}).{e}"),
            None => format!("{stem} ({idx})"),
        };
        let next = parent.join(name);
        if !next.exists() {
            return next;
        }
    }
    parent.join(format!("{stem}-{}", Uuid::new_v4()))
}

pub(super) fn unique_temp_path(base_dir: &Path, transfer_id: &str) -> PathBuf {
    let seq = TEMP_SEQ.fetch_add(1, Ordering::Relaxed);
    base_dir.join(format!(
        ".keynear-part-{transfer_id}-{}-{seq}.tmp",
        std::process::id()
    ))
}

pub(super) fn open_private_new_file(path: &Path) -> std::io::Result<File> {
    let mut opts = OpenOptions::new();
    opts.create_new(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    opts.open(path)
}

pub(super) fn zip_err(err: zip::result::ZipError) -> KeynearError {
    KeynearError::Transfer(format!("zip stream error: {err}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::MetadataExt;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!("keynear-paths-{label}-{nanos}"))
    }

    #[test]
    fn expand_tilde_replaces_only_the_home_prefix() {
        let relative = Path::new("relative/file.txt");
        assert_eq!(expand_tilde(relative), relative);
        let absolute = Path::new("/var/tmp/file.txt");
        assert_eq!(expand_tilde(absolute), absolute);
        let tilde = Path::new("~/notes.txt");
        match std::env::var("HOME") {
            Ok(home) => assert_eq!(expand_tilde(tilde), PathBuf::from(home).join("notes.txt")),
            Err(_) => assert_eq!(expand_tilde(tilde), tilde),
        }
    }

    #[test]
    fn unique_target_path_keeps_the_requested_name_when_free() {
        let dir = temp_dir("free");
        fs::create_dir_all(&dir).expect("create dir");
        let rel = Path::new("notes.txt");
        assert_eq!(unique_target_path(&dir, rel, false), dir.join(rel));
        fs::remove_dir_all(&dir).expect("remove dir");
    }

    #[test]
    fn unique_target_path_adds_numbered_suffixes_on_file_collisions() {
        let dir = temp_dir("collision-file");
        fs::create_dir_all(&dir).expect("create dir");
        let rel = Path::new("notes.txt");
        fs::write(dir.join(rel), b"first").expect("write original");
        let first = unique_target_path(&dir, rel, false);
        assert_eq!(first, dir.join("notes (1).txt"));
        fs::write(&first, b"second").expect("write first copy");
        assert_eq!(
            unique_target_path(&dir, rel, false),
            dir.join("notes (2).txt")
        );
        fs::remove_dir_all(&dir).expect("remove dir");
    }

    #[test]
    fn unique_target_path_numbers_directory_collisions_without_an_extension() {
        let dir = temp_dir("collision-dir");
        fs::create_dir_all(dir.join("archive.tar.gz")).expect("create dir");
        let rel = Path::new("archive.tar.gz");
        assert_eq!(
            unique_target_path(&dir, rel, true),
            dir.join("archive.tar.gz (1)")
        );
        fs::remove_dir_all(&dir).expect("remove dir");
    }

    #[test]
    fn unique_target_path_falls_back_to_a_uuid_after_exhaustion() {
        let dir = temp_dir("exhaustion");
        fs::create_dir_all(&dir).expect("create dir");
        let rel = Path::new("payload.bin");
        fs::write(dir.join("payload.bin"), b"original").expect("write original");
        for idx in 1..10_000 {
            let name = format!("payload ({idx}).bin");
            fs::write(dir.join(name), b"taken").expect("write collision");
        }
        let fallback = unique_target_path(&dir, rel, false);
        assert_eq!(fallback.parent(), Some(dir.as_path()));
        let name = fallback.file_name().expect("file name").to_string_lossy();
        assert!(name.starts_with("payload-") && !name.ends_with(".bin"));
        assert!(!fallback.exists());
        fs::remove_dir_all(&dir).expect("remove dir");
    }

    #[test]
    fn unique_temp_path_is_scoped_and_distinct_per_call() {
        let dir = temp_dir("temp");
        let first = unique_temp_path(&dir, "tx-1");
        let second = unique_temp_path(&dir, "tx-1");
        assert_eq!(first.parent(), Some(dir.as_path()));
        assert_ne!(first, second);
        let name = first.file_name().expect("file name").to_string_lossy();
        assert!(name.starts_with(".keynear-part-tx-1-"));
        assert!(name.ends_with(".tmp"));
    }

    #[test]
    fn open_private_new_file_creates_once_with_owner_only_mode() {
        let dir = temp_dir("private");
        fs::create_dir_all(&dir).expect("create dir");
        let path = dir.join("secret.bin");
        let mut file = open_private_new_file(&path).expect("create private file");
        write!(file, "payload").expect("write private file");
        drop(file);
        assert_eq!(fs::read(&path).expect("read private file"), b"payload");
        assert!(open_private_new_file(&path).is_err());
        #[cfg(unix)]
        assert_eq!(fs::metadata(&path).expect("metadata").mode() & 0o777, 0o600);
        fs::remove_dir_all(&dir).expect("remove dir");
    }

    #[test]
    fn zip_err_maps_a_zip_error_to_a_transfer_error() {
        let err = ZipArchive::new(std::io::Cursor::new(b"not a zip file")).expect_err("bad zip");
        let mapped = zip_err(err);
        match mapped {
            KeynearError::Transfer(message) => {
                assert!(message.starts_with("zip stream error: "));
            }
            other => panic!("expected a transfer error, got {other:?}"),
        }
    }
}
