use std::fs;
use std::io::{Read, Write};
use std::os::unix::fs::{FileTypeExt, PermissionsExt};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

use keynear_core::{
    default_config_dir, sanitize_device_id, KeynearError, TransferDirection, TransferItem,
    TransferSession, TransferState, VisibilityMode,
};

pub const MAX_IPC_FRAME_BYTES: usize = 262_144;
const IPC_WIRE_VERSION: u8 = 1;
const MAX_IPC_PATHS: usize = 1024;
const MAX_IPC_ITEMS: usize = 1024;
const MAX_IPC_TRANSFERS: usize = 256;
const MAX_TRANSFER_ID_BYTES: usize = 128;
const IPC_CONTROL_TIMEOUT_SECS: u64 = 10;
const IPC_SEND_TIMEOUT_SECS: u64 = 300;

const REQ_SEND_FILES: u8 = 1;
const REQ_ACCEPT_TRANSFER: u8 = 2;
const REQ_DECLINE_TRANSFER: u8 = 3;
const REQ_CANCEL_TRANSFER: u8 = 4;
const REQ_LIST_TRANSFERS: u8 = 5;
const REQ_STATUS: u8 = 6;

const RESP_TRANSFER: u8 = 1;
const RESP_TRANSFERS: u8 = 2;
const RESP_STATUS: u8 = 3;
const RESP_ERROR: u8 = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IpcRequest {
    SendFiles {
        device_id: String,
        paths: Vec<PathBuf>,
    },
    AcceptTransfer {
        transfer_id: String,
    },
    DeclineTransfer {
        transfer_id: String,
    },
    CancelTransfer {
        transfer_id: String,
    },
    ListTransfers,
    Status,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonStatus {
    pub device_id: String,
    pub endpoint_id: String,
    pub listen_addrs: Vec<String>,
    pub visibility_mode: VisibilityMode,
    pub mdns_degraded: bool,
    pub active_transfer_count: u32,
    pub paired_device_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IpcResponse {
    Transfer(TransferSession),
    Transfers(Vec<TransferSession>),
    Status(DaemonStatus),
    Error { message: String },
}

pub fn daemon_socket_path() -> PathBuf {
    default_config_dir().join("daemon.sock")
}

pub fn prepare_daemon_socket_dir(socket_path: &Path) -> Result<(), KeynearError> {
    if let Some(parent) = socket_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
            fs::set_permissions(parent, fs::Permissions::from_mode(0o700))?;
        }
    }

    match fs::symlink_metadata(socket_path) {
        Ok(meta) => {
            if !meta.file_type().is_socket() {
                return Err(KeynearError::Config(format!(
                    "refusing to replace non-socket path at {}",
                    socket_path.display()
                )));
            }
            if UnixStream::connect(socket_path).is_ok() {
                return Err(KeynearError::Config(format!(
                    "keynear-linux daemon is already running at {}",
                    socket_path.display()
                )));
            }
            fs::remove_file(socket_path)?;
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => return Err(KeynearError::Io(err)),
    }

    Ok(())
}

pub fn set_daemon_socket_permissions(socket_path: &Path) -> Result<(), KeynearError> {
    fs::set_permissions(socket_path, fs::Permissions::from_mode(0o600))?;
    Ok(())
}

#[allow(dead_code)]
pub fn send_request(request: &IpcRequest) -> Result<IpcResponse, KeynearError> {
    send_request_to(&daemon_socket_path(), request)
}

pub fn send_request_to(
    socket_path: &Path,
    request: &IpcRequest,
) -> Result<IpcResponse, KeynearError> {
    let encoded = encode_request(request)?;
    let mut stream = UnixStream::connect(socket_path).map_err(|err| match err.kind() {
        std::io::ErrorKind::NotFound | std::io::ErrorKind::ConnectionRefused => {
            KeynearError::Transport(format!(
                "keynear-linux daemon is not running (socket {} unavailable)",
                socket_path.display()
            ))
        }
        _ => KeynearError::Io(err),
    })?;

    let read_timeout = match request {
        IpcRequest::SendFiles { .. } => Duration::from_secs(IPC_SEND_TIMEOUT_SECS),
        _ => Duration::from_secs(IPC_CONTROL_TIMEOUT_SECS),
    };
    stream.set_write_timeout(Some(Duration::from_secs(IPC_CONTROL_TIMEOUT_SECS)))?;
    stream.set_read_timeout(Some(read_timeout))?;

    let frame_len = u32::try_from(encoded.len()).map_err(|_| KeynearError::FrameTooLarge {
        size: encoded.len(),
        max: MAX_IPC_FRAME_BYTES,
    })?;
    stream.write_all(&frame_len.to_be_bytes())?;
    stream.write_all(&encoded)?;
    stream.flush()?;

    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf)?;
    let resp_len = u32::from_be_bytes(len_buf) as usize;
    if resp_len == 0 {
        return Err(KeynearError::Protocol(
            "IPC response frame length must be positive".to_string(),
        ));
    }
    if resp_len > MAX_IPC_FRAME_BYTES {
        return Err(KeynearError::FrameTooLarge {
            size: resp_len,
            max: MAX_IPC_FRAME_BYTES,
        });
    }
    let mut resp_buf = vec![0u8; resp_len];
    stream.read_exact(&mut resp_buf)?;
    decode_response(&resp_buf)
}

