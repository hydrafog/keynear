use std::sync::Arc;

use keynear_core::{PairedDeviceStore, RangingSample, SecretStore};
use tokio::sync::mpsc;
use tokio::sync::watch::Sender;
use tokio::task::JoinHandle;

use crate::ble::gatt::PairingUiEvent;
use crate::service::clipboard::{spawn_clipboard_loops, ClipboardLoopParams};
use crate::service::pairing::{
    spawn_bluez_loops, spawn_devices_poll_loop, spawn_mdns_browse_loop, BluezLoopParams,
};
use crate::service::transfer::spawn_quic_accept_loop;

use super::{RuntimeOptions, RuntimeServices, RuntimeState};

pub(super) struct LoopParams<'a> {
    pub options: &'a RuntimeOptions,
    pub device_store: &'a PairedDeviceStore,
    pub secret_store: Option<&'a Arc<dyn SecretStore>>,
    pub state: &'a RuntimeState,
    pub services: &'a mut RuntimeServices,
    pub pairing_decision_rx: Option<mpsc::Receiver<bool>>,
    pub pairing_event_tx: Option<mpsc::Sender<PairingUiEvent>>,
}

struct LoopCtx<'a> {
    options: &'a RuntimeOptions,
    device_store: &'a PairedDeviceStore,
    secret_store: Option<&'a Arc<dyn SecretStore>>,
    state: &'a RuntimeState,
    services: &'a mut RuntimeServices,
    pairing_decision_rx: Option<mpsc::Receiver<bool>>,
    pairing_event_tx: Option<mpsc::Sender<PairingUiEvent>>,
}

fn spawn_network_loops(ctx: &LoopCtx<'_>, tasks: &mut Vec<JoinHandle<()>>) {
    let services = &*ctx.services;
    spawn_mdns_browse_loop(
        services.mdns.as_ref(),
        services.quic_node.as_ref(),
        ctx.secret_store,
        ctx.device_store,
        &ctx.options.local_device_id,
        tasks,
    );
    if let Some(ref node) = services.quic_node {
        spawn_quic_accept_loop(
            node,
            ctx.device_store,
            ctx.secret_store,
            &services.transfer_engine,
            &services.clipboard_engine,
            &services.known_peers,
            tasks,
        );
    }
    spawn_devices_poll_loop(
        ctx.device_store,
        ctx.secret_store,
        services.quic_node.as_ref(),
        &services.transfer_engine,
        &services.known_peers,
        tasks,
    );
}

fn spawn_clipboard_and_bluez(
    ctx: &mut LoopCtx<'_>,
    tasks: &mut Vec<JoinHandle<()>>,
) -> Sender<Option<keynear_protocol::v1::LanEndpoint>> {
    let services = &mut *ctx.services;
    let (ranging_tx, ranging_rx) = mpsc::channel::<RangingSample>(32);
    if let Some(supervisor) = services.phoenix_supervisor.clone() {
        crate::proximity::spawn_proximity_loop(supervisor, ranging_rx, tasks);
    }
    spawn_clipboard_loops(
        ClipboardLoopParams {
            quic_node: services.quic_node.as_ref(),
            clipboard_engine: &services.clipboard_engine,
            device_store: ctx.device_store,
            clipboard_handle: &services.clipboard_handle,
            latest_otp: &services.latest_otp,
            refresh_notify: &ctx.state.refresh_notify,
            clip_change_rx: services
                .clip_change_rx
                .take()
                .expect("clip change receiver already taken"),
            incoming_clip_rx: services
                .incoming_clip_rx
                .take()
                .expect("incoming clip receiver already taken"),
        },
        tasks,
    );

    let (lan_watch_tx, lan_watch_rx) =
        tokio::sync::watch::channel(services.quic_node.as_ref().map(|q| q.local_lan_endpoint()));
    spawn_bluez_loops(
        BluezLoopParams {
            enable_bluez: ctx.options.enable_bluez,
            local_device_id: &ctx.options.local_device_id,
            display_name: &ctx.options.display_name,
            visibility_rx: &ctx.state.visibility_rx,
            lan_watch_rx,
            device_store: ctx.device_store,
            secret_store: ctx.secret_store,
            quic_node: services.quic_node.as_ref(),
            pairing_decision_rx: ctx.pairing_decision_rx.take(),
            pairing_event_tx: ctx.pairing_event_tx.take(),
            ranging_tx: Some(ranging_tx),
        },
        tasks,
    );
    lan_watch_tx
}

pub(super) fn spawn_all_loops(
    params: LoopParams<'_>,
    tasks: &mut Vec<JoinHandle<()>>,
) -> Sender<Option<keynear_protocol::v1::LanEndpoint>> {
    let LoopParams {
        options,
        device_store,
        secret_store,
        state,
        services,
        pairing_decision_rx,
        pairing_event_tx,
    } = params;
    let mut ctx = LoopCtx {
        options,
        device_store,
        secret_store,
        state,
        services,
        pairing_decision_rx,
        pairing_event_tx,
    };
    spawn_network_loops(&ctx, tasks);
    spawn_clipboard_and_bluez(&mut ctx, tasks)
}

pub(super) async fn visibility_revert_loop(
    state: &RuntimeState,
    config_path: Option<std::path::PathBuf>,
) {
    loop {
        let deadline = *state.everyone_deadline.lock().await;
        match deadline {
            Some(target) => {
                tokio::select! {
                    _ = tokio::time::sleep_until(target) => {
                        revert_when_due(state, config_path.as_deref()).await;
                    }
                    _ = state.deadline_notify.notified() => {}
                }
            }
            None => {
                state.deadline_notify.notified().await;
            }
        }
    }
}

fn log_visibility_revert() {
    tracing::info!(
        event = "visibility_auto_reverted",
        to = ?keynear_protocol::VisibilityMode::PairedOnly
    );
}

async fn revert_when_due(state: &RuntimeState, config_path: Option<&std::path::Path>) {
    *state.everyone_deadline.lock().await = None;
    if *state.visibility_rx.borrow() != keynear_protocol::VisibilityMode::EveryoneNearby {
        return;
    }
    let _ = state
        .visibility_tx
        .send(keynear_protocol::VisibilityMode::PairedOnly);
    let mut cfg = state.config_state.lock().await;
    cfg.visibility_mode = keynear_protocol::VisibilityMode::PairedOnly;
    if let Some(path) = config_path {
        let _ = cfg.save_to_path(path);
    }
    drop(cfg);
    state.refresh_notify.notify_waiters();
    log_visibility_revert();
}
