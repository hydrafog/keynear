use std::path::PathBuf;

use keynear_core::ClipboardPolicy;
use keynear_protocol::{DaemonConfig, KeynearError, VisibilityMode};
use tokio::time::Instant;

use super::IpcDispatchContext;

fn deadline_for(mode: VisibilityMode, ctx: &IpcDispatchContext) -> Option<Instant> {
    if mode == VisibilityMode::EveryoneNearby {
        Some(Instant::now() + ctx.everyone_nearby_timeout)
    } else {
        None
    }
}

pub(crate) async fn apply_visibility_change(mode: VisibilityMode, ctx: &IpcDispatchContext) {
    *ctx.everyone_deadline.lock().await = deadline_for(mode, ctx);
    let _ = ctx.visibility_tx.send(mode);
    ctx.deadline_notify.notify_waiters();
    save_visibility(mode, ctx).await;
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
    *ctx.everyone_deadline.lock().await = deadline_for(new_config.visibility_mode, ctx);
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

async fn save_visibility(mode: keynear_protocol::VisibilityMode, ctx: &IpcDispatchContext) {
    let mut cfg = ctx.config_state.lock().await;
    cfg.visibility_mode = mode;
    if let Some(ref path) = ctx.config_path {
        if let Err(err) = cfg.save_to_path(path) {
            log_config_save(err);
        }
    }
}

fn log_config_save(err: keynear_protocol::KeynearError) {
    tracing::warn!(event = "config_save_failed", error = %err);
}