pub fn encode_request(request: &IpcRequest) -> Result<Vec<u8>, KeynearError> {
    let mut out = Vec::new();
    out.push(IPC_WIRE_VERSION);
    match request {
        IpcRequest::SendFiles { device_id, paths } => {
            let clean_id = sanitize_device_id(device_id)?;
            if paths.is_empty() || paths.len() > MAX_IPC_PATHS {
                return Err(KeynearError::Validation {
                    field: "paths",
                    message: format!("paths count must be within 1..={MAX_IPC_PATHS}"),
                });
            }
            out.push(REQ_SEND_FILES);
            write_str(&mut out, &clean_id)?;
            write_u32(&mut out, paths.len() as u32);
            for path in paths {
                let raw = path.to_str().ok_or_else(|| KeynearError::Validation {
                    field: "paths",
                    message: "path must be valid UTF-8".to_string(),
                })?;
                if raw.trim().is_empty() {
                    return Err(KeynearError::Validation {
                        field: "paths",
                        message: "path must not be empty".to_string(),
                    });
                }
                write_str(&mut out, raw)?;
            }
        }
        IpcRequest::AcceptTransfer { transfer_id } => {
            let clean_tid = validate_transfer_id(transfer_id)?;
            out.push(REQ_ACCEPT_TRANSFER);
            write_str(&mut out, clean_tid)?;
        }
        IpcRequest::DeclineTransfer { transfer_id } => {
            let clean_tid = validate_transfer_id(transfer_id)?;
            out.push(REQ_DECLINE_TRANSFER);
            write_str(&mut out, clean_tid)?;
        }
        IpcRequest::CancelTransfer { transfer_id } => {
            let clean_tid = validate_transfer_id(transfer_id)?;
            out.push(REQ_CANCEL_TRANSFER);
            write_str(&mut out, clean_tid)?;
        }
        IpcRequest::ListTransfers => {
            out.push(REQ_LIST_TRANSFERS);
        }
        IpcRequest::Status => {
            out.push(REQ_STATUS);
        }
    }

    if out.len() > MAX_IPC_FRAME_BYTES {
        return Err(KeynearError::FrameTooLarge {
            size: out.len(),
            max: MAX_IPC_FRAME_BYTES,
        });
    }
    Ok(out)
}

