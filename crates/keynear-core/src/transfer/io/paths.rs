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
