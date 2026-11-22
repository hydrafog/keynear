use std::path::PathBuf;

use keynear_ipc::daemon_socket_path;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::ipc::bind_and_serve_ipc;
use crate::ipc::{
    build_tray_state, handle_notification_action, handle_tray_command, IpcDispatchContext,
};
use crate::notifications::NotificationsClient;
use crate::service::transfer::sync_transfer_notifications;
use crate::tray::spawn_tray;

use super::{build_shared_ctx, RuntimeOptions, RuntimeServices, RuntimeState};

const TRAY_REFRESH_INTERVAL: std::time::Duration = std::time::Duration::from_secs(1);

async fn spawn_tray_loop(
    options: &RuntimeOptions,
    ctx: &IpcDispatchContext,
    tasks: &mut Vec<JoinHandle<()>>,
) -> Option<ksni::Handle<crate::tray::KeynearTray>> {
    if !options.enable_tray {
        return None;
    }
    let (tray_cmd_tx, mut tray_cmd_rx) = mpsc::unbounded_channel();
    let initial_tray_state = build_tray_state(ctx).await;
    let handle = spawn_tray(initial_tray_state, tray_cmd_tx).await;
    let cmd_ctx = ctx.clone();
    tasks.push(tokio::spawn(async move {
        while let Some(cmd) = tray_cmd_rx.recv().await {
            handle_tray_command(cmd, &cmd_ctx).await;
        }
    }));
    handle
}

async fn spawn_notification_loop(
    options: &RuntimeOptions,
    ctx: &IpcDispatchContext,
    tasks: &mut Vec<JoinHandle<()>>,
) -> Option<NotificationsClient> {
    if !options.enable_notifications {
        return None;
    }
    let (action_tx, mut action_rx) = mpsc::unbounded_channel();
    let (client, listener_handle) = NotificationsClient::connect_and_listen(action_tx).await?;
    tasks.push(listener_handle);
    let action_ctx = ctx.clone();
    tasks.push(tokio::spawn(async move {
        while let Some(action) = action_rx.recv().await {
            handle_notification_action(action, &action_ctx).await;
        }
    }));
    Some(client)
}

fn spawn_sync_loop(
    tray_handle: Option<ksni::Handle<crate::tray::KeynearTray>>,
    notif_client: Option<NotificationsClient>,
    ctx: &IpcDispatchContext,
    tasks: &mut Vec<JoinHandle<()>>,
) {
    if tray_handle.is_none() && notif_client.is_none() {
        return;
    }
    let sync_ctx = ctx.clone();
    tasks.push(tokio::spawn(async move {
        let mut interval = tokio::time::interval(TRAY_REFRESH_INTERVAL);
        let mut media_sequence = 0u64;
        let mut sent_notifs: std::collections::HashMap<
            String,
            (u32, keynear_core::TransferState, u64),
        > = std::collections::HashMap::new();
        loop {
            tokio::select! {
                _ = interval.tick() => {}
                _ = sync_ctx.refresh_notify.notified() => {}
            }
            if let Some(ref handle) = tray_handle {
                let mut next_state = build_tray_state(&sync_ctx).await;
                media_sequence = media_sequence.saturating_add(1);
                let players = crate::media::observe(media_sequence).await;
                if let Some(first) = players.first() {
                    next_state.now_playing = Some(crate::tray::NowPlayingState {
                        title: first.title.clone(),
                        artist: first.artist.clone(),
                        playing: first.playback == keynear_core::PlaybackState::Playing,
                    });
                }
                let _ = handle.update(move |tray| tray.state = next_state).await;
            }
            if let Some(ref client) = notif_client {
                sync_transfer_notifications(client, &sync_ctx, &mut sent_notifs).await;
            }
        }
    }));
}

pub(super) async fn setup_ui_loops(
    options: &RuntimeOptions,
    ctx: &IpcDispatchContext,
    tasks: &mut Vec<JoinHandle<()>>,
) -> Option<ksni::Handle<crate::tray::KeynearTray>> {
    let tray_handle = spawn_tray_loop(options, ctx, tasks).await;
    let notif_client = spawn_notification_loop(options, ctx, tasks).await;
    spawn_sync_loop(tray_handle.clone(), notif_client, ctx, tasks);
    tray_handle
}

pub(super) async fn spawn_ipc_and_ui(
    options: &RuntimeOptions,
    state: &RuntimeState,
    services: &RuntimeServices,
    tasks: &mut Vec<JoinHandle<()>>,
) -> Result<
    (
        IpcDispatchContext,
        Option<PathBuf>,
        Option<ksni::Handle<crate::tray::KeynearTray>>,
    ),
    keynear_protocol::KeynearError,
> {
    let shared_ctx = build_shared_ctx(options, state, services, services.mdns_degraded());
    let bound_socket_path = if options.enable_ipc {
        let sock_path = options
            .ipc_socket_path
            .clone()
            .unwrap_or_else(daemon_socket_path);
        Some(bind_and_serve_ipc(sock_path, shared_ctx.clone(), tasks).await?)
    } else {
        None
    };
    let tray_handle = setup_ui_loops(options, &shared_ctx, tasks).await;
    Ok((shared_ctx, bound_socket_path, tray_handle))
}