pub fn decode_request(bytes: &[u8]) -> Result<IpcRequest, KeynearError> {
    if bytes.is_empty() {
        return Err(KeynearError::Protocol("empty IPC request".to_string()));
    }
    if bytes.len() > MAX_IPC_FRAME_BYTES {
        return Err(KeynearError::FrameTooLarge {
            size: bytes.len(),
            max: MAX_IPC_FRAME_BYTES,
        });
    }
    let mut cur = Cursor::new(bytes);
    let version = cur.read_u8()?;
    if version != IPC_WIRE_VERSION {
        return Err(KeynearError::Protocol(format!(
            "unsupported IPC protocol version {version}"
        )));
    }
    let kind = cur.read_u8()?;
    let request = match kind {
        REQ_SEND_FILES => {
            let raw_id = cur.read_string()?;
            let device_id = sanitize_device_id(&raw_id)?;
            let count = cur.read_u32()? as usize;
            if count == 0 || count > MAX_IPC_PATHS {
                return Err(KeynearError::Validation {
                    field: "paths",
                    message: format!("IPC paths count {count} out of bounds"),
                });
            }
            let mut paths = Vec::with_capacity(count);
            for _ in 0..count {
                let raw = cur.read_string()?;
                if raw.trim().is_empty() {
                    return Err(KeynearError::Validation {
                        field: "paths",
                        message: "IPC path must not be empty".to_string(),
                    });
                }
                paths.push(PathBuf::from(raw));
            }
            IpcRequest::SendFiles { device_id, paths }
        }
        REQ_ACCEPT_TRANSFER => {
            let tid = cur.read_string()?;
            let clean = validate_transfer_id(&tid)?.to_string();
            IpcRequest::AcceptTransfer { transfer_id: clean }
        }
        REQ_DECLINE_TRANSFER => {
            let tid = cur.read_string()?;
            let clean = validate_transfer_id(&tid)?.to_string();
            IpcRequest::DeclineTransfer { transfer_id: clean }
        }
        REQ_CANCEL_TRANSFER => {
            let tid = cur.read_string()?;
            let clean = validate_transfer_id(&tid)?.to_string();
            IpcRequest::CancelTransfer { transfer_id: clean }
        }
        REQ_LIST_TRANSFERS => IpcRequest::ListTransfers,
        REQ_STATUS => IpcRequest::Status,
        other => {
            return Err(KeynearError::Protocol(format!(
                "unknown IPC request kind {other}"
            )));
        }
    };
    cur.ensure_exhausted()?;
    Ok(request)
}

pub fn encode_response(response: &IpcResponse) -> Result<Vec<u8>, KeynearError> {
    let mut out = Vec::new();
    out.push(IPC_WIRE_VERSION);
    match response {
        IpcResponse::Transfer(session) => {
            out.push(RESP_TRANSFER);
            write_transfer_session(&mut out, session)?;
        }
        IpcResponse::Transfers(sessions) => {
            if sessions.len() > MAX_IPC_TRANSFERS {
                return Err(KeynearError::FrameTooLarge {
                    size: sessions.len(),
                    max: MAX_IPC_TRANSFERS,
                });
            }
            out.push(RESP_TRANSFERS);
            write_u32(&mut out, sessions.len() as u32);
            for session in sessions {
                write_transfer_session(&mut out, session)?;
            }
        }
        IpcResponse::Status(status) => {
            if status.listen_addrs.len() > MAX_IPC_PATHS {
                return Err(KeynearError::FrameTooLarge {
                    size: status.listen_addrs.len(),
                    max: MAX_IPC_PATHS,
                });
            }
            out.push(RESP_STATUS);
            write_str(&mut out, &status.device_id)?;
            write_str(&mut out, &status.endpoint_id)?;
            write_u32(&mut out, status.listen_addrs.len() as u32);
            for addr in &status.listen_addrs {
                write_str(&mut out, addr)?;
            }
            out.push(encode_visibility(status.visibility_mode));
            out.push(u8::from(status.mdns_degraded));
            write_u32(&mut out, status.active_transfer_count);
            write_u32(&mut out, status.paired_device_count);
        }
        IpcResponse::Error { message } => {
            out.push(RESP_ERROR);
            write_str(&mut out, message)?;
        }
    }

    if out.len() > MAX_IPC_FRAME_BYTES {
        return Err(KeynearError::FrameTooLarge {
            size: out.len(),
            max: MAX_IPC_FRAME_BYTES,
        });
    }
    Ok(out)
}

