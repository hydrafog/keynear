use super::*;

pub(crate) struct SendPlan {
    pub(crate) items: Vec<TransferItem>,
    pub(crate) total_bytes: u64,
    pub(crate) payload: OutboundPayload,
}

pub(crate) enum OutboundPayload {
    SingleFile(PathBuf),
    ArchiveEntries(Vec<ArchiveSourceEntry>),
}

pub(crate) struct ArchiveSourceEntry {
    pub(crate) archive_path: String,
    pub(crate) source_path: Option<PathBuf>,
    pub(crate) is_dir: bool,
}

pub(crate) fn prepare_send_plan(paths: &[PathBuf]) -> Result<SendPlan, KeynearError> {
    if paths.len() == 1 {
        if let Some(plan) = plan_single_file(&paths[0])? {
            return Ok(plan);
        }
    }

    let mut entries = Vec::new();
    let mut items = Vec::new();
    let mut total_bytes: u64 = 0;
    for path in paths {
        add_path_to_plan(path, &mut entries, &mut items, &mut total_bytes)?;
    }

    if items.is_empty() {
        return Err(KeynearError::Validation {
            field: "paths",
            message: "no transferable regular files or directories found".to_string(),
        });
    }

    Ok(SendPlan {
        items,
        total_bytes,
        payload: OutboundPayload::ArchiveEntries(entries),
    })
}

fn plan_single_file(path: &Path) -> Result<Option<SendPlan>, KeynearError> {
    let meta = fs::symlink_metadata(path)?;
    if meta.file_type().is_symlink() {
        return Err(KeynearError::Transfer(
            "symlinks cannot be transferred".to_string(),
        ));
    }
    if !meta.is_file() {
        return Ok(None);
    }
    let file_name =
        path.file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| KeynearError::Validation {
                field: "path",
                message: "source file has invalid filename".to_string(),
            })?;
    let clean_name = sanitize_relative_path(file_name)?;
    let size = meta.len();
    let item = TransferItem {
        name: clean_name.to_string_lossy().to_string(),
        mime_type: "application/octet-stream".to_string(),
        size_bytes: size,
        is_directory: false,
        target_path: None,
        checksum: None,
    };
    Ok(Some(SendPlan {
        items: vec![item],
        total_bytes: size,
        payload: OutboundPayload::SingleFile(path.to_path_buf()),
    }))
}

fn add_path_to_plan(
    path: &Path,
    entries: &mut Vec<ArchiveSourceEntry>,
    items: &mut Vec<TransferItem>,
    total_bytes: &mut u64,
) -> Result<(), KeynearError> {
    let meta = fs::symlink_metadata(path)?;
    if meta.file_type().is_symlink() {
        return Ok(());
    }
    let base_name =
        path.file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| KeynearError::Validation {
                field: "path",
                message: "source path has invalid filename".to_string(),
            })?;
    let base_rel = sanitize_relative_path(base_name)?;

    if meta.is_dir() {
        let mut dir_bytes: u64 = 0;
        collect_directory_entries(path, &base_rel, entries, &mut dir_bytes)?;
        *total_bytes = total_bytes
            .checked_add(dir_bytes)
            .ok_or_else(|| KeynearError::Transfer("transfer size overflow".to_string()))?;
        items.push(TransferItem {
            name: base_rel.to_string_lossy().to_string(),
            mime_type: "application/zip".to_string(),
            size_bytes: dir_bytes,
            is_directory: true,
            target_path: None,
            checksum: None,
        });
    } else if meta.is_file() {
        let size = meta.len();
        *total_bytes = total_bytes
            .checked_add(size)
            .ok_or_else(|| KeynearError::Transfer("transfer size overflow".to_string()))?;
        let rel_str = base_rel.to_string_lossy().replace('\\', "/");
        entries.push(ArchiveSourceEntry {
            archive_path: rel_str.clone(),
            source_path: Some(path.to_path_buf()),
            is_dir: false,
        });
        items.push(TransferItem {
            name: rel_str,
            mime_type: "application/octet-stream".to_string(),
            size_bytes: size,
            is_directory: false,
            target_path: None,
            checksum: None,
        });
    }
    Ok(())
}

fn collect_directory_entries(
    fs_dir: &Path,
    archive_rel: &Path,
    entries: &mut Vec<ArchiveSourceEntry>,
    dir_bytes: &mut u64,
) -> Result<(), KeynearError> {
    let rel_str = archive_rel.to_string_lossy().replace('\\', "/");
    entries.push(ArchiveSourceEntry {
        archive_path: rel_str,
        source_path: None,
        is_dir: true,
    });

    let mut children: Vec<fs::DirEntry> = fs::read_dir(fs_dir)?.collect::<Result<Vec<_>, _>>()?;
    children.sort_by_key(|e| e.file_name());

    for child in children {
        let child_path = child.path();
        let meta = fs::symlink_metadata(&child_path)?;
        if meta.file_type().is_symlink() {
            continue;
        }
        let child_name = child.file_name();
        let Some(name_str) = child_name.to_str() else {
            continue;
        };
        let clean_child = sanitize_relative_path(name_str)?;
        let next_rel = archive_rel.join(clean_child);
        if meta.is_dir() {
            collect_directory_entries(&child_path, &next_rel, entries, dir_bytes)?;
        } else if meta.is_file() {
            *dir_bytes = dir_bytes
                .checked_add(meta.len())
                .ok_or_else(|| KeynearError::Transfer("directory size overflow".to_string()))?;
            entries.push(ArchiveSourceEntry {
                archive_path: next_rel.to_string_lossy().replace('\\', "/"),
                source_path: Some(child_path),
                is_dir: false,
            });
        }
    }
    Ok(())
}

pub fn sanitize_relative_path(raw: &str) -> Result<PathBuf, KeynearError> {
    if raw.is_empty() || raw.contains('\0') || raw.contains('\\') {
        return Err(KeynearError::Validation {
            field: "path",
            message: "relative path contains forbidden characters".to_string(),
        });
    }
    if raw.starts_with('/') || raw.as_bytes().get(1) == Some(&b':') {
        return Err(KeynearError::Validation {
            field: "path",
            message: "absolute path is forbidden".to_string(),
        });
    }
    let mut out = PathBuf::new();
    for component in Path::new(raw).components() {
        match component {
            Component::Normal(seg) => {
                if seg.is_empty() || seg == "." || seg == ".." {
                    return Err(KeynearError::Validation {
                        field: "path",
                        message: "path traversal segment is forbidden".to_string(),
                    });
                }
                out.push(seg);
            }
            Component::CurDir
            | Component::ParentDir
            | Component::RootDir
            | Component::Prefix(_) => {
                return Err(KeynearError::Validation {
                    field: "path",
                    message: "path traversal or root component is forbidden".to_string(),
                });
            }
        }
    }
    if out.as_os_str().is_empty() {
        return Err(KeynearError::Validation {
            field: "path",
            message: "relative path must not be empty".to_string(),
        });
    }
    Ok(out)
}
