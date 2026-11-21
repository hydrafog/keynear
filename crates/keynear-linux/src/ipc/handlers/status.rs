use keynear_core::TransferState;
use keynear_ipc::DaemonStatus;
use keynear_protocol::{KeynearError, VisibilityMode};
use tokio::time::Instant;

use crate::service::pairing::sync_paired_devices_and_policies;
use crate::tray::TrayState;

use super::IpcDispatchContext;

pub(crate) async fn sync_ipc_context(ctx: &IpcDispatchContext) -> Result<(), KeynearError> {
    sync_paired_devices_and_policies(
        &ctx.device_store,
        ctx.secret_store.as_deref(),
        ctx.quic_node.as_deref(),
        &ctx.transfer_engine,
        &ctx.known_peers,
    )
    .await
}

fn endpoint_info(ctx: &IpcDispatchContext) -> (String, Vec<String>) {
    match ctx.quic_node {
        Some(ref node) => {
            let eid = hex::encode(node.endpoint_id().as_bytes());
            let addrs = node
                .endpoint_addr()
                .ip_addrs()
                .map(ToString::to_string)
                .collect();
            (eid, addrs)
        }
        None => (String::new(), Vec::new()),
    }
}

fn current_mdns_degraded(ctx: &IpcDispatchContext) -> bool {
    ctx.mdns
        .as_ref()
        .map(|m| m.is_degraded())
        .unwrap_or(ctx.mdns_degraded)
}

pub(crate) async fn build_daemon_status(
    ctx: &IpcDispatchContext,
) -> Result<DaemonStatus, KeynearError> {
    sync_ipc_context(ctx).await?;
    let devices = ctx.device_store.load()?;
    let transfers = ctx.transfer_engine.list_transfers();
    let active_count = transfers
        .iter()
        .filter(|t| {
            matches!(
                t.state,
                TransferState::PendingConsent | TransferState::Active
            )
        })
        .count() as u32;
    let (endpoint_id, listen_addrs) = endpoint_info(ctx);
    let mdns_degraded = current_mdns_degraded(ctx);
    let visibility_mode = *ctx.visibility_rx.borrow();
    let deadline = *ctx.everyone_deadline.lock().await;
    let everyone_remaining_secs = compute_remaining_secs(visibility_mode, deadline);
    let cfg = ctx.config_state.lock().await.clone();
    Ok(DaemonStatus {
        device_id: ctx.local_device_id.clone(),
        endpoint_id,
        listen_addrs,
        visibility_mode,
        mdns_degraded,
        active_transfer_count: active_count,
        paired_device_count: devices.len() as u32,
        everyone_remaining_secs,
        auto_accept_from_paired: cfg.auto_accept_from_paired,
        incoming_directory: cfg.incoming_directory,
    })
}

pub(crate) fn compute_remaining_secs(
    mode: VisibilityMode,
    deadline: Option<Instant>,
) -> Option<u64> {
    if mode != VisibilityMode::EveryoneNearby {
        return None;
    }
    deadline.map(|target| {
        let now = Instant::now();
        if target > now {
            target.saturating_duration_since(now).as_secs()
        } else {
            0
        }
    })
}

pub(crate) async fn build_tray_state(ctx: &IpcDispatchContext) -> TrayState {
    let visibility_mode = *ctx.visibility_rx.borrow();
    let deadline = *ctx.everyone_deadline.lock().await;
    let everyone_remaining_secs = compute_remaining_secs(visibility_mode, deadline);
    let auto_accept_from_paired = ctx.config_state.lock().await.auto_accept_from_paired;
    let has_latest_otp = ctx.latest_otp.lock().await.is_some();
    let paired_devices = ctx.device_store.load().unwrap_or_default();
    let transfers = ctx.transfer_engine.list_transfers();
    let mdns_degraded = current_mdns_degraded(ctx);
    TrayState {
        visibility_mode,
        everyone_remaining_secs,
        auto_accept_from_paired,
        has_latest_otp,
        paired_devices,
        transfers,
        mdns_degraded,
    }
}
