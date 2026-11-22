fn log_config_update(err: KeynearError) {
    tracing::warn!(event = "tray_config_update_failed", error = %err);
}

fn log_device_toggle(err: KeynearError) {
    tracing::warn!(event = "tray_device_toggle_failed", error = %err);
}

fn log_otp_copy(err: KeynearError) {
    tracing::warn!(event = "copy_latest_otp_failed", error = %err);
}

fn log_send_picker_launch(err: std::io::Error) {
    tracing::warn!(event = "send_picker_launch_failed", error = %err);
}

fn log_pairing_approve(peer_id: &str) {
    tracing::info!(event = "pairing_notification_approve", peer_device_id = %peer_id);
}

fn log_pairing_reject(peer_id: &str) {
    tracing::info!(event = "pairing_notification_reject", peer_device_id = %peer_id);
}

use keynear_core::{encode_envelope, v1, CallActionKind, MediaCommandKind};
use keynear_protocol::KeynearError;
use tracing::{info, warn};

use crate::notifications::NotificationAction;
use crate::tray::{spawn_send_picker_process, TrayCommand};

use super::config::apply_daemon_config;
use super::IpcDispatchContext;

fn log_outbound_missing(label: &'static str) {
    warn!(event = "continuity_outbound_queue_missing", kind = label);
}

fn log_outbound_failure(label: &'static str, err: keynear_protocol::KeynearError) {
    warn!(
        event = "continuity_outbound_action_failed",
        kind = label,
        error = %err
    );
}

fn emit_envelope(payload: v1::envelope::Payload, label: &'static str, ctx: &IpcDispatchContext) {
    let envelope = v1::Envelope {
        version: 1,
        sequence: 0,
        timestamp_ms: 0,
        payload: Some(payload),
    };
    match encode_envelope(&envelope) {
        Ok(bytes) => match ctx.outbound_queue() {
            Some(queue) => queue.enqueue(label, bytes),
            None => log_outbound_missing(label),
        },
        Err(err) => log_outbound_failure(label, err),
    }
}

fn emit_mirror_action(
    key: String,
    kind: v1::NotificationActionKind,
    reply_text: String,
    ctx: &IpcDispatchContext,
) {
    let label = match kind {
        v1::NotificationActionKind::NotificationActionReply => "reply",
        v1::NotificationActionKind::NotificationActionMarkRead => "mark_read",
        _ => "dismiss",
    };
    emit_envelope(
        v1::envelope::Payload::NotificationAction(v1::NotificationAction {
            notification_key: key,
            kind: kind as i32,
            reply_text,
            sequence: 0,
        }),
        label,
        ctx,
    );
}

fn emit_call_action(call_id: String, kind: v1::CallActionKind, ctx: &IpcDispatchContext) {
    let label = match kind {
        v1::CallActionKind::CallActionAnswer => "answer",
        v1::CallActionKind::CallActionDecline => "decline",
        _ => "mute",
    };
    emit_envelope(
        v1::envelope::Payload::CallAction(v1::CallAction {
            call_id,
            kind: kind as i32,
            sequence: 0,
        }),
        label,
        ctx,
    );
}

async fn handle_media_tray(cmd: &TrayCommand) -> bool {
    let kind = match cmd {
        TrayCommand::MediaPlay => MediaCommandKind::Play,
        TrayCommand::MediaPause => MediaCommandKind::Pause,
        TrayCommand::MediaNext => MediaCommandKind::Next,
        _ => return false,
    };
    let delivered = crate::media::dispatch_command(kind).await;
    info!(event = "media_command_dispatched", players = delivered);
    true
}

async fn set_auto_accept_from_paired(enabled: bool, ctx: &IpcDispatchContext) {
    let mut next = ctx.config_state.lock().await.clone();
    next.auto_accept_from_paired = enabled;
    if let Err(err) = apply_daemon_config(&next, ctx).await {
        log_config_update(err);
    }
}

fn launch_send_picker(device_id: Option<String>) {
    if let Err(err) = spawn_send_picker_process(device_id.as_deref()) {
        log_send_picker_launch(err);
    }
}

async fn respond_transfer(id: String, accept: bool, ctx: &IpcDispatchContext) {
    let _ = ctx.transfer_engine.respond_consent(&id, accept);
    ctx.refresh_notify.notify_waiters();
}

async fn cancel_transfer(id: String, ctx: &IpcDispatchContext) {
    let _ = ctx.transfer_engine.cancel_transfer(&id);
    ctx.refresh_notify.notify_waiters();
}

fn toggle_device_auto_accept(device_id: String, enabled: bool, ctx: &IpcDispatchContext) {
    match ctx.device_store.set_auto_accept_files(&device_id, enabled) {
        Ok(_) => {
            let _ = ctx
                .transfer_engine
                .set_peer_auto_accept(&device_id, enabled);
            ctx.refresh_notify.notify_waiters();
        }
        Err(err) => {
            log_device_toggle(err);
        }
    }
}

