use std::sync::Arc;

use tokio::sync::Mutex;

use keynear_core::{DaemonConfig, PairedDeviceStore, ProximityConfig, SecretStore};
use keynear_protocol::KeynearError;
use tracing::{info, warn};

use crate::proximity::{Login1SessionLock, ProximitySupervisor};

use super::services::{bind_quic_node, init_clipboard, setup_mdns, RuntimeServices};
use super::{sync_initial_peers, RuntimeOptions, RuntimeState, SyncPeersParams};

pub(super) struct InitParams<'a> {
    pub config: &'a DaemonConfig,
    pub options: &'a RuntimeOptions,
    pub state: &'a RuntimeState,
    pub device_store: &'a PairedDeviceStore,
    pub secret_store: Option<&'a Arc<dyn SecretStore>>,
}

pub(super) async fn init_services(params: InitParams<'_>) -> Result<RuntimeServices, KeynearError> {
    let (
        transfer_engine,
        incoming_clip_rx,
        clipboard_engine,
        clip_change_rx,
        clipboard_handle,
        latest_otp,
        known_peers,
    ) = init_clipboard(params.config, params.options, params.state);
    let quic_node = bind_quic_node(params.options, params.secret_store).await?;
    sync_initial_peers(SyncPeersParams {
        device_store: params.device_store,
        secret_store: params.secret_store,
        quic_node: &quic_node,
        transfer_engine: &transfer_engine,
        known_peers: &known_peers,
    })
    .await;
    let mdns = setup_mdns(params.options, quic_node.as_ref());
    let phoenix_supervisor = init_phoenix(params.config)?;

    Ok(RuntimeServices {
        device_store: params.device_store.clone(),
        secret_store: params.secret_store.cloned(),
        transfer_engine,
        incoming_clip_rx: Some(incoming_clip_rx),
        clipboard_engine,
        clip_change_rx: Some(clip_change_rx),
        clipboard_handle,
        latest_otp,
        known_peers,
        quic_node,
        mdns,
        phoenix_supervisor: phoenix_supervisor.map(|value| Arc::new(Mutex::new(value))),
    })
}

fn init_phoenix(
    config: &DaemonConfig,
) -> Result<Option<ProximitySupervisor<Login1SessionLock>>, KeynearError> {
    if !config.proximity_lock_enabled && !config.presence_gating_enabled {
        return Ok(None);
    }
    let bounds = ProximityConfig {
        enter_distance_meters: (config.proximity_distance_meters * 0.6).max(0.5),
        exit_distance_meters: config.proximity_distance_meters,
        ..ProximityConfig::default()
    };
    let session_id = std::env::var("XDG_SESSION_ID").unwrap_or_default();
    let supervisor = ProximitySupervisor::new(
        Login1SessionLock,
        bounds,
        session_id,
        config.proximity_lock_enabled,
    )?;
    if config.audio_continuity_enabled {
        tokio::spawn(async {
            match tokio::task::spawn_blocking(crate::audio::pipewire::observe_sinks).await {
                Ok(Ok(sinks)) => info!(event = "phoenix_pipewire_sinks", count = sinks.len()),
                Ok(Err(err)) => warn!(event = "phoenix_pipewire_unavailable", error = %err),
                Err(err) => warn!(event = "phoenix_pipewire_task_failed", error = %err),
            }
        });
    }
    Ok(Some(supervisor))
}
