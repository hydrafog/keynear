use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use keynear_core::{
    remove_paired_device, ClipboardEngine, ClipboardPolicy, MdnsDiscovery, OrionQuicNode,
    PairedDeviceStore, SecretStore, TransferEngine, TransferState,
};
use keynear_ipc::{DaemonStatus, IpcRequest, IpcResponse};
use keynear_protocol::{DaemonConfig, KeynearError, VisibilityMode};
use tokio::sync::{watch, Mutex, Notify};
use tokio::time::Instant;
use tracing::{info, warn};
use zeroize::Zeroizing;

use crate::clipboard::ClipboardMonitorHandle;
use crate::notifications::NotificationAction;
use crate::service::pairing::sync_paired_devices_and_policies;
use crate::tray::{spawn_settings_process, TrayCommand, TrayState};

#[derive(Clone)]
pub(crate) struct IpcDispatchContext {
    pub local_device_id: String,
    pub visibility_tx: watch::Sender<VisibilityMode>,
    pub visibility_rx: watch::Receiver<VisibilityMode>,
    pub config_state: Arc<Mutex<DaemonConfig>>,
    pub config_watch_tx: watch::Sender<DaemonConfig>,
    pub config_path: Option<PathBuf>,
    pub everyone_nearby_timeout: Duration,
    pub everyone_deadline: Arc<Mutex<Option<Instant>>>,
    pub deadline_notify: Arc<Notify>,
    pub refresh_notify: Arc<Notify>,
    pub device_store: PairedDeviceStore,
    pub secret_store: Option<Arc<dyn SecretStore>>,
    pub quic_node: Option<Arc<OrionQuicNode>>,
    pub transfer_engine: Arc<TransferEngine>,
    pub clipboard_engine: Arc<ClipboardEngine>,
    pub clipboard_handle: ClipboardMonitorHandle,
    pub latest_otp: Arc<Mutex<Option<Zeroizing<String>>>>,
    pub known_peers: Arc<Mutex<HashSet<String>>>,
    pub mdns_degraded: bool,
    pub mdns: Option<Arc<MdnsDiscovery>>,
}

