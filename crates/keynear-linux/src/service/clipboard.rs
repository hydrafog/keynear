use std::sync::Arc;

use keynear_core::{ClipboardEngine, ClipboardItem, OrionQuicNode, PairedDeviceStore};
use keynear_protocol::KeynearError;
use tokio::sync::{mpsc, Mutex, Notify};
use tokio::task::JoinHandle;
use tracing::warn;
use zeroize::Zeroizing;

use crate::clipboard::{ClipboardChange, ClipboardMonitorHandle};

pub(crate) struct ClipboardLoopParams<'a> {
    pub(crate) quic_node: Option<&'a Arc<OrionQuicNode>>,
    pub(crate) clipboard_engine: &'a Arc<ClipboardEngine>,
    pub(crate) device_store: &'a PairedDeviceStore,
    pub(crate) clipboard_handle: &'a ClipboardMonitorHandle,
    pub(crate) latest_otp: &'a Arc<Mutex<Option<Zeroizing<String>>>>,
    pub(crate) refresh_notify: &'a Arc<Notify>,
    pub(crate) clip_change_rx: mpsc::UnboundedReceiver<ClipboardChange>,
    pub(crate) incoming_clip_rx: mpsc::UnboundedReceiver<ClipboardItem>,
}

pub(crate) fn spawn_clipboard_loops(
    params: ClipboardLoopParams<'_>,
    tasks: &mut Vec<JoinHandle<()>>,
) {
    let ClipboardLoopParams {
        quic_node,
        clipboard_engine,
        device_store,
        clipboard_handle,
        latest_otp,
        refresh_notify,
        mut clip_change_rx,
        mut incoming_clip_rx,
    } = params;
    if let Some(node) = quic_node {
        let out_node = Arc::clone(node);
        let out_clip = Arc::clone(clipboard_engine);
        let out_store = device_store.clone();
        tasks.push(tokio::spawn(async move {
            while let Some(change) = clip_change_rx.recv().await {
                let peers = out_store.load().unwrap_or_default();
                match change {
                    ClipboardChange::Text { mime_type, text } => {
                        for peer in peers {
                            if let Err(err) = out_clip
                                .send_clipboard_payload(
                                    &out_node,
                                    &peer.device_id,
                                    &mime_type,
                                    text.as_bytes(),
                                    false,
                                )
                                .await
                            {
                                if !matches!(err, KeynearError::UnauthorizedPeer(_)) {
                                    warn!(
                                        event = "clipboard_send_failed",
                                        peer_device_id = %peer.device_id,
                                        error = %err
                                    );
                                }
                            }
                        }
                    }
                    ClipboardChange::ImageOffer {
                        offered_mimes,
                        render,
                    } => {
                        match out_clip.register_delayed_offer(offered_mimes, true, false, 0, render)
                        {
                            Ok(offer) => {
                                for peer in peers {
                                    if let Err(err) = out_clip
                                        .send_delayed_offer(&out_node, &peer.device_id, &offer)
                                        .await
                                    {
                                        if !matches!(err, KeynearError::UnauthorizedPeer(_)) {
                                            warn!(
                                                event = "clipboard_offer_send_failed",
                                                peer_device_id = %peer.device_id,
                                                error = %err
                                            );
                                        }
                                    }
                                }
                            }
                            Err(err) => {
                                warn!(event = "clipboard_offer_rejected", error = %err);
                            }
                        }
                    }
                }
            }
        }));
    }

    let in_clip_engine = Arc::clone(clipboard_engine);
    let in_node = quic_node.cloned();
    let in_handle = clipboard_handle.clone();
    let in_otp = Arc::clone(latest_otp);
    let in_refresh = Arc::clone(refresh_notify);
    tasks.push(tokio::spawn(async move {
        while let Some(item) = incoming_clip_rx.recv().await {
            apply_incoming_clipboard_item(
                item,
                &in_clip_engine,
                in_node.as_deref(),
                &in_handle,
                &in_otp,
                &in_refresh,
            )
            .await;
        }
    }));
}

pub(crate) async fn apply_incoming_clipboard_item(
    item: ClipboardItem,
    clipboard_engine: &ClipboardEngine,
    quic_node: Option<&OrionQuicNode>,
    clipboard_handle: &ClipboardMonitorHandle,
    latest_otp: &Mutex<Option<Zeroizing<String>>>,
    refresh_notify: &Notify,
) {
    if item.is_image_offer {
        let policy = clipboard_engine.policy();
        if !policy.clipboard_sync_enabled || !policy.clipboard_images_enabled {
            return;
        }
        let Some(node) = quic_node else {
            return;
        };
        match clipboard_engine
            .fetch_clipboard_image(node, &item.peer_device_id, &item.offer_id, &item.mime_type)
            .await
        {
            Ok(fetched) => {
                if let Err(err) = clipboard_handle
                    .write_clipboard_async(&fetched.mime_type, &fetched.data)
                    .await
                {
                    warn!(event = "clipboard_apply_image_failed", error = %err);
                }
            }
            Err(err) => {
                warn!(event = "clipboard_fetch_image_failed", error = %err);
            }
        }
        return;
    }

    if item.is_otp {
        if let Ok(code) = std::str::from_utf8(&item.data) {
            let trimmed = code.trim();
            if !trimmed.is_empty() {
                let mut guard = latest_otp.lock().await;
                *guard = Some(Zeroizing::new(trimmed.to_string()));
                drop(guard);
                refresh_notify.notify_waiters();
                if let Err(err) = clipboard_handle
                    .write_clipboard_async("text/plain;charset=utf-8", trimmed.as_bytes())
                    .await
                {
                    warn!(event = "clipboard_apply_otp_failed", error = %err);
                }
                return;
            }
        }
    }

    if let Err(err) = clipboard_handle
        .write_clipboard_async(&item.mime_type, &item.data)
        .await
    {
        warn!(event = "clipboard_apply_failed", error = %err);
    }
}
