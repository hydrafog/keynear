use std::path::PathBuf;

use keynear_core::remove_paired_device;
use keynear_ipc::{IpcRequest, IpcResponse};
use keynear_protocol::KeynearError;

use super::config::{apply_daemon_config, apply_visibility_change};
use super::status::{build_daemon_status, sync_ipc_context};
use super::IpcDispatchContext;

fn error_response(err: &KeynearError) -> IpcResponse {
    IpcResponse::Error {
        message: err.to_string(),
    }
}

async fn status_response(ctx: &IpcDispatchContext) -> IpcResponse {
    match build_daemon_status(ctx).await {
        Ok(status) => IpcResponse::Status(status),
        Err(err) => error_response(&err),
    }
}

async fn visibility_response(
    mode: keynear_protocol::VisibilityMode,
    ctx: &IpcDispatchContext,
) -> IpcResponse {
    apply_visibility_change(mode, ctx).await;
    status_response(ctx).await
}

fn rename_response(
    device_id: String,
    display_name: String,
    ctx: &IpcDispatchContext,
) -> IpcResponse {
    match ctx.device_store.rename(&device_id, &display_name) {
        Ok(updated) => {
            ctx.refresh_notify.notify_waiters();
            IpcResponse::Device(updated)
        }
        Err(err) => error_response(&err),
    }
}

fn set_device_auto_accept_response(
    device_id: String,
    enabled: bool,
    ctx: &IpcDispatchContext,
) -> IpcResponse {
    match ctx.device_store.set_auto_accept_files(&device_id, enabled) {
        Ok(updated) => {
            let _ = ctx
                .transfer_engine
                .set_peer_auto_accept(&device_id, enabled);
            ctx.refresh_notify.notify_waiters();
            IpcResponse::Device(updated)
        }
        Err(err) => error_response(&err),
    }
}

pub(crate) async fn unpair_device(
    device_id: &str,
    ctx: &IpcDispatchContext,
) -> Result<(), KeynearError> {
    let remove_result = match ctx.secret_store {
        Some(ref secrets) => remove_paired_device(&ctx.device_store, secrets.as_ref(), device_id),
        None => ctx.device_store.remove(device_id),
    };
    remove_result?;
    sync_ipc_context(ctx).await?;
    ctx.refresh_notify.notify_waiters();
    Ok(())
}

async fn unpair_response(device_id: String, ctx: &IpcDispatchContext) -> IpcResponse {
    match unpair_device(&device_id, ctx).await {
        Ok(()) => IpcResponse::Unpaired { device_id },
        Err(err) => error_response(&err),
    }
}

fn consent_response(transfer_id: String, accept: bool, ctx: &IpcDispatchContext) -> IpcResponse {
    match ctx.transfer_engine.respond_consent(&transfer_id, accept) {
        Ok(session) => {
            ctx.refresh_notify.notify_waiters();
            IpcResponse::Transfer(session)
        }
        Err(err) => error_response(&err),
    }
}

fn cancel_response(transfer_id: String, ctx: &IpcDispatchContext) -> IpcResponse {
    match ctx.transfer_engine.cancel_transfer(&transfer_id) {
        Ok(session) => {
            ctx.refresh_notify.notify_waiters();
            IpcResponse::Transfer(session)
        }
        Err(err) => error_response(&err),
    }
}

async fn send_files_response(
    device_id: String,
    paths: Vec<PathBuf>,
    ctx: &IpcDispatchContext,
) -> IpcResponse {
    if let Err(err) = sync_ipc_context(ctx).await {
        return error_response(&err);
    }
    let Some(ref node) = ctx.quic_node else {
        return error_response(&KeynearError::Transport(
            "QUIC transport is unavailable (keyring degraded)".to_string(),
        ));
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
        Err(err) => error_response(&err),
    }
}

fn respond_pairing_response(approve: bool, ctx: &IpcDispatchContext) -> IpcResponse {
    let Some(ref tx) = ctx.pairing_decision_tx else {
        return error_response(&KeynearError::Bluetooth(
            "BLE pairing is unavailable".to_string(),
        ));
    };
    match tx.try_send(approve) {
        Ok(()) => IpcResponse::PairingDecision { approved: approve },
        Err(err) => error_response(&KeynearError::Bluetooth(format!(
            "failed to deliver pairing decision: {err}"
        ))),
    }
}

pub(crate) async fn dispatch_ipc_request(req: IpcRequest, ctx: &IpcDispatchContext) -> IpcResponse {
    match req {
        IpcRequest::Status => status_response(ctx).await,
        IpcRequest::ListTransfers => IpcResponse::Transfers(ctx.transfer_engine.list_transfers()),
        IpcRequest::ListDevices => match ctx.device_store.load() {
            Ok(devices) => IpcResponse::Devices(devices),
            Err(err) => error_response(&err),
        },
        IpcRequest::SetVisibility { mode } => visibility_response(mode, ctx).await,
        IpcRequest::GetConfig => IpcResponse::Config(ctx.config_state.lock().await.clone()),
        IpcRequest::UpdateConfig { config } => match apply_daemon_config(&config, ctx).await {
            Ok(updated) => IpcResponse::Config(updated),
            Err(err) => error_response(&err),
        },
        IpcRequest::RenameDevice {
            device_id,
            display_name,
        } => rename_response(device_id, display_name, ctx),
        IpcRequest::SetDeviceAutoAccept { device_id, enabled } => {
            set_device_auto_accept_response(device_id, enabled, ctx)
        }
        IpcRequest::UnpairDevice { device_id } => unpair_response(device_id, ctx).await,
        IpcRequest::AcceptTransfer { transfer_id } => consent_response(transfer_id, true, ctx),
        IpcRequest::DeclineTransfer { transfer_id } => consent_response(transfer_id, false, ctx),
        IpcRequest::CancelTransfer { transfer_id } => cancel_response(transfer_id, ctx),
        IpcRequest::SendFiles { device_id, paths } => {
            send_files_response(device_id, paths, ctx).await
        }
        IpcRequest::RespondPairing { approve } => respond_pairing_response(approve, ctx),
    }
}
