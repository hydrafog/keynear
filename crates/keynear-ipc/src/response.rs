use crate::config::*;
use crate::*;

use crate::codec::*;

pub fn encode_response(response: &IpcResponse) -> Result<Vec<u8>, KeynearError> {
    let mut out = Vec::new();
    out.push(IPC_WIRE_VERSION);
    match response {
        IpcResponse::Transfer(session) => {
            out.push(RESP_TRANSFER);
            write_transfer_session(&mut out, session)?;
        }
        IpcResponse::Transfers(sessions) => write_transfers(&mut out, sessions)?,
        IpcResponse::Status(status) => write_status(&mut out, status)?,
        IpcResponse::Devices(devices) => write_devices(&mut out, devices)?,
        IpcResponse::Device(device) => {
            out.push(RESP_DEVICE);
            write_paired_device(&mut out, device)?;
        }
        IpcResponse::Config(config) => {
            config.validate()?;
            out.push(RESP_CONFIG);
            write_daemon_config(&mut out, config)?;
        }
        IpcResponse::Unpaired { device_id } => {
            let clean_id = sanitize_device_id(device_id)?;
            out.push(RESP_UNPAIRED);
            write_str(&mut out, &clean_id)?;
        }
        IpcResponse::PairingDecision { approved } => {
            out.extend_from_slice(&[RESP_PAIRING_DECISION, u8::from(*approved)]);
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

fn write_transfers(out: &mut Vec<u8>, sessions: &[TransferSession]) -> Result<(), KeynearError> {
    if sessions.len() > MAX_IPC_TRANSFERS {
        return Err(KeynearError::FrameTooLarge {
            size: sessions.len(),
            max: MAX_IPC_TRANSFERS,
        });
    }
    out.push(RESP_TRANSFERS);
    write_u32(out, sessions.len() as u32);
    for session in sessions {
        write_transfer_session(out, session)?;
    }
    Ok(())
}

fn write_status(out: &mut Vec<u8>, status: &DaemonStatus) -> Result<(), KeynearError> {
    if status.listen_addrs.len() > MAX_IPC_PATHS {
        return Err(KeynearError::FrameTooLarge {
            size: status.listen_addrs.len(),
            max: MAX_IPC_PATHS,
        });
    }
    out.push(RESP_STATUS);
    write_str(out, &status.device_id)?;
    write_str(out, &status.endpoint_id)?;
    write_u32(out, status.listen_addrs.len() as u32);
    for addr in &status.listen_addrs {
        write_str(out, addr)?;
    }
    out.push(encode_visibility(status.visibility_mode));
    out.push(u8::from(status.mdns_degraded));
    write_u32(out, status.active_transfer_count);
    write_u32(out, status.paired_device_count);
    match status.everyone_remaining_secs {
        Some(rem) => {
            out.push(1);
            write_u64(out, rem);
        }
        None => out.push(0),
    }
    out.push(u8::from(status.auto_accept_from_paired));
    write_str(out, &status.incoming_directory)
}

fn write_devices(out: &mut Vec<u8>, devices: &[PairedDevice]) -> Result<(), KeynearError> {
    if devices.len() > MAX_IPC_DEVICES {
        return Err(KeynearError::FrameTooLarge {
            size: devices.len(),
            max: MAX_IPC_DEVICES,
        });
    }
    out.push(RESP_DEVICES);
    write_u32(out, devices.len() as u32);
    for device in devices {
        write_paired_device(out, device)?;
    }
    Ok(())
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
        RESP_TRANSFERS => read_transfers(&mut cur)?,
        RESP_STATUS => read_status(&mut cur)?,
        RESP_DEVICES => read_devices(&mut cur)?,
        RESP_DEVICE => IpcResponse::Device(read_paired_device(&mut cur)?),
        RESP_CONFIG => IpcResponse::Config(read_daemon_config(&mut cur)?),
        RESP_UNPAIRED => read_unpaired(&mut cur)?,
        RESP_PAIRING_DECISION => IpcResponse::PairingDecision {
            approved: cur.read_bool()?,
        },
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

fn read_transfers(cur: &mut Cursor<'_>) -> Result<IpcResponse, KeynearError> {
    let count = cur.read_u32()? as usize;
    if count > MAX_IPC_TRANSFERS {
        return Err(KeynearError::FrameTooLarge {
            size: count,
            max: MAX_IPC_TRANSFERS,
        });
    }
    let mut list = Vec::with_capacity(count);
    for _ in 0..count {
        list.push(read_transfer_session(cur)?);
    }
    Ok(IpcResponse::Transfers(list))
}

fn read_status(cur: &mut Cursor<'_>) -> Result<IpcResponse, KeynearError> {
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
    let everyone_remaining_secs = if cur.read_bool()? {
        Some(cur.read_u64()?)
    } else {
        None
    };
    let auto_accept_from_paired = cur.read_bool()?;
    let incoming_directory = cur.read_string()?;
    Ok(IpcResponse::Status(DaemonStatus {
        device_id,
        endpoint_id,
        listen_addrs,
        visibility_mode,
        mdns_degraded,
        active_transfer_count,
        paired_device_count,
        everyone_remaining_secs,
        auto_accept_from_paired,
        incoming_directory,
    }))
}

fn read_devices(cur: &mut Cursor<'_>) -> Result<IpcResponse, KeynearError> {
    let count = cur.read_u32()? as usize;
    if count > MAX_IPC_DEVICES {
        return Err(KeynearError::FrameTooLarge {
            size: count,
            max: MAX_IPC_DEVICES,
        });
    }
    let mut devices = Vec::with_capacity(count);
    for _ in 0..count {
        devices.push(read_paired_device(cur)?);
    }
    Ok(IpcResponse::Devices(devices))
}

fn read_unpaired(cur: &mut Cursor<'_>) -> Result<IpcResponse, KeynearError> {
    let device_id = sanitize_device_id(&cur.read_string()?)?;
    Ok(IpcResponse::Unpaired { device_id })
}
