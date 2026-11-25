use std::io::SeekFrom;
use std::path::{Component, Path};
use std::sync::Arc;

use keynear_core::TransferEngine;
use keynear_protocol::{KeynearError, STREAM_CHUNK_BYTES};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncSeekExt, AsyncWrite, AsyncWriteExt};

use crate::callbacks::FfiError;

const MAX_RELATIVE_PATH_BYTES: usize = 256;

pub(crate) fn validate_relative_path(raw: &str) -> Result<String, FfiError> {
    let path = raw.trim();
    if path.is_empty() {
        return Err(FfiError::Validation {
            field: "relativePath".to_string(),
            detail: "path must not be empty".to_string(),
        });
    }
    if path.len() > MAX_RELATIVE_PATH_BYTES {
        return Err(FfiError::Validation {
            field: "relativePath".to_string(),
            detail: "path must not exceed 256 bytes".to_string(),
        });
    }
    if path.contains('\0') {
        return Err(FfiError::Validation {
            field: "relativePath".to_string(),
            detail: "path must not contain NUL bytes".to_string(),
        });
    }
    for component in Path::new(path).components() {
        if matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        ) {
            return Err(FfiError::Validation {
                field: "relativePath".to_string(),
                detail: "path must be relative without parent traversal".to_string(),
            });
        }
    }
    Ok(path.to_string())
}

pub(crate) async fn write_vfs_read_request<W: AsyncWrite + Unpin>(
    writer: &mut W,
    path: &str,
    offset: u64,
    max_bytes: u32,
) -> Result<(), KeynearError> {
    let path_len = u16::try_from(path.len()).map_err(|_| KeynearError::Validation {
        field: "relativePath",
        message: "path length exceeds u16".to_string(),
    })?;
    if max_bytes == 0 || max_bytes > STREAM_CHUNK_BYTES as u32 {
        return Err(KeynearError::Validation {
            field: "maxBytes",
            message: "max_bytes must be 1..=STREAM_CHUNK_BYTES".to_string(),
        });
    }
    writer.write_all(&path_len.to_be_bytes()).await?;
    writer.write_all(path.as_bytes()).await?;
    writer.write_all(&offset.to_be_bytes()).await?;
    writer.write_all(&max_bytes.to_be_bytes()).await?;
    Ok(())
}

pub(crate) async fn read_vfs_read_request<R: AsyncRead + Unpin>(
    reader: &mut R,
) -> Result<(String, u64, u32), KeynearError> {
    let mut len_buf = [0u8; 2];
    reader.read_exact(&mut len_buf).await?;
    let path_len = u16::from_be_bytes(len_buf) as usize;
    if path_len == 0 || path_len > MAX_RELATIVE_PATH_BYTES {
        return Err(KeynearError::Validation {
            field: "relativePath",
            message: "path length out of bounds".to_string(),
        });
    }
    let mut path_buf = vec![0u8; path_len];
    reader.read_exact(&mut path_buf).await?;
    let raw = String::from_utf8(path_buf)
        .map_err(|_| KeynearError::Protocol("vfs read path is not UTF-8".to_string()))?;
    let path = validate_relative_path(&raw)
        .map_err(|err| KeynearError::Protocol(format!("invalid vfs read path: {err}")))?;

    let mut offset_buf = [0u8; 8];
    reader.read_exact(&mut offset_buf).await?;
    let offset = u64::from_be_bytes(offset_buf);

    let mut max_buf = [0u8; 4];
    reader.read_exact(&mut max_buf).await?;
    let max_bytes = u32::from_be_bytes(max_buf);
    if max_bytes == 0 || max_bytes > STREAM_CHUNK_BYTES as u32 {
        return Err(KeynearError::Validation {
            field: "maxBytes",
            message: "max_bytes out of bounds".to_string(),
        });
    }
    Ok((path, offset, max_bytes))
}

pub(crate) async fn read_vfs_read_reply<R: AsyncRead + Unpin>(
    reader: &mut R,
    max_bytes: u32,
) -> Result<Vec<u8>, KeynearError> {
    let mut len_buf = [0u8; 4];
    reader.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf);
    if len == 0 {
        return Err(KeynearError::Protocol(
            "vfs read reply length must be positive".to_string(),
        ));
    }
    let bound = max_bytes.min(STREAM_CHUNK_BYTES as u32);
    if len > bound {
        return Err(KeynearError::FrameTooLarge {
            size: len as usize,
            max: bound as usize,
        });
    }
    let mut payload = vec![0u8; len as usize];
    reader.read_exact(&mut payload).await?;
    Ok(payload)
}

async fn write_vfs_read_reply<W: AsyncWrite + Unpin>(
    writer: &mut W,
    bytes: &[u8],
) -> Result<(), KeynearError> {
    let len = u32::try_from(bytes.len()).map_err(|_| KeynearError::FrameTooLarge {
        size: bytes.len(),
        max: u32::MAX as usize,
    })?;
    writer.write_all(&len.to_be_bytes()).await?;
    writer.write_all(bytes).await?;
    Ok(())
}

async fn read_bounded_chunk(
    engine: &TransferEngine,
    path: &str,
    offset: u64,
    max_bytes: u32,
) -> Option<Vec<u8>> {
    let incoming = engine.incoming_directory();
    let root = tokio::fs::canonicalize(&incoming).await.ok()?;
    let candidate = tokio::fs::canonicalize(incoming.join(path)).await.ok()?;
    if !candidate.starts_with(&root) {
        return None;
    }
    let mut file = tokio::fs::File::open(candidate).await.ok()?;
    file.seek(SeekFrom::Start(offset)).await.ok()?;
    let mut buf = vec![0u8; max_bytes.min(STREAM_CHUNK_BYTES as u32) as usize];
    let read = file.read(&mut buf).await.ok()?;
    buf.truncate(read);
    Some(buf)
}

async fn resolve_vfs_read<R: AsyncRead + Unpin>(engine: &TransferEngine, recv: &mut R) -> Vec<u8> {
    let (path, offset, max_bytes) = match read_vfs_read_request(recv).await {
        Ok(request) => request,
        Err(err) => {
            eprintln!("keynear: rejected vfs read request: {err}");
            return Vec::new();
        }
    };
    match read_bounded_chunk(engine, &path, offset, max_bytes).await {
        Some(bytes) => bytes,
        None => {
            eprintln!("keynear: vfs read chunk unavailable");
            Vec::new()
        }
    }
}

pub(crate) async fn serve_vfs_read<S, R>(engine: &Arc<TransferEngine>, mut send: S, mut recv: R)
where
    S: AsyncWrite + Unpin,
    R: AsyncRead + Unpin,
{
    let bytes = resolve_vfs_read(engine, &mut recv).await;
    if let Err(err) = write_vfs_read_reply(&mut send, &bytes).await {
        eprintln!("keynear: failed to write vfs read reply: {err}");
    }
    if let Err(err) = send.shutdown().await {
        eprintln!("keynear: failed to finish vfs read stream: {err}");
    }
}

#[cfg(test)]
#[path = "vfs_read/tests.rs"]
mod tests;