async fn handle_transfer_tray(cmd: &TrayCommand, ctx: &IpcDispatchContext) -> bool {
    match cmd {
        TrayCommand::AcceptTransfer(id) => respond_transfer(id.clone(), true, ctx).await,
        TrayCommand::DeclineTransfer(id) => respond_transfer(id.clone(), false, ctx).await,
        TrayCommand::CancelTransfer(id) => cancel_transfer(id.clone(), ctx).await,
        _ => return false,
    }
    true
}

async fn handle_config_tray(cmd: &TrayCommand, ctx: &IpcDispatchContext) -> bool {
    match cmd {
        TrayCommand::SetVisibility(mode) => {
            super::config::apply_visibility_change(*mode, ctx).await;
        }
        TrayCommand::SetAutoAcceptFromPaired(enabled) => {
            set_auto_accept_from_paired(*enabled, ctx).await;
        }
        TrayCommand::SetDeviceAutoAccept { device_id, enabled } => {
            toggle_device_auto_accept(device_id.clone(), *enabled, ctx);
        }
        _ => return false,
    }
    true
}

async fn handle_launch_tray(cmd: &TrayCommand, ctx: &IpcDispatchContext) -> bool {
    match cmd {
        TrayCommand::CopyLatestOtp => {
            let otp = ctx.latest_otp.lock().await.clone();
            if let Some(code) = otp {
                if let Err(err) = ctx
                    .clipboard_handle
                    .write_clipboard_async("text/plain;charset=utf-8", code.as_bytes())
                    .await
                {
                    log_otp_copy(err);
                }
            }
        }
        TrayCommand::LaunchSendPicker { device_id } => launch_send_picker(device_id.clone()),
        _ => return false,
    }
    true
}

pub(crate) async fn handle_tray_command(cmd: TrayCommand, ctx: &IpcDispatchContext) {
    if handle_transfer_tray(&cmd, ctx).await {
        return;
    }
    if handle_config_tray(&cmd, ctx).await {
        return;
    }
    if handle_media_tray(&cmd).await {
        return;
    }
    handle_launch_tray(&cmd, ctx).await;
}

fn log_pairing_channel_unavailable(approve: bool) {
    warn!(event = "pairing_decision_channel_unavailable", approve);
}

fn log_pairing_forward_failed(approve: bool, err: impl std::fmt::Display) {
    warn!(event = "pairing_decision_forward_failed", approve, error = %err);
}

fn forward_pairing_decision(approve: bool, ctx: &IpcDispatchContext) {
    let Some(ref tx) = ctx.pairing_decision_tx else {
        log_pairing_channel_unavailable(approve);
        return;
    };
    if let Err(err) = tx.try_send(approve) {
        log_pairing_forward_failed(approve, err);
    }
}

fn handle_pairing_notification(action: NotificationAction, ctx: &IpcDispatchContext) -> bool {
    match action {
        NotificationAction::ApprovePairing(peer_id) => {
            log_pairing_approve(&peer_id);
            forward_pairing_decision(true, ctx);
        }
        NotificationAction::RejectPairing(peer_id) => {
            log_pairing_reject(&peer_id);
            forward_pairing_decision(false, ctx);
        }
        _ => return false,
    }
    true
}

pub(crate) async fn handle_notification_action(
    action: NotificationAction,
    ctx: &IpcDispatchContext,
) {
    match action {
        NotificationAction::AcceptTransfer(id) => respond_transfer(id, true, ctx).await,
        NotificationAction::DeclineTransfer(id) => respond_transfer(id, false, ctx).await,
        NotificationAction::CancelTransfer(id) => cancel_transfer(id, ctx).await,
        NotificationAction::MirrorReply(key) => emit_mirror_action(
            key,
            v1::NotificationActionKind::NotificationActionReply,
            String::new(),
            ctx,
        ),
        NotificationAction::MirrorMarkRead(key) => emit_mirror_action(
            key,
            v1::NotificationActionKind::NotificationActionMarkRead,
            String::new(),
            ctx,
        ),
        NotificationAction::MirrorDismiss(key) => emit_mirror_action(
            key,
            v1::NotificationActionKind::NotificationActionDismiss,
            String::new(),
            ctx,
        ),
        NotificationAction::CallAnswer(id) => {
            crate::call::log_call_action(&id, CallActionKind::Answer);
            emit_call_action(id, v1::CallActionKind::CallActionAnswer, ctx)
        }
        NotificationAction::CallDecline(id) => {
            crate::call::log_call_action(&id, CallActionKind::Decline);
            emit_call_action(id, v1::CallActionKind::CallActionDecline, ctx)
        }
        NotificationAction::CallMute(id) => {
            crate::call::log_call_action(&id, CallActionKind::Mute);
            emit_call_action(id, v1::CallActionKind::CallActionMute, ctx)
        }
        other => {
            handle_pairing_notification(other, ctx);
        }
    }
}
