use std::sync::Arc;

use keynear_core::{
    ClipboardEngine, ClipboardItem, DelayedClipboardOffer, DelayedRenderCallback, OrionQuicNode,
    PairedDeviceStore,
};
use keynear_protocol::KeynearError;
use tokio::sync::{mpsc, Mutex, Notify};
use tokio::task::JoinHandle;
use tracing::warn;
use zeroize::Zeroizing;

use crate::clipboard::{ClipboardChange, ClipboardMonitorHandle};
use crate::service::clipboard_bridge::ClipboardBridgeProxy;

fn probe_clipboard_bridge(tasks: &mut Vec<JoinHandle<()>>) {
    tasks.push(tokio::spawn(async {
        let Some(proxy) = ClipboardBridgeProxy::connect().await else {
            tracing::debug!(event = "clipboard_bridge_unavailable");
            return;
        };
        if let Err(err) = proxy.sync().await {
            warn!(event = "clipboard_bridge_sync_failed", error = %err);
        }
        match proxy.status().await {
            Ok(status) => tracing::info!(event = "clipboard_bridge_status", status = %status),
            Err(err) => warn!(event = "clipboard_bridge_status_failed", error = %err),
        }
    }));
}

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

struct ClipboardForwardCtx<'a> {
    node: &'a OrionQuicNode,
    clip: &'a ClipboardEngine,
    store: &'a PairedDeviceStore,
}

fn warn_clipboard_send(err: KeynearError, event: &str, peer_device_id: &str) {
    if !matches!(err, KeynearError::UnauthorizedPeer(_)) {
        warn!(event = event, peer_device_id = peer_device_id, error = %err);
    }
}

fn warn_image_fetch_failed(err: &KeynearError) {
    warn!(event = "clipboard_fetch_image_failed", error = %err);
}

fn warn_otp_apply_failed(err: &KeynearError) {
    warn!(event = "clipboard_apply_otp_failed", error = %err);
}

async fn forward_text_change(ctx: &ClipboardForwardCtx<'_>, mime_type: &str, text: &[u8]) {
    for peer in ctx.store.load().unwrap_or_default() {
        if let Err(err) = ctx
            .clip
            .send_clipboard_payload(ctx.node, &peer.device_id, mime_type, text, false)
            .await
        {
            warn_clipboard_send(err, "clipboard_send_failed", &peer.device_id);
        }
    }
}

async fn send_offer_to_peers(ctx: &ClipboardForwardCtx<'_>, offer: &DelayedClipboardOffer) {
    for peer in ctx.store.load().unwrap_or_default() {
        if let Err(err) = ctx
            .clip
            .send_delayed_offer(ctx.node, &peer.device_id, offer)
            .await
        {
            warn_clipboard_send(err, "clipboard_offer_send_failed", &peer.device_id);
        }
    }
}

async fn forward_image_offer(
    ctx: &ClipboardForwardCtx<'_>,
    offered_mimes: Vec<String>,
    render: DelayedRenderCallback,
) {
    let offer = match ctx
        .clip
        .register_delayed_offer(offered_mimes, true, false, 0, render)
    {
        Ok(offer) => offer,
        Err(err) => {
            warn!(event = "clipboard_offer_rejected", error = %err);
            return;
        }
    };
    send_offer_to_peers(ctx, &offer).await;
}

async fn run_clipboard_out_loop(
    node: Arc<OrionQuicNode>,
    clip: Arc<ClipboardEngine>,
    store: PairedDeviceStore,
    mut clip_change_rx: mpsc::UnboundedReceiver<ClipboardChange>,
) {
    let ctx = ClipboardForwardCtx {
        node: &node,
        clip: &clip,
        store: &store,
    };
    while let Some(change) = clip_change_rx.recv().await {
        match change {
            ClipboardChange::Text { mime_type, text } => {
                forward_text_change(&ctx, &mime_type, text.as_bytes()).await;
            }
            ClipboardChange::ImageOffer {
                offered_mimes,
                render,
            } => {
                forward_image_offer(&ctx, offered_mimes, render).await;
            }
        }
    }
}

