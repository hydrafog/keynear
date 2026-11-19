use std::fs::{self, File, OpenOptions};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use iroh::endpoint::{Connection, RecvStream};
use sha2::{Digest, Sha256};
use tokio::sync::mpsc;
use uuid::Uuid;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use keynear_protocol::{
    v1, write_control_envelope, write_stream_kind, KeynearError, QuicStreamKind, TransferItem,
    STREAM_CHUNK_BYTES, ZIP_IO_BUFFER_BYTES,
};

use super::framing::{
    await_transfer_consent, read_data_chunk, send_cancel_frame, send_completion_and_wait_ack,
    verify_completion_envelope, wrap_payload, write_data_chunk, ChannelChunkWriter,
    PIPE_CHANNEL_SLOTS,
};
use super::state::TransferRegistry;

const MAX_FILES_IN_ARCHIVE: usize = 10_000;
const MAX_ZIP_STREAM_OVERHEAD_BYTES: u64 = (MAX_FILES_IN_ARCHIVE as u64) * 512 + 65_536;
static TEMP_SEQ: AtomicU64 = AtomicU64::new(0);

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
        let path = &paths[0];
        let meta = fs::symlink_metadata(path)?;
        if meta.file_type().is_symlink() {
            return Err(KeynearError::Transfer(
                "symlinks cannot be transferred".to_string(),
            ));
        }
        if meta.is_file() {
            let file_name = path.file_name().and_then(|s| s.to_str()).ok_or_else(|| {
                KeynearError::Validation {
                    field: "path",
                    message: "source file has invalid filename".to_string(),
                }
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
            return Ok(SendPlan {
                items: vec![item],
                total_bytes: size,
                payload: OutboundPayload::SingleFile(path.clone()),
            });
        }
    }

    let mut entries = Vec::new();
    let mut items = Vec::new();
    let mut total_bytes: u64 = 0;

    for path in paths {
        let meta = fs::symlink_metadata(path)?;
        if meta.file_type().is_symlink() {
            continue;
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
            collect_directory_entries(path, &base_rel, &mut entries, &mut dir_bytes)?;
            total_bytes = total_bytes
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
            total_bytes = total_bytes
                .checked_add(size)
                .ok_or_else(|| KeynearError::Transfer("transfer size overflow".to_string()))?;
            let rel_str = base_rel.to_string_lossy().replace('\\', "/");
            entries.push(ArchiveSourceEntry {
                archive_path: rel_str.clone(),
                source_path: Some(path.clone()),
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

pub(crate) async fn send_file_handle_stream(
    registry: &TransferRegistry,
    conn: &Connection,
    offer: &v1::TransferOffer,
    file: File,
    cancel_flag: &Arc<AtomicBool>,
) -> Result<String, KeynearError> {
    let (mut send, mut recv) = conn
        .open_bi()
        .await
        .map_err(|err| KeynearError::Transport(format!("open transfer stream failed: {err}")))?;

    write_stream_kind(&mut send, QuicStreamKind::RawFileChunk).await?;
    let offer_env = wrap_payload(v1::envelope::Payload::TransferOffer(offer.clone()));
    write_control_envelope(&mut send, &offer_env).await?;

    await_transfer_consent(&mut recv, &offer.transfer_id).await?;

    let (chunk_tx, mut chunk_rx) =
        mpsc::channel::<Result<Vec<u8>, KeynearError>>(PIPE_CHANNEL_SLOTS);
    let cancel_reader = Arc::clone(cancel_flag);
    let reader_task = tokio::task::spawn_blocking(move || {
        let mut reader = BufReader::with_capacity(ZIP_IO_BUFFER_BYTES, file);
        let mut buf = vec![0u8; STREAM_CHUNK_BYTES];
        loop {
            if cancel_reader.load(Ordering::Acquire) {
                let _ = chunk_tx.blocking_send(Err(KeynearError::TransferCancelled(
                    "cancelled by sender".to_string(),
                )));
                break;
            }
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    if chunk_tx.blocking_send(Ok(buf[..n].to_vec())).is_err() {
                        break;
                    }
                }
                Err(err) => {
                    let _ = chunk_tx.blocking_send(Err(KeynearError::Io(err)));
                    break;
                }
            }
        }
    });

    let mut hasher = Sha256::new();
    let mut bytes_sent: u64 = 0;
    while let Some(item) = chunk_rx.recv().await {
        if cancel_flag.load(Ordering::Acquire) {
            let _ = send_cancel_frame(&mut send, &offer.transfer_id, "cancelled_by_sender").await;
            return Err(KeynearError::TransferCancelled(
                "cancelled by sender".to_string(),
            ));
        }
        let chunk = item?;
        hasher.update(&chunk);
        write_data_chunk(&mut send, &chunk).await?;
        bytes_sent = bytes_sent.saturating_add(chunk.len() as u64);
        registry.update_progress(&offer.transfer_id, bytes_sent);
    }

    reader_task
        .await
        .map_err(|err| KeynearError::Transfer(format!("file reader task failed: {err}")))?;

    let digest: [u8; 32] = hasher.finalize().into();
    send_completion_and_wait_ack(
        &mut send,
        &mut recv,
        &offer.transfer_id,
        bytes_sent,
        &digest,
    )
    .await?;

    Ok(hex::encode(digest))
}

pub(crate) async fn send_zip_stream(
    registry: &TransferRegistry,
    conn: &Connection,
    offer: &v1::TransferOffer,
    entries: Vec<ArchiveSourceEntry>,
    cancel_flag: &Arc<AtomicBool>,
) -> Result<String, KeynearError> {
    let (mut send, mut recv) = conn
        .open_bi()
        .await
        .map_err(|err| KeynearError::Transport(format!("open zip stream failed: {err}")))?;

    write_stream_kind(&mut send, QuicStreamKind::StoredZipArchive).await?;
    let offer_env = wrap_payload(v1::envelope::Payload::TransferOffer(offer.clone()));
    write_control_envelope(&mut send, &offer_env).await?;

    await_transfer_consent(&mut recv, &offer.transfer_id).await?;

    let (chunk_tx, mut chunk_rx) =
        mpsc::channel::<Result<Vec<u8>, KeynearError>>(PIPE_CHANNEL_SLOTS);
    let cancel_writer = Arc::clone(cancel_flag);
    let writer_task = tokio::task::spawn_blocking(move || -> Result<(), KeynearError> {
        let channel_writer = ChannelChunkWriter::new(chunk_tx.clone(), cancel_writer);
        let buffered = BufWriter::with_capacity(ZIP_IO_BUFFER_BYTES, channel_writer);
        let mut zip = ZipWriter::new_stream(buffered);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);

        for entry in entries {
            if entry.is_dir {
                let dir_name = if entry.archive_path.ends_with('/') {
                    entry.archive_path
                } else {
                    format!("{}/", entry.archive_path)
                };
                zip.add_directory(dir_name, options).map_err(zip_err)?;
                continue;
            }
            let Some(src_path) = entry.source_path else {
                continue;
            };
            let meta = fs::symlink_metadata(&src_path)?;
            if meta.file_type().is_symlink() || !meta.is_file() {
                continue;
            }
            zip.start_file(&entry.archive_path, options)
                .map_err(zip_err)?;
            let file = File::open(&src_path)?;
            let mut reader = BufReader::with_capacity(ZIP_IO_BUFFER_BYTES, file);
            let mut copy_buf = vec![0u8; STREAM_CHUNK_BYTES];
            loop {
                let n = reader.read(&mut copy_buf)?;
                if n == 0 {
                    break;
                }
                zip.write_all(&copy_buf[..n])?;
            }
        }

        let stream_writer = zip.finish().map_err(zip_err)?;
        let mut buffered = stream_writer.into_inner();
        buffered.flush()?;
        Ok(())
    });

    let mut hasher = Sha256::new();
    let mut bytes_sent: u64 = 0;
    while let Some(item) = chunk_rx.recv().await {
        if cancel_flag.load(Ordering::Acquire) {
            let _ = send_cancel_frame(&mut send, &offer.transfer_id, "cancelled_by_sender").await;
            return Err(KeynearError::TransferCancelled(
                "cancelled by sender".to_string(),
            ));
        }
        let chunk = item?;
        hasher.update(&chunk);
        write_data_chunk(&mut send, &chunk).await?;
        bytes_sent = bytes_sent.saturating_add(chunk.len() as u64);
        registry.update_progress(&offer.transfer_id, bytes_sent);
    }

    let writer_res = writer_task
        .await
        .map_err(|err| KeynearError::Transfer(format!("zip writer task failed: {err}")))?;
    writer_res?;

    let digest: [u8; 32] = hasher.finalize().into();
    send_completion_and_wait_ack(
        &mut send,
        &mut recv,
        &offer.transfer_id,
        bytes_sent,
        &digest,
    )
    .await?;

    Ok(hex::encode(digest))
}

pub(crate) async fn receive_raw_file_stream(
    registry: &TransferRegistry,
    incoming_dir: PathBuf,
    offer: &v1::TransferOffer,
    recv: &mut RecvStream,
    explicit_file: Option<File>,
    cancel_flag: &Arc<AtomicBool>,
) -> Result<(u64, Vec<String>, String), KeynearError> {
    let rel_name = sanitize_relative_path(&offer.items[0].name)?;
    let (dest_dir, tmp_path, final_path, out_file) = if let Some(file) = explicit_file {
        (None, None, None, file)
    } else {
        fs::create_dir_all(&incoming_dir)?;
        let final_target = unique_target_path(&incoming_dir, &rel_name);
        let tmp = unique_temp_path(&incoming_dir, &offer.transfer_id);
        if let Some(parent) = tmp.parent() {
            fs::create_dir_all(parent)?;
        }
        let file = open_private_new_file(&tmp)?;
        (Some(incoming_dir), Some(tmp), Some(final_target), file)
    };

    let (chunk_tx, mut chunk_rx) = mpsc::channel::<Vec<u8>>(PIPE_CHANNEL_SLOTS);
    let writer_task = tokio::task::spawn_blocking(move || -> Result<(), KeynearError> {
        let mut writer = BufWriter::with_capacity(ZIP_IO_BUFFER_BYTES, out_file);
        while let Some(chunk) = chunk_rx.blocking_recv() {
            writer.write_all(&chunk)?;
        }
        writer.flush()?;
        let inner = writer
            .into_inner()
            .map_err(|err| KeynearError::Io(err.into_error()))?;
        let _ = inner.sync_all();
        Ok(())
    });

    let stream_result: Result<(u64, [u8; 32]), KeynearError> = async {
        let mut hasher = Sha256::new();
        let mut total_received: u64 = 0;

        while let Some(chunk) = read_data_chunk(recv).await? {
            if cancel_flag.load(Ordering::Acquire) {
                return Err(KeynearError::TransferCancelled(
                    "cancelled by receiver".to_string(),
                ));
            }
            let next_total = total_received
                .checked_add(chunk.len() as u64)
                .ok_or_else(|| KeynearError::Transfer("byte count overflow".to_string()))?;
            if next_total > offer.total_bytes {
                return Err(KeynearError::Transfer(format!(
                    "received bytes {next_total} exceed TransferOffer.total_bytes {}",
                    offer.total_bytes
                )));
            }
            hasher.update(&chunk);
            if chunk_tx.send(chunk).await.is_err() {
                return Err(KeynearError::Transfer(
                    "disk writer task terminated unexpectedly".to_string(),
                ));
            }
            total_received = next_total;
            registry.update_progress(&offer.transfer_id, total_received);
        }

        drop(chunk_tx);
        let write_res = writer_task
            .await
            .map_err(|err| KeynearError::Transfer(format!("disk writer join failed: {err}")))?;
        write_res?;

        let digest: [u8; 32] = hasher.finalize().into();
        verify_completion_envelope(recv, &offer.transfer_id, total_received, &digest).await?;
        Ok((total_received, digest))
    }
    .await;

    match stream_result {
        Ok((total_received, digest)) => {
            let target_paths = if let (Some(tmp), Some(final_target)) = (tmp_path, final_path) {
                if let Some(parent) = final_target.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::rename(&tmp, &final_target)?;
                vec![final_target.to_string_lossy().to_string()]
            } else {
                vec!["mediastore://fd".to_string()]
            };
            let _ = dest_dir;
            Ok((total_received, target_paths, hex::encode(digest)))
        }
        Err(err) => {
            if let Some(tmp) = tmp_path {
                let _ = fs::remove_file(tmp);
            }
            Err(err)
        }
    }
}

pub(crate) async fn receive_zip_stream(
    registry: &TransferRegistry,
    dir: PathBuf,
    offer: &v1::TransferOffer,
    recv: &mut RecvStream,
    cancel_flag: &Arc<AtomicBool>,
) -> Result<(u64, Vec<String>, String), KeynearError> {
    fs::create_dir_all(&dir)?;
    let staging_dir = dir.join(format!(
        ".keynear-staging-{}-{}",
        offer.transfer_id,
        TEMP_SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&staging_dir)?;

    let archive_tmp_path = staging_dir.join(".archive.zip.tmp");
    let max_wire_bytes = offer
        .total_bytes
        .saturating_add(MAX_ZIP_STREAM_OVERHEAD_BYTES);
    let max_extracted = offer.total_bytes;
    let staging_clone = staging_dir.clone();
    let archive_path_clone = archive_tmp_path.clone();

    let stream_result: Result<(u64, [u8; 32], Vec<PathBuf>), KeynearError> = async {
        let mut archive_writer = BufWriter::with_capacity(
            ZIP_IO_BUFFER_BYTES,
            open_private_new_file(&archive_tmp_path)?,
        );
        let mut hasher = Sha256::new();
        let mut wire_bytes: u64 = 0;

        while let Some(chunk) = read_data_chunk(recv).await? {
            if cancel_flag.load(Ordering::Acquire) {
                return Err(KeynearError::TransferCancelled(
                    "cancelled by receiver".to_string(),
                ));
            }
            wire_bytes = wire_bytes
                .checked_add(chunk.len() as u64)
                .ok_or_else(|| KeynearError::Transfer("wire byte overflow".to_string()))?;
            if wire_bytes > max_wire_bytes {
                return Err(KeynearError::Transfer(
                    "zip stream exceeded permitted framing overhead".to_string(),
                ));
            }
            hasher.update(&chunk);
            archive_writer.write_all(&chunk)?;
            registry.update_progress(&offer.transfer_id, wire_bytes.min(offer.total_bytes));
        }
        archive_writer.flush()?;
        drop(archive_writer);

        let digest: [u8; 32] = hasher.finalize().into();
        verify_completion_envelope(recv, &offer.transfer_id, wire_bytes, &digest).await?;

        let extracted_paths =
            tokio::task::spawn_blocking(move || -> Result<Vec<PathBuf>, KeynearError> {
                let archive_file = File::open(&archive_path_clone)?;
                let reader = BufReader::with_capacity(ZIP_IO_BUFFER_BYTES, archive_file);
                let mut archive = ZipArchive::new(reader).map_err(zip_err)?;
                if archive.len() > MAX_FILES_IN_ARCHIVE {
                    return Err(KeynearError::Transfer(
                        "archive exceeds maximum file entry count".to_string(),
                    ));
                }

                let mut extracted_bytes: u64 = 0;
                let mut extracted_rel_paths = Vec::new();
                let mut buf = vec![0u8; STREAM_CHUNK_BYTES];

                for idx in 0..archive.len() {
                    let mut entry = archive.by_index(idx).map_err(zip_err)?;
                    if entry.encrypted() {
                        return Err(KeynearError::Transfer(
                            "encrypted zip entries are not allowed".to_string(),
                        ));
                    }
                    if entry.is_symlink() {
                        return Err(KeynearError::Transfer(
                            "symlink zip entries are not allowed".to_string(),
                        ));
                    }
                    if entry.compression() != CompressionMethod::Stored {
                        return Err(KeynearError::Transfer(format!(
                            "unsupported zip compression method {:?}; only STORED is permitted",
                            entry.compression()
                        )));
                    }
                    let Some(enclosed) = entry.enclosed_name() else {
                        return Err(KeynearError::Transfer(
                            "zip entry contains invalid or escaping path".to_string(),
                        ));
                    };
                    let safe_rel = sanitize_relative_path(&enclosed.to_string_lossy())?;
                    if entry.is_dir() {
                        fs::create_dir_all(staging_clone.join(&safe_rel))?;
                        continue;
                    }
                    if entry.compressed_size() != entry.size() {
                        return Err(KeynearError::Transfer(
                            "STORED zip entry has mismatched compressed and uncompressed size"
                                .to_string(),
                        ));
                    }
                    let target = staging_clone.join(&safe_rel);
                    if let Some(parent) = target.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    let out_file = open_private_new_file(&target)?;
                    let mut writer = BufWriter::with_capacity(ZIP_IO_BUFFER_BYTES, out_file);
                    loop {
                        let n = entry.read(&mut buf)?;
                        if n == 0 {
                            break;
                        }
                        extracted_bytes = extracted_bytes
                            .checked_add(n as u64)
                            .ok_or_else(|| {
                                KeynearError::Transfer("extracted byte overflow".to_string())
                            })?;
                        if extracted_bytes > max_extracted {
                            return Err(KeynearError::Transfer(format!(
                                "extracted bytes {extracted_bytes} exceed TransferOffer.total_bytes {max_extracted}"
                            )));
                        }
                        writer.write_all(&buf[..n])?;
                    }
                    writer.flush()?;
                    extracted_rel_paths.push(safe_rel);
                }

                Ok(extracted_rel_paths)
            })
            .await
            .map_err(|err| KeynearError::Transfer(format!("zip extract task failed: {err}")))??;

        let _ = fs::remove_file(&archive_tmp_path);
        Ok((wire_bytes, digest, extracted_paths))
    }
    .await;

    match stream_result {
        Ok((wire_bytes, digest, extracted_paths)) => {
            let mut promoted = Vec::with_capacity(extracted_paths.len());
            for rel in extracted_paths {
                let src = staging_dir.join(&rel);
                let dest = unique_target_path(&dir, &rel);
                if let Some(parent) = dest.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::rename(&src, &dest)?;
                promoted.push(dest.to_string_lossy().to_string());
            }
            let _ = fs::remove_dir_all(&staging_dir);
            Ok((wire_bytes, promoted, hex::encode(digest)))
        }
        Err(err) => {
            let _ = fs::remove_dir_all(&staging_dir);
            Err(err)
        }
    }
}

pub(crate) fn expand_tilde(path: &Path) -> PathBuf {
    let raw = path.to_string_lossy();
    if let Some(rest) = raw.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join(rest);
        }
    }
    path.to_path_buf()
}

fn unique_target_path(base_dir: &Path, rel: &Path) -> PathBuf {
    let candidate = base_dir.join(rel);
    if !candidate.exists() {
        return candidate;
    }
    let stem = candidate
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("file");
    let ext = candidate.extension().and_then(|s| s.to_str());
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

fn unique_temp_path(base_dir: &Path, transfer_id: &str) -> PathBuf {
    let seq = TEMP_SEQ.fetch_add(1, Ordering::Relaxed);
    base_dir.join(format!(
        ".keynear-part-{transfer_id}-{}-{seq}.tmp",
        std::process::id()
    ))
}

fn open_private_new_file(path: &Path) -> std::io::Result<File> {
    let mut opts = OpenOptions::new();
    opts.create_new(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    opts.open(path)
}

fn zip_err(err: zip::result::ZipError) -> KeynearError {
    KeynearError::Transfer(format!("zip stream error: {err}"))
}