pub fn decode_response(bytes: &[u8]) -> Result<IpcResponse, KeynearError> {
    if bytes.is_empty() {
        return Err(KeynearError::Protocol("empty IPC response".to_string()));
    }
    if bytes.len() > MAX_IPC_FRAME_BYTES {
        return Err(KeynearError::FrameTooLarge {
            size: bytes.len(),
            max: MAX_IPC_FRAME_BYTES,
        });
    }
    let mut cur = Cursor::new(bytes);
    let version = cur.read_u8()?;
    if version != IPC_WIRE_VERSION {
        return Err(KeynearError::Protocol(format!(
            "unsupported IPC protocol version {version}"
        )));
    }
    let kind = cur.read_u8()?;
    let response = match kind {
        RESP_TRANSFER => IpcResponse::Transfer(read_transfer_session(&mut cur)?),
        RESP_TRANSFERS => {
            let count = cur.read_u32()? as usize;
            if count > MAX_IPC_TRANSFERS {
                return Err(KeynearError::FrameTooLarge {
                    size: count,
                    max: MAX_IPC_TRANSFERS,
                });
            }
            let mut list = Vec::with_capacity(count);
            for _ in 0..count {
                list.push(read_transfer_session(&mut cur)?);
            }
            IpcResponse::Transfers(list)
        }
        RESP_STATUS => {
            let device_id = cur.read_string()?;
            let endpoint_id = cur.read_string()?;
            let addr_count = cur.read_u32()? as usize;
            if addr_count > MAX_IPC_PATHS {
                return Err(KeynearError::FrameTooLarge {
                    size: addr_count,
                    max: MAX_IPC_PATHS,
                });
            }
            let mut listen_addrs = Vec::with_capacity(addr_count);
            for _ in 0..addr_count {
                listen_addrs.push(cur.read_string()?);
            }
            let visibility_mode = decode_visibility(cur.read_u8()?)?;
            let mdns_degraded = cur.read_bool()?;
            let active_transfer_count = cur.read_u32()?;
            let paired_device_count = cur.read_u32()?;
            IpcResponse::Status(DaemonStatus {
                device_id,
                endpoint_id,
                listen_addrs,
                visibility_mode,
                mdns_degraded,
                active_transfer_count,
                paired_device_count,
            })
        }
        RESP_ERROR => {
            let message = cur.read_string()?;
            IpcResponse::Error { message }
        }
        other => {
            return Err(KeynearError::Protocol(format!(
                "unknown IPC response kind {other}"
            )));
        }
    };
    cur.ensure_exhausted()?;
    Ok(response)
}

pub fn validate_transfer_id(raw: &str) -> Result<&str, KeynearError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.len() > MAX_TRANSFER_ID_BYTES {
        return Err(KeynearError::Validation {
            field: "transferId",
            message: format!("transfer_id must be between 1 and {MAX_TRANSFER_ID_BYTES} bytes"),
        });
    }
    if !trimmed
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
    {
        return Err(KeynearError::Validation {
            field: "transferId",
            message: "transfer_id contains invalid characters".to_string(),
        });
    }
    Ok(trimmed)
}

fn write_transfer_session(
    out: &mut Vec<u8>,
    session: &TransferSession,
) -> Result<(), KeynearError> {
    if session.items.len() > MAX_IPC_ITEMS {
        return Err(KeynearError::FrameTooLarge {
            size: session.items.len(),
            max: MAX_IPC_ITEMS,
        });
    }
    write_str(out, &session.transfer_id)?;
    write_str(out, &session.peer_device_id)?;
    out.push(match session.direction {
        TransferDirection::Send => 1,
        TransferDirection::Receive => 2,
    });
    write_u32(out, session.items.len() as u32);
    for item in &session.items {
        write_str(out, &item.name)?;
        write_str(out, &item.mime_type)?;
        write_u64(out, item.size_bytes);
        out.push(u8::from(item.is_directory));
        write_opt_str(out, item.target_path.as_deref())?;
        write_opt_str(out, item.checksum.as_deref())?;
    }
    write_u64(out, session.total_bytes);
    write_u64(out, session.transferred_bytes);
    out.push(match session.state {
        TransferState::PendingConsent => 1,
        TransferState::Active => 2,
        TransferState::Completed => 3,
        TransferState::Cancelled => 4,
        TransferState::Failed => 5,
    });
    write_str(out, &session.started_at)?;
    Ok(())
}