async fn sync_ipc_context(ctx: &IpcDispatchContext) -> Result<(), KeynearError> {
    sync_paired_devices_and_policies(
        &ctx.device_store,
        ctx.secret_store.as_deref(),
        ctx.quic_node.as_deref(),
        &ctx.transfer_engine,
        &ctx.known_peers,
    )
    .await
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
    let (endpoint_id, listen_addrs) = match ctx.quic_node {
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
    };
    let mdns_degraded = ctx
        .mdns
        .as_ref()
        .map(|m| m.is_degraded())
        .unwrap_or(ctx.mdns_degraded);
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

pub(crate) async fn apply_visibility_change(mode: VisibilityMode, ctx: &IpcDispatchContext) {
    *ctx.everyone_deadline.lock().await = if mode == VisibilityMode::EveryoneNearby {
        Some(Instant::now() + ctx.everyone_nearby_timeout)
    } else {
        None
    };
    let _ = ctx.visibility_tx.send(mode);
    ctx.deadline_notify.notify_waiters();
    {
        let mut cfg = ctx.config_state.lock().await;
        cfg.visibility_mode = mode;
        if let Some(ref path) = ctx.config_path {
            if let Err(err) = cfg.save_to_path(path) {
                warn!(event = "config_save_failed", error = %err);
            }
        }
    }
    ctx.refresh_notify.notify_waiters();
}

pub(crate) async fn apply_daemon_config(
    new_config: &DaemonConfig,
    ctx: &IpcDispatchContext,
) -> Result<DaemonConfig, KeynearError> {
    new_config.validate()?;
    ctx.transfer_engine
        .set_auto_accept_from_paired(new_config.auto_accept_from_paired);
    ctx.transfer_engine
        .set_incoming_directory(PathBuf::from(&new_config.incoming_directory))?;
    ctx.clipboard_engine
        .set_policy(ClipboardPolicy::from(new_config));
    let _ = ctx.config_watch_tx.send(new_config.clone());
    *ctx.everyone_deadline.lock().await =
        if new_config.visibility_mode == VisibilityMode::EveryoneNearby {
            Some(Instant::now() + ctx.everyone_nearby_timeout)
        } else {
            None
        };
    let _ = ctx.visibility_tx.send(new_config.visibility_mode);
    ctx.deadline_notify.notify_waiters();
    {
        let mut guard = ctx.config_state.lock().await;
        *guard = new_config.clone();
        if let Some(ref path) = ctx.config_path {
            guard.save_to_path(path)?;
        }
    }
    ctx.refresh_notify.notify_waiters();
    Ok(new_config.clone())
}

pub(crate) async fn build_tray_state(ctx: &IpcDispatchContext) -> TrayState {
    let visibility_mode = *ctx.visibility_rx.borrow();
    let deadline = *ctx.everyone_deadline.lock().await;
    let everyone_remaining_secs = compute_remaining_secs(visibility_mode, deadline);
    let auto_accept_from_paired = ctx.config_state.lock().await.auto_accept_from_paired;
    let has_latest_otp = ctx.latest_otp.lock().await.is_some();
    let paired_devices = ctx.device_store.load().unwrap_or_default();
    let transfers = ctx.transfer_engine.list_transfers();
    let mdns_degraded = ctx
        .mdns
        .as_ref()
        .map(|m| m.is_degraded())
        .unwrap_or(ctx.mdns_degraded);
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

pub(crate) async fn handle_tray_command(cmd: TrayCommand, ctx: &IpcDispatchContext) {
    match cmd {
        TrayCommand::SetVisibility(mode) => {
            apply_visibility_change(mode, ctx).await;
        }
        TrayCommand::AcceptTransfer(id) => {
            let _ = ctx.transfer_engine.respond_consent(&id, true);
            ctx.refresh_notify.notify_waiters();
        }
        TrayCommand::DeclineTransfer(id) => {
            let _ = ctx.transfer_engine.respond_consent(&id, false);
            ctx.refresh_notify.notify_waiters();
        }
        TrayCommand::CancelTransfer(id) => {
            let _ = ctx.transfer_engine.cancel_transfer(&id);
            ctx.refresh_notify.notify_waiters();
        }
        TrayCommand::SetAutoAcceptFromPaired(enabled) => {
            let mut next = ctx.config_state.lock().await.clone();
            next.auto_accept_from_paired = enabled;
            if let Err(err) = apply_daemon_config(&next, ctx).await {
                warn!(event = "tray_config_update_failed", error = %err);
            }
        }
        TrayCommand::SetDeviceAutoAccept { device_id, enabled } => {
            match ctx.device_store.set_auto_accept_files(&device_id, enabled) {
                Ok(_) => {
                    let _ = ctx
                        .transfer_engine
                        .set_peer_auto_accept(&device_id, enabled);
                    ctx.refresh_notify.notify_waiters();
                }
                Err(err) => {
                    warn!(event = "tray_device_toggle_failed", error = %err);
                }
            }
        }
        TrayCommand::CopyLatestOtp => {
            let otp = ctx.latest_otp.lock().await.clone();
            if let Some(code) = otp {
                if let Err(err) = ctx
                    .clipboard_handle
                    .write_clipboard_async("text/plain;charset=utf-8", code.as_bytes())
                    .await
                {
                    warn!(event = "copy_latest_otp_failed", error = %err);
                }
            }
        }
        TrayCommand::LaunchSettings => {
            if let Err(err) = spawn_settings_process(&[]) {
                warn!(event = "settings_launch_failed", error = %err);
            }
        }
        TrayCommand::LaunchSendPicker { device_id } => {
            let mut args = vec!["--send-picker".to_string()];
            if let Some(id) = device_id {
                args.push("--device".to_string());
                args.push(id);
            }
            if let Err(err) = spawn_settings_process(&args) {
                warn!(event = "send_picker_launch_failed", error = %err);
            }
        }
    }
}

pub(crate) async fn handle_notification_action(
    action: NotificationAction,
    ctx: &IpcDispatchContext,
) {
    match action {
        NotificationAction::AcceptTransfer(id) => {
            let _ = ctx.transfer_engine.respond_consent(&id, true);
            ctx.refresh_notify.notify_waiters();
        }
        NotificationAction::DeclineTransfer(id) => {
            let _ = ctx.transfer_engine.respond_consent(&id, false);
            ctx.refresh_notify.notify_waiters();
        }
        NotificationAction::CancelTransfer(id) => {
            let _ = ctx.transfer_engine.cancel_transfer(&id);
            ctx.refresh_notify.notify_waiters();
        }
        NotificationAction::ApprovePairing(peer_id) => {
            info!(event = "pairing_notification_approve", peer_device_id = %peer_id);
        }
        NotificationAction::RejectPairing(peer_id) => {
            info!(event = "pairing_notification_reject", peer_device_id = %peer_id);
        }
    }
}

pub(crate) async fn dispatch_ipc_request(req: IpcRequest, ctx: &IpcDispatchContext) -> IpcResponse {
    match req {
        IpcRequest::Status => match build_daemon_status(ctx).await {
            Ok(status) => IpcResponse::Status(status),
            Err(err) => IpcResponse::Error {
                message: err.to_string(),
            },
        },
        IpcRequest::ListTransfers => IpcResponse::Transfers(ctx.transfer_engine.list_transfers()),
        IpcRequest::ListDevices => match ctx.device_store.load() {
            Ok(devices) => IpcResponse::Devices(devices),
            Err(err) => IpcResponse::Error {
                message: err.to_string(),
            },
        },
        IpcRequest::SetVisibility { mode } => {
            apply_visibility_change(mode, ctx).await;
            match build_daemon_status(ctx).await {
                Ok(status) => IpcResponse::Status(status),
                Err(err) => IpcResponse::Error {
                    message: err.to_string(),
                },
            }
        }
        IpcRequest::GetConfig => IpcResponse::Config(ctx.config_state.lock().await.clone()),
        IpcRequest::UpdateConfig { config } => match apply_daemon_config(&config, ctx).await {
            Ok(updated) => IpcResponse::Config(updated),
            Err(err) => IpcResponse::Error {
                message: err.to_string(),
            },
        },
        IpcRequest::RenameDevice {
            device_id,
            display_name,
        } => match ctx.device_store.rename(&device_id, &display_name) {
            Ok(updated) => {
                ctx.refresh_notify.notify_waiters();
                IpcResponse::Device(updated)
            }
            Err(err) => IpcResponse::Error {
                message: err.to_string(),
            },
        },
        IpcRequest::SetDeviceAutoAccept { device_id, enabled } => {
            match ctx.device_store.set_auto_accept_files(&device_id, enabled) {
                Ok(updated) => {
                    let _ = ctx
                        .transfer_engine
                        .set_peer_auto_accept(&device_id, enabled);
                    ctx.refresh_notify.notify_waiters();
                    IpcResponse::Device(updated)
                }
                Err(err) => IpcResponse::Error {
                    message: err.to_string(),
                },
            }
        }
        IpcRequest::UnpairDevice { device_id } => {
            let remove_result = match ctx.secret_store {
                Some(ref secrets) => {
                    remove_paired_device(&ctx.device_store, secrets.as_ref(), &device_id)
                }
                None => ctx.device_store.remove(&device_id),
            };
            match remove_result {
                Ok(_) => {
                    if let Err(err) = sync_ipc_context(ctx).await {
                        return IpcResponse::Error {
                            message: err.to_string(),
                        };
                    }
                    ctx.refresh_notify.notify_waiters();
                    IpcResponse::Unpaired { device_id }
                }
                Err(err) => IpcResponse::Error {
                    message: err.to_string(),
                },
            }
        }
        IpcRequest::AcceptTransfer { transfer_id } => {
            match ctx.transfer_engine.respond_consent(&transfer_id, true) {
                Ok(session) => {
                    ctx.refresh_notify.notify_waiters();
                    IpcResponse::Transfer(session)
                }
                Err(err) => IpcResponse::Error {
                    message: err.to_string(),
                },
            }
        }
        IpcRequest::DeclineTransfer { transfer_id } => {
            match ctx.transfer_engine.respond_consent(&transfer_id, false) {
                Ok(session) => {
                    ctx.refresh_notify.notify_waiters();
                    IpcResponse::Transfer(session)
                }
                Err(err) => IpcResponse::Error {
                    message: err.to_string(),
                },
            }
        }
        IpcRequest::CancelTransfer { transfer_id } => {
            match ctx.transfer_engine.cancel_transfer(&transfer_id) {
                Ok(session) => {
                    ctx.refresh_notify.notify_waiters();
                    IpcResponse::Transfer(session)
                }
                Err(err) => IpcResponse::Error {
                    message: err.to_string(),
                },
            }
        }
        IpcRequest::SendFiles { device_id, paths } => {
            if let Err(err) = sync_ipc_context(ctx).await {
                return IpcResponse::Error {
                    message: err.to_string(),
                };
            }
            let Some(ref node) = ctx.quic_node else {
                return IpcResponse::Error {
                    message: "QUIC transport is unavailable (keyring degraded)".to_string(),
                };
            };
            match ctx
                .transfer_engine
                .send_paths(node, &device_id, &paths)
                .await
            {
                Ok(session) => {
                    ctx.refresh_notify.notify_waiters();
                    IpcResponse::Transfer(session)
                }
                Err(err) => IpcResponse::Error {
                    message: err.to_string(),
                },
            }
        }
    }
}
