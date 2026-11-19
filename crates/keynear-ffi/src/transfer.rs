#[cfg(unix)]
use std::fs::File;
#[cfg(unix)]
use std::os::fd::{FromRawFd, OwnedFd};
use std::path::PathBuf;
use std::sync::Arc;

use keynear_core::{OrionQuicNode, TransferEngine};
use keynear_protocol::{
    KeynearError, TransferDirection, TransferItem, TransferSession, TransferState,
};

use crate::callbacks::FfiError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum FfiTransferDirection {
    Send,
    Receive,
}

impl From<TransferDirection> for FfiTransferDirection {
    fn from(value: TransferDirection) -> Self {
        match value {
            TransferDirection::Send => Self::Send,
            TransferDirection::Receive => Self::Receive,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum FfiTransferState {
    PendingConsent,
    Active,
    Completed,
    Cancelled,
    Failed,
}

impl From<TransferState> for FfiTransferState {
    fn from(value: TransferState) -> Self {
        match value {
            TransferState::PendingConsent => Self::PendingConsent,
            TransferState::Active => Self::Active,
            TransferState::Completed => Self::Completed,
            TransferState::Cancelled => Self::Cancelled,
            TransferState::Failed => Self::Failed,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct FfiTransferItem {
    pub name: String,
    pub mime_type: String,
    pub size_bytes: u64,
    pub is_directory: bool,
    pub target_path: Option<String>,
    pub checksum: Option<String>,
}

impl From<TransferItem> for FfiTransferItem {
    fn from(value: TransferItem) -> Self {
        Self {
            name: value.name,
            mime_type: value.mime_type,
            size_bytes: value.size_bytes,
            is_directory: value.is_directory,
            target_path: value.target_path,
            checksum: value.checksum,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct FfiTransferSession {
    pub transfer_id: String,
    pub peer_device_id: String,
    pub direction: FfiTransferDirection,
    pub items: Vec<FfiTransferItem>,
    pub total_bytes: u64,
    pub transferred_bytes: u64,
    pub state: FfiTransferState,
    pub started_at: String,
}

impl From<TransferSession> for FfiTransferSession {
    fn from(value: TransferSession) -> Self {
        Self {
            transfer_id: value.transfer_id,
            peer_device_id: value.peer_device_id,
            direction: FfiTransferDirection::from(value.direction),
            items: value.items.into_iter().map(FfiTransferItem::from).collect(),
            total_bytes: value.total_bytes,
            transferred_bytes: value.transferred_bytes,
            state: FfiTransferState::from(value.state),
            started_at: value.started_at,
        }
    }
}

#[cfg(unix)]
pub(crate) fn wrap_raw_fd(raw_fd: i32) -> Result<OwnedFd, KeynearError> {
    if raw_fd < 0 {
        return Err(KeynearError::Validation {
            field: "rawFd",
            message: "file descriptor must be non-negative".to_string(),
        });
    }
    Ok(unsafe { OwnedFd::from_raw_fd(raw_fd) })
}

pub(crate) fn send_paths_impl(
    runtime: &Arc<tokio::runtime::Runtime>,
    engine: &Arc<TransferEngine>,
    node: &OrionQuicNode,
    peer_device_id: String,
    paths: Vec<String>,
) -> Result<FfiTransferSession, FfiError> {
    let path_bufs: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    let session = runtime.block_on(engine.send_paths(node, &peer_device_id, &path_bufs))?;
    Ok(FfiTransferSession::from(session))
}

#[cfg(unix)]
pub(crate) struct SendFdParams {
    pub peer_device_id: String,
    pub file_name: String,
    pub mime_type: String,
    pub size_bytes: u64,
    pub raw_fd: i32,
}

#[cfg(unix)]
pub(crate) fn send_fd_impl(
    runtime: &Arc<tokio::runtime::Runtime>,
    engine: &Arc<TransferEngine>,
    node: &OrionQuicNode,
    params: SendFdParams,
) -> Result<FfiTransferSession, FfiError> {
    let owned = wrap_raw_fd(params.raw_fd)?;
    let file = File::from(owned);
    let session = runtime.block_on(engine.send_file_handle(
        node,
        &params.peer_device_id,
        params.file_name,
        params.mime_type,
        params.size_bytes,
        file,
    ))?;
    Ok(FfiTransferSession::from(session))
}

#[cfg(unix)]
pub(crate) fn provide_receive_fd_impl(
    engine: &Arc<TransferEngine>,
    transfer_id: String,
    raw_fd: i32,
) -> Result<(), FfiError> {
    let owned = wrap_raw_fd(raw_fd)?;
    let file = File::from(owned);
    engine
        .provide_receive_fd(&transfer_id, file)
        .map_err(FfiError::from)
}
