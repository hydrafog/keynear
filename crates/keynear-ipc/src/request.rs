use crate::config::*;
use crate::*;

use crate::codec::*;

pub fn encode_request(request: &IpcRequest) -> Result<Vec<u8>, KeynearError> {
    let mut out = Vec::new();
    out.push(IPC_WIRE_VERSION);
    match request {
        IpcRequest::SendFiles { device_id, paths } => write_send_files(&mut out, device_id, paths)?,
        IpcRequest::AcceptTransfer { transfer_id } => {
            write_transfer_id_request(&mut out, REQ_ACCEPT_TRANSFER, transfer_id)?
        }
        IpcRequest::DeclineTransfer { transfer_id } => {
            write_transfer_id_request(&mut out, REQ_DECLINE_TRANSFER, transfer_id)?
        }
        IpcRequest::CancelTransfer { transfer_id } => {
            write_transfer_id_request(&mut out, REQ_CANCEL_TRANSFER, transfer_id)?
        }
        IpcRequest::ListTransfers => out.push(REQ_LIST_TRANSFERS),
        IpcRequest::Status => out.push(REQ_STATUS),
        IpcRequest::ListDevices => out.push(REQ_LIST_DEVICES),
        IpcRequest::SetVisibility { mode } => {
            out.push(REQ_SET_VISIBILITY);
            out.push(encode_visibility(*mode));
        }
        IpcRequest::GetConfig => out.push(REQ_GET_CONFIG),
        IpcRequest::UpdateConfig { config } => write_config_request(&mut out, config)?,
        IpcRequest::RenameDevice {
            device_id,
            display_name,
        } => write_rename_request(&mut out, device_id, display_name)?,
        IpcRequest::SetDeviceAutoAccept { device_id, enabled } => {
            let clean_id = sanitize_device_id(device_id)?;
            out.push(REQ_SET_DEVICE_AUTO_ACCEPT);
            write_str(&mut out, &clean_id)?;
            out.push(u8::from(*enabled));
        }
        IpcRequest::UnpairDevice { device_id } => {
            let clean_id = sanitize_device_id(device_id)?;
            out.push(REQ_UNPAIR_DEVICE);
            write_str(&mut out, &clean_id)?;
        }
    }

    check_frame_size(&out)?;
    Ok(out)
}

fn write_send_files(
    out: &mut Vec<u8>,
    device_id: &str,
    paths: &[PathBuf],
) -> Result<(), KeynearError> {
    let clean_id = sanitize_device_id(device_id)?;
    if paths.is_empty() || paths.len() > MAX_IPC_PATHS {
        return Err(KeynearError::Validation {
            field: "paths",
            message: format!("paths count must be within 1..={MAX_IPC_PATHS}"),
        });
    }
    out.push(REQ_SEND_FILES);
    write_str(out, &clean_id)?;
    write_u32(out, paths.len() as u32);
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
        write_str(out, raw)?;
    }
    Ok(())
}

fn write_transfer_id_request(
    out: &mut Vec<u8>,
    code: u8,
    transfer_id: &str,
) -> Result<(), KeynearError> {
    let clean_tid = validate_transfer_id(transfer_id)?;
    out.push(code);
    write_str(out, clean_tid)
}

fn write_config_request(out: &mut Vec<u8>, config: &DaemonConfig) -> Result<(), KeynearError> {
    config.validate()?;
    out.push(REQ_UPDATE_CONFIG);
    write_daemon_config(out, config)
}

fn write_rename_request(
    out: &mut Vec<u8>,
    device_id: &str,
    display_name: &str,
) -> Result<(), KeynearError> {
    let clean_id = sanitize_device_id(device_id)?;
    let clean_name = validate_display_name(display_name)?;
    out.push(REQ_RENAME_DEVICE);
    write_str(out, &clean_id)?;
    write_str(out, &clean_name)
}

fn check_frame_size(out: &[u8]) -> Result<(), KeynearError> {
    if out.len() > MAX_IPC_FRAME_BYTES {
        return Err(KeynearError::FrameTooLarge {
            size: out.len(),
            max: MAX_IPC_FRAME_BYTES,
        });
    }
    Ok(())
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
        REQ_SEND_FILES => read_send_files(&mut cur)?,
        REQ_ACCEPT_TRANSFER => read_transfer_id_request(&mut cur, |id| {
            IpcRequest::AcceptTransfer { transfer_id: id }
        })?,
        REQ_DECLINE_TRANSFER => read_transfer_id_request(&mut cur, |id| {
            IpcRequest::DeclineTransfer { transfer_id: id }
        })?,
        REQ_CANCEL_TRANSFER => read_transfer_id_request(&mut cur, |id| {
            IpcRequest::CancelTransfer { transfer_id: id }
        })?,
        REQ_LIST_TRANSFERS => IpcRequest::ListTransfers,
        REQ_STATUS => IpcRequest::Status,
        REQ_LIST_DEVICES => IpcRequest::ListDevices,
        REQ_SET_VISIBILITY => IpcRequest::SetVisibility {
            mode: decode_visibility(cur.read_u8()?)?,
        },
        REQ_GET_CONFIG => IpcRequest::GetConfig,
        REQ_UPDATE_CONFIG => IpcRequest::UpdateConfig {
            config: read_daemon_config(&mut cur)?,
        },
        REQ_RENAME_DEVICE => read_rename_request(&mut cur)?,
        REQ_SET_DEVICE_AUTO_ACCEPT => read_auto_accept_request(&mut cur)?,
        REQ_UNPAIR_DEVICE => read_unpair_request(&mut cur)?,
        other => {
            return Err(KeynearError::Protocol(format!(
                "unknown IPC request kind {other}"
            )));
        }
    };
    cur.ensure_exhausted()?;
    Ok(request)
}

fn read_send_files(cur: &mut Cursor<'_>) -> Result<IpcRequest, KeynearError> {
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
    Ok(IpcRequest::SendFiles { device_id, paths })
}

fn read_transfer_id_request(
    cur: &mut Cursor<'_>,
    build: impl FnOnce(String) -> IpcRequest,
) -> Result<IpcRequest, KeynearError> {
    let tid = cur.read_string()?;
    let clean = validate_transfer_id(&tid)?.to_string();
    Ok(build(clean))
}

fn read_rename_request(cur: &mut Cursor<'_>) -> Result<IpcRequest, KeynearError> {
    let raw_id = cur.read_string()?;
    let device_id = sanitize_device_id(&raw_id)?;
    let raw_name = cur.read_string()?;
    let display_name = validate_display_name(&raw_name)?;
    Ok(IpcRequest::RenameDevice {
        device_id,
        display_name,
    })
}

fn read_auto_accept_request(cur: &mut Cursor<'_>) -> Result<IpcRequest, KeynearError> {
    let raw_id = cur.read_string()?;
    let device_id = sanitize_device_id(&raw_id)?;
    let enabled = cur.read_bool()?;
    Ok(IpcRequest::SetDeviceAutoAccept { device_id, enabled })
}

fn read_unpair_request(cur: &mut Cursor<'_>) -> Result<IpcRequest, KeynearError> {
    let raw_id = cur.read_string()?;
    let device_id = sanitize_device_id(&raw_id)?;
    Ok(IpcRequest::UnpairDevice { device_id })
}
