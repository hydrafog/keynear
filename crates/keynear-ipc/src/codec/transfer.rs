use crate::config::*;
use crate::*;
use keynear_protocol::{TransferDirection, TransferItem, TransferState};

use super::primitives::{write_opt_str, write_str, write_u32, write_u64, Cursor};

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

pub(crate) fn write_transfer_session(
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

pub(crate) fn read_transfer_session(cur: &mut Cursor<'_>) -> Result<TransferSession, KeynearError> {
    let transfer_id = cur.read_string()?;
    let peer_device_id = cur.read_string()?;
    let direction = read_direction(cur)?;
    let items = read_items(cur)?;
    let total_bytes = cur.read_u64()?;
    let transferred_bytes = cur.read_u64()?;
    let state = read_state(cur)?;
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

fn read_direction(cur: &mut Cursor<'_>) -> Result<TransferDirection, KeynearError> {
    match cur.read_u8()? {
        1 => Ok(TransferDirection::Send),
        2 => Ok(TransferDirection::Receive),
        other => Err(KeynearError::Protocol(format!(
            "invalid IPC transfer direction {other}"
        ))),
    }
}

fn read_items(cur: &mut Cursor<'_>) -> Result<Vec<TransferItem>, KeynearError> {
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
    Ok(items)
}

fn read_state(cur: &mut Cursor<'_>) -> Result<TransferState, KeynearError> {
    match cur.read_u8()? {
        1 => Ok(TransferState::PendingConsent),
        2 => Ok(TransferState::Active),
        3 => Ok(TransferState::Completed),
        4 => Ok(TransferState::Cancelled),
        5 => Ok(TransferState::Failed),
        other => Err(KeynearError::Protocol(format!(
            "invalid IPC transfer state {other}"
        ))),
    }
}