fn read_transfer_session(cur: &mut Cursor<'_>) -> Result<TransferSession, KeynearError> {
    let transfer_id = cur.read_string()?;
    let peer_device_id = cur.read_string()?;
    let direction = match cur.read_u8()? {
        1 => TransferDirection::Send,
        2 => TransferDirection::Receive,
        other => {
            return Err(KeynearError::Protocol(format!(
                "invalid IPC transfer direction {other}"
            )));
        }
    };
    let item_count = cur.read_u32()? as usize;
    if item_count > MAX_IPC_ITEMS {
        return Err(KeynearError::FrameTooLarge {
            size: item_count,
            max: MAX_IPC_ITEMS,
        });
    }
    let mut items = Vec::with_capacity(item_count);
    for _ in 0..item_count {
        let name = cur.read_string()?;
        let mime_type = cur.read_string()?;
        let size_bytes = cur.read_u64()?;
        let is_directory = cur.read_bool()?;
        let target_path = cur.read_opt_string()?;
        let checksum = cur.read_opt_string()?;
        items.push(TransferItem {
            name,
            mime_type,
            size_bytes,
            is_directory,
            target_path,
            checksum,
        });
    }
    let total_bytes = cur.read_u64()?;
    let transferred_bytes = cur.read_u64()?;
    let state = match cur.read_u8()? {
        1 => TransferState::PendingConsent,
        2 => TransferState::Active,
        3 => TransferState::Completed,
        4 => TransferState::Cancelled,
        5 => TransferState::Failed,
        other => {
            return Err(KeynearError::Protocol(format!(
                "invalid IPC transfer state {other}"
            )));
        }
    };
    let started_at = cur.read_string()?;
    Ok(TransferSession {
        transfer_id,
        peer_device_id,
        direction,
        items,
        total_bytes,
        transferred_bytes,
        state,
        started_at,
    })
}

fn encode_visibility(mode: VisibilityMode) -> u8 {
    match mode {
        VisibilityMode::Hidden => 1,
        VisibilityMode::PairedOnly => 2,
        VisibilityMode::EveryoneNearby => 3,
    }
}

fn decode_visibility(raw: u8) -> Result<VisibilityMode, KeynearError> {
    match raw {
        1 => Ok(VisibilityMode::Hidden),
        2 => Ok(VisibilityMode::PairedOnly),
        3 => Ok(VisibilityMode::EveryoneNearby),
        other => Err(KeynearError::Protocol(format!(
            "invalid IPC visibility mode {other}"
        ))),
    }
}

fn write_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn write_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn write_str(out: &mut Vec<u8>, value: &str) -> Result<(), KeynearError> {
    let len = u32::try_from(value.len()).map_err(|_| KeynearError::FrameTooLarge {
        size: value.len(),
        max: MAX_IPC_FRAME_BYTES,
    })?;
    write_u32(out, len);
    out.extend_from_slice(value.as_bytes());
    Ok(())
}

fn write_opt_str(out: &mut Vec<u8>, value: Option<&str>) -> Result<(), KeynearError> {
    match value {
        Some(inner) => {
            out.push(1);
            write_str(out, inner)?;
        }
        None => {
            out.push(0);
        }
    }
    Ok(())
}