pub(crate) fn spawn_clipboard_loops(
    params: ClipboardLoopParams<'_>,
    tasks: &mut Vec<JoinHandle<()>>,
) {
    probe_clipboard_bridge(tasks);
    let ClipboardLoopParams {
        quic_node,
        clipboard_engine,
        device_store,
        clipboard_handle,
        latest_otp,
        refresh_notify,
        clip_change_rx,
        mut incoming_clip_rx,
    } = params;
    if let Some(node) = quic_node {
        let out_node = Arc::clone(node);
        let out_clip = Arc::clone(clipboard_engine);
        let out_store = device_store.clone();
        tasks.push(tokio::spawn(run_clipboard_out_loop(
            out_node,
            out_clip,
            out_store,
            clip_change_rx,
        )));
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

fn clipboard_images_sync_enabled(clipboard_engine: &ClipboardEngine) -> bool {
    let policy = clipboard_engine.policy();
    policy.clipboard_sync_enabled && policy.clipboard_images_enabled
}

async fn write_fetched_image(handle: &ClipboardMonitorHandle, fetched: &ClipboardItem) {
    if let Err(err) = handle
        .write_clipboard_async(&fetched.mime_type, &fetched.data)
        .await
    {
        warn!(event = "clipboard_apply_image_failed", error = %err);
    }
}

fn image_sync_node<'a>(
    clipboard_engine: &ClipboardEngine,
    quic_node: Option<&'a OrionQuicNode>,
) -> Option<&'a OrionQuicNode> {
    if !clipboard_images_sync_enabled(clipboard_engine) {
        return None;
    }
    quic_node
}

async fn fetch_and_write_image(
    clipboard_engine: &ClipboardEngine,
    node: &OrionQuicNode,
    item: &ClipboardItem,
    clipboard_handle: &ClipboardMonitorHandle,
) {
    match clipboard_engine
        .fetch_clipboard_image(node, &item.peer_device_id, &item.offer_id, &item.mime_type)
        .await
    {
        Ok(fetched) => write_fetched_image(clipboard_handle, &fetched).await,
        Err(err) => warn_image_fetch_failed(&err),
    }
}

async fn apply_image_offer(
    item: ClipboardItem,
    clipboard_engine: &ClipboardEngine,
    quic_node: Option<&OrionQuicNode>,
    clipboard_handle: &ClipboardMonitorHandle,
) {
    let Some(node) = image_sync_node(clipboard_engine, quic_node) else {
        return;
    };
    fetch_and_write_image(clipboard_engine, node, &item, clipboard_handle).await;
}

fn otp_code(item: &ClipboardItem) -> Option<&str> {
    if !item.is_otp {
        return None;
    }
    let code = std::str::from_utf8(&item.data).ok()?;
    let trimmed = code.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed)
}

async fn store_and_write_otp(
    code: &str,
    clipboard_handle: &ClipboardMonitorHandle,
    latest_otp: &Mutex<Option<Zeroizing<String>>>,
    refresh_notify: &Notify,
) {
    let mut guard = latest_otp.lock().await;
    *guard = Some(Zeroizing::new(code.to_string()));
    drop(guard);
    refresh_notify.notify_waiters();
    if let Err(err) = clipboard_handle
        .write_clipboard_async("text/plain;charset=utf-8", code.as_bytes())
        .await
    {
        warn_otp_apply_failed(&err);
    }
}

async fn try_apply_otp(
    item: &ClipboardItem,
    clipboard_handle: &ClipboardMonitorHandle,
    latest_otp: &Mutex<Option<Zeroizing<String>>>,
    refresh_notify: &Notify,
) -> bool {
    let Some(code) = otp_code(item) else {
        return false;
    };
    store_and_write_otp(code, clipboard_handle, latest_otp, refresh_notify).await;
    true
}

async fn write_incoming_item(handle: &ClipboardMonitorHandle, item: &ClipboardItem) {
    if let Err(err) = handle
        .write_clipboard_async(&item.mime_type, &item.data)
        .await
    {
        warn!(event = "clipboard_apply_failed", error = %err);
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn apply_incoming_clipboard_item(
    item: ClipboardItem,
    clipboard_engine: &ClipboardEngine,
    quic_node: Option<&OrionQuicNode>,
    clipboard_handle: &ClipboardMonitorHandle,
    latest_otp: &Mutex<Option<Zeroizing<String>>>,
    refresh_notify: &Notify,
) {
    if item.is_image_offer {
        apply_image_offer(item, clipboard_engine, quic_node, clipboard_handle).await;
        return;
    }
    if try_apply_otp(&item, clipboard_handle, latest_otp, refresh_notify).await {
        return;
    }
    write_incoming_item(clipboard_handle, &item).await;
}
