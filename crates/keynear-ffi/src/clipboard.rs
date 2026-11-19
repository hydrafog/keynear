use std::sync::{Arc, Mutex};

use keynear_core::{ClipboardEngine, ClipboardItem, ClipboardPolicy, OrionQuicNode};
use keynear_protocol::KeynearError;

use crate::callbacks::FfiError;

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct FfiClipboardItem {
    pub id: String,
    pub peer_device_id: String,
    pub mime_type: String,
    pub data: Vec<u8>,
    pub is_otp: bool,
    pub is_image_offer: bool,
    pub offer_id: String,
    pub offered_mimes: Vec<String>,
}

impl From<ClipboardItem> for FfiClipboardItem {
    fn from(value: ClipboardItem) -> Self {
        Self {
            id: value.id,
            peer_device_id: value.peer_device_id,
            mime_type: value.mime_type,
            data: value.data,
            is_otp: value.is_otp,
            is_image_offer: value.is_image_offer,
            offer_id: value.offer_id,
            offered_mimes: value.offered_mimes,
        }
    }
}

pub(crate) fn set_clipboard_config_impl(
    clipboard: &Arc<ClipboardEngine>,
    clipboard_sync_enabled: bool,
    clipboard_images_enabled: bool,
    otp_passthrough_enabled: bool,
    ignored_mime_types: Vec<String>,
) {
    clipboard.set_policy(ClipboardPolicy {
        clipboard_sync_enabled,
        clipboard_images_enabled,
        otp_passthrough_enabled,
        ignored_mime_types,
    });
}

pub(crate) fn send_clipboard_text_impl(
    runtime: &Arc<tokio::runtime::Runtime>,
    clipboard: &Arc<ClipboardEngine>,
    node: &OrionQuicNode,
    synced_peers: &Mutex<Vec<String>>,
    text: String,
    is_otp: bool,
) -> Result<(), FfiError> {
    let peers = synced_peers
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .clone();
    if peers.is_empty() {
        return Err(FfiError::Transport {
            detail: "no paired peers synchronized".to_string(),
        });
    }
    let mut delivered = 0usize;
    let mut last_err: Option<KeynearError> = None;
    for peer_id in &peers {
        match runtime.block_on(clipboard.send_clipboard_text(node, peer_id, &text, is_otp)) {
            Ok(()) => delivered += 1,
            Err(err) => last_err = Some(err),
        }
    }
    if delivered > 0 {
        Ok(())
    } else if let Some(err) = last_err {
        Err(FfiError::from(err))
    } else {
        Ok(())
    }
}

pub(crate) fn send_clipboard_blob_impl(
    runtime: &Arc<tokio::runtime::Runtime>,
    clipboard: &Arc<ClipboardEngine>,
    node: &OrionQuicNode,
    synced_peers: &Mutex<Vec<String>>,
    mime_type: String,
    data: Vec<u8>,
) -> Result<(), FfiError> {
    let peers = synced_peers
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .clone();
    if peers.is_empty() {
        return Err(FfiError::Transport {
            detail: "no paired peers synchronized".to_string(),
        });
    }
    let mut delivered = 0usize;
    let mut last_err: Option<KeynearError> = None;
    for peer_id in &peers {
        match runtime
            .block_on(clipboard.send_clipboard_payload(node, peer_id, &mime_type, &data, false))
        {
            Ok(()) => delivered += 1,
            Err(err) => last_err = Some(err),
        }
    }
    if delivered > 0 {
        Ok(())
    } else if let Some(err) = last_err {
        Err(FfiError::from(err))
    } else {
        Ok(())
    }
}

pub(crate) fn poll_incoming_clipboard_impl(
    clipboard: &Arc<ClipboardEngine>,
) -> Vec<FfiClipboardItem> {
    clipboard
        .poll_incoming()
        .into_iter()
        .map(FfiClipboardItem::from)
        .collect()
}

pub(crate) fn fetch_clipboard_offer_impl(
    runtime: &Arc<tokio::runtime::Runtime>,
    clipboard: &Arc<ClipboardEngine>,
    node: &OrionQuicNode,
    peer_device_id: String,
    offer_id: String,
    mime_type: String,
) -> Result<FfiClipboardItem, FfiError> {
    let item = runtime.block_on(clipboard.fetch_clipboard_image(
        node,
        &peer_device_id,
        &offer_id,
        &mime_type,
    ))?;
    Ok(FfiClipboardItem::from(item))
}