struct Cursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Cursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn read_exact(&mut self, len: usize) -> Result<&'a [u8], KeynearError> {
        let end = self
            .offset
            .checked_add(len)
            .ok_or_else(|| KeynearError::Protocol("IPC frame offset overflow".to_string()))?;
        if end > self.bytes.len() {
            return Err(KeynearError::Protocol("truncated IPC frame".to_string()));
        }
        let slice = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(slice)
    }

    fn read_u8(&mut self) -> Result<u8, KeynearError> {
        Ok(self.read_exact(1)?[0])
    }

    fn read_bool(&mut self) -> Result<bool, KeynearError> {
        match self.read_u8()? {
            0 => Ok(false),
            1 => Ok(true),
            other => Err(KeynearError::Protocol(format!(
                "invalid IPC boolean byte {other}"
            ))),
        }
    }

    fn read_u32(&mut self) -> Result<u32, KeynearError> {
        let slice = self.read_exact(4)?;
        let mut arr = [0u8; 4];
        arr.copy_from_slice(slice);
        Ok(u32::from_be_bytes(arr))
    }

    fn read_u64(&mut self) -> Result<u64, KeynearError> {
        let slice = self.read_exact(8)?;
        let mut arr = [0u8; 8];
        arr.copy_from_slice(slice);
        Ok(u64::from_be_bytes(arr))
    }

    fn read_string(&mut self) -> Result<String, KeynearError> {
        let len = self.read_u32()? as usize;
        if len > MAX_IPC_FRAME_BYTES {
            return Err(KeynearError::FrameTooLarge {
                size: len,
                max: MAX_IPC_FRAME_BYTES,
            });
        }
        let raw = self.read_exact(len)?;
        std::str::from_utf8(raw)
            .map(ToString::to_string)
            .map_err(|_| KeynearError::Protocol("invalid UTF-8 in IPC frame".to_string()))
    }

    fn read_opt_string(&mut self) -> Result<Option<String>, KeynearError> {
        if self.read_bool()? {
            Ok(Some(self.read_string()?))
        } else {
            Ok(None)
        }
    }

    fn ensure_exhausted(&self) -> Result<(), KeynearError> {
        if self.offset != self.bytes.len() {
            return Err(KeynearError::Protocol(
                "unexpected trailing bytes in IPC frame".to_string(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::os::unix::net::UnixListener;
    use std::thread;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    fn temp_sock_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!("keynear-ipc-{label}-{nanos}"))
    }

    fn sample_session() -> TransferSession {
        TransferSession {
            transfer_id: "tx-123".to_string(),
            peer_device_id: "pixel-9".to_string(),
            direction: TransferDirection::Receive,
            items: vec![TransferItem {
                name: "report\nfinal.pdf".to_string(),
                mime_type: "application/pdf".to_string(),
                size_bytes: 12_345,
                is_directory: false,
                target_path: Some("/tmp/report\nfinal.pdf".to_string()),
                checksum: Some("abcd".repeat(16)),
            }],
            total_bytes: 12_345,
            transferred_bytes: 6_144,
            state: TransferState::PendingConsent,
            started_at: "1700000000".to_string(),
        }
    }

    #[test]
    fn request_and_response_roundtrip_all_variants() {
        let requests = vec![
            IpcRequest::SendFiles {
                device_id: "pixel-9".to_string(),
                paths: vec![
                    PathBuf::from("/tmp/hello world.txt"),
                    PathBuf::from("/tmp/dir/sub\nfile.bin"),
                ],
            },
            IpcRequest::AcceptTransfer {
                transfer_id: "tx-1".to_string(),
            },
            IpcRequest::DeclineTransfer {
                transfer_id: "tx-2".to_string(),
            },
            IpcRequest::CancelTransfer {
                transfer_id: "tx-3".to_string(),
            },
            IpcRequest::ListTransfers,
            IpcRequest::Status,
        ];
        for req in requests {
            let encoded = encode_request(&req).expect("encode request");
            let decoded = decode_request(&encoded).expect("decode request");
            assert_eq!(decoded, req);
        }

        let responses = vec![
            IpcResponse::Transfer(sample_session()),
            IpcResponse::Transfers(vec![sample_session()]),
            IpcResponse::Status(DaemonStatus {
                device_id: "linux-workstation".to_string(),
                endpoint_id: "ab".repeat(32),
                listen_addrs: vec!["127.0.0.1:42000".to_string()],
                visibility_mode: VisibilityMode::PairedOnly,
                mdns_degraded: true,
                active_transfer_count: 2,
                paired_device_count: 1,
            }),
            IpcResponse::Error {
                message: "transfer not found".to_string(),
            },
        ];
        for resp in responses {
            let encoded = encode_response(&resp).expect("encode response");
            let decoded = decode_response(&encoded).expect("decode response");
            assert_eq!(decoded, resp);
        }
    }

    #[test]
    fn rejects_invalid_and_oversized_ipc_frames() {
        assert!(matches!(
            encode_request(&IpcRequest::SendFiles {
                device_id: "../evil".to_string(),
                paths: vec![PathBuf::from("/tmp/a")],
            }),
            Err(KeynearError::Validation {
                field: "deviceId",
                ..
            })
        ));
        assert!(matches!(
            encode_request(&IpcRequest::SendFiles {
                device_id: "pixel-9".to_string(),
                paths: Vec::new(),
            }),
            Err(KeynearError::Validation { field: "paths", .. })
        ));
        assert!(matches!(
            encode_request(&IpcRequest::AcceptTransfer {
                transfer_id: "bad/id".to_string(),
            }),
            Err(KeynearError::Validation {
                field: "transferId",
                ..
            })
        ));
        assert!(matches!(
            decode_request(&[]),
            Err(KeynearError::Protocol(_))
        ));
        let mut bad_trailing = encode_request(&IpcRequest::Status).expect("encode");
        bad_trailing.push(0x99);
        assert!(matches!(
            decode_request(&bad_trailing),
            Err(KeynearError::Protocol(_))
        ));
        let oversized = vec![0u8; MAX_IPC_FRAME_BYTES + 1];
        assert!(matches!(
            decode_response(&oversized),
            Err(KeynearError::FrameTooLarge { .. })
        ));
    }

    #[test]
    fn loopback_unix_socket_enforces_0600_permissions_and_roundtrips() {
        let dir = temp_sock_dir("loopback");
        let sock_path = dir.join("daemon.sock");
        prepare_daemon_socket_dir(&sock_path).expect("prepare dir");
        let listener = UnixListener::bind(&sock_path).expect("bind listener");
        set_daemon_socket_permissions(&sock_path).expect("set 0600");

        let dir_mode = fs::metadata(&dir).expect("dir meta").permissions().mode() & 0o777;
        let sock_mode = fs::metadata(&sock_path)
            .expect("sock meta")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(dir_mode, 0o700);
        assert_eq!(sock_mode, 0o600);

        let expected = sample_session();
        let expected_clone = expected.clone();
        let server = thread::spawn(move || {
            let (mut conn, _) = listener.accept().expect("accept");
            let mut len_buf = [0u8; 4];
            conn.read_exact(&mut len_buf).expect("read len");
            let len = u32::from_be_bytes(len_buf) as usize;
            let mut buf = vec![0u8; len];
            conn.read_exact(&mut buf).expect("read body");
            let req = decode_request(&buf).expect("decode req");
            assert_eq!(
                req,
                IpcRequest::AcceptTransfer {
                    transfer_id: "tx-123".to_string()
                }
            );
            let resp =
                encode_response(&IpcResponse::Transfer(expected_clone)).expect("encode resp");
            let rlen = (resp.len() as u32).to_be_bytes();
            conn.write_all(&rlen).expect("write len");
            conn.write_all(&resp).expect("write body");
        });

        let response = send_request_to(
            &sock_path,
            &IpcRequest::AcceptTransfer {
                transfer_id: "tx-123".to_string(),
            },
        )
        .expect("send ipc request");
        assert_eq!(response, IpcResponse::Transfer(expected));
        server.join().expect("server thread");

        let _ = fs::remove_dir_all(&dir);
    }
}
