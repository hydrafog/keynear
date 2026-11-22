mod context;
mod init;
mod loops;
mod options;
mod services;
mod shutdown;
mod state;
mod ui;

pub use options::RuntimeOptions;

use std::future::Future;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use keynear_core::{DaemonConfig, PairedDeviceStore, SecretStore};
use keynear_protocol::{default_config_dir, KeynearError};
use tokio::net::UnixListener;
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::ble::gatt::PairingUiEvent;
use crate::ipc::server::bind_ipc_listener;
use crate::keyring_oo7::Oo7SecretStore;
use crate::service::pairing::sync_paired_devices_and_policies;

pub(crate) use context::build_shared_ctx;
use init::{init_services, InitParams};
use loops::{spawn_all_loops, visibility_revert_loop, LoopParams};
pub(crate) use services::RuntimeServices;
use shutdown::{shutdown_all, ShutdownParams};
use state::init_state;
pub(crate) use state::RuntimeState;
use ui::{spawn_ipc_and_ui, IpcUiParams};

pub struct RuntimeDeps {
    pub device_store: PairedDeviceStore,
    pub secret_store: Option<Arc<dyn SecretStore>>,
}

pub struct DaemonRuntime;

impl DaemonRuntime {
    pub async fn run_with_shutdown_with_config_path<F>(
        config: &DaemonConfig,
        config_path: Option<PathBuf>,
        shutdown: F,
    ) -> Result<(), KeynearError>
    where
        F: Future<Output = ()>,
    {
        config.validate()?;
        let device_store = PairedDeviceStore::default_location();
        let secret_store: Option<Arc<dyn SecretStore>> = match Oo7SecretStore::open_default() {
            Ok(store) => {
                if store.is_file_fallback() {
                    log_keyring_fallback();
                }
                Some(Arc::new(store))
            }
            Err(err) => {
                log_keyring_unavailable(err);
                None
            }
        };

        let mut options = RuntimeOptions {
            config_path,
            ..RuntimeOptions::default()
        };
        if let Some(port) = pinned_quic_port() {
            options.quic_bind_addr = Some(SocketAddr::from(([0, 0, 0, 0], port)));
        }
        Self::run_with_deps(
            config,
            options,
            RuntimeDeps {
                device_store,
                secret_store,
            },
            shutdown,
        )
        .await
    }

    pub async fn run_with_deps<F>(
        config: &DaemonConfig,
        options: RuntimeOptions,
        deps: RuntimeDeps,
        shutdown: F,
    ) -> Result<(), KeynearError>
    where
        F: Future<Output = ()>,
    {
        config.validate()?;
        let bound_ipc = bind_early_ipc(&options)?;
        let state = init_state(config, &options);
        crate::vega::initialize(config).await;
        let _fuse_probe_guard = crate::fuse::initialize();
        let mut services = init_services(InitParams {
            config,
            options: &options,
            state: &state,
            device_store: &deps.device_store,
            secret_store: deps.secret_store.as_ref(),
        })
        .await?;
        let mut tasks = Vec::new();
        spawn_outbound_if_ready(&services, &mut tasks);
        let pairing = init_pairing_channels(&options, deps.secret_store.as_ref());
        let lan_watch_tx = spawn_all_loops(
            LoopParams {
                options: &options,
                device_store: &deps.device_store,
                secret_store: deps.secret_store.as_ref(),
                state: &state,
                services: &mut services,
                pairing_decision_rx: pairing.decision_rx,
                pairing_event_tx: pairing.event_tx,
            },
            &mut tasks,
        );
        let (shared_ctx, bound_socket_path, tray_handle) = spawn_ipc_and_ui(
            IpcUiParams {
                options: &options,
                state: &state,
                services: &services,
                bound_ipc,
                pairing_decision_tx: pairing.decision_tx,
                pairing_event_rx: pairing.event_rx,
            },
            &mut tasks,
        )
        .await;
        wait_for_shutdown(shutdown, &state).await;
        shutdown_all(ShutdownParams {
            clipboard_handle: services.clipboard_handle,
            tray_handle,
            shared_ctx,
            config_watch_tx: state.config_watch_tx,
            lan_watch_tx,
            visibility_tx: state.visibility_tx,
            tasks,
            mdns: services.mdns,
            quic_node: services.quic_node,
            bound_socket_path,
        })
        .await;
        Ok(())
    }

    pub async fn run_until_shutdown_with_config_path(
        config: &DaemonConfig,
        config_path: Option<PathBuf>,
    ) -> Result<(), KeynearError> {
        let mut sigterm = signal(SignalKind::terminate())?;
        let shutdown = async {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {}
                _ = sigterm.recv() => {}
            }
        };
        Self::run_with_shutdown_with_config_path(config, config_path, shutdown).await
    }
}

struct PairingChannels {
    decision_tx: Option<mpsc::Sender<bool>>,
    decision_rx: Option<mpsc::Receiver<bool>>,
    event_tx: Option<mpsc::Sender<PairingUiEvent>>,
    event_rx: Option<mpsc::Receiver<PairingUiEvent>>,
}

fn init_pairing_channels(
    options: &RuntimeOptions,
    secret_store: Option<&Arc<dyn SecretStore>>,
) -> PairingChannels {
    if !options.enable_bluez || secret_store.is_none() {
        return PairingChannels {
            decision_tx: None,
            decision_rx: None,
            event_tx: None,
            event_rx: None,
        };
    }
    let (decision_tx, decision_rx) = mpsc::channel(8);
    let (event_tx, event_rx) = mpsc::channel(8);
    PairingChannels {
        decision_tx: Some(decision_tx),
        decision_rx: Some(decision_rx),
        event_tx: Some(event_tx),
        event_rx: Some(event_rx),
    }
}

fn bind_early_ipc(
    options: &RuntimeOptions,
) -> Result<Option<(PathBuf, UnixListener)>, KeynearError> {
    if !options.enable_ipc {
        return Ok(None);
    }
    let sock_path = options
        .ipc_socket_path
        .clone()
        .unwrap_or_else(keynear_ipc::daemon_socket_path);
    Ok(Some(bind_ipc_listener(sock_path)?))
}

fn spawn_outbound_if_ready(services: &RuntimeServices, tasks: &mut Vec<JoinHandle<()>>) {
    let audio_tx = super::audio_control::spawn_audio_control(tasks);
    if let (Some(rx), Some(node)) = (
        crate::ipc::handlers::install_outbound_queue(),
        services.quic_node.clone(),
    ) {
        super::outbound::spawn_outbound_drain(
            super::outbound::OutboundDrainParams {
                rx,
                node,
                device_store: services.device_store.clone(),
                audio_tx,
            },
            tasks,
        );
    } else {
        tracing::warn!(event = "continuity_outbound_transport_unavailable");
    }
}

struct SyncPeersParams<'a> {
    device_store: &'a PairedDeviceStore,
    secret_store: Option<&'a Arc<dyn SecretStore>>,
    quic_node: &'a Option<Arc<keynear_core::OrionQuicNode>>,
    transfer_engine: &'a Arc<keynear_core::TransferEngine>,
    known_peers: &'a Arc<tokio::sync::Mutex<std::collections::HashSet<String>>>,
}

async fn sync_initial_peers(params: SyncPeersParams<'_>) {
    if let Err(err) = sync_paired_devices_and_policies(
        params.device_store,
        params.secret_store.map(|s| s.as_ref()),
        params.quic_node.as_deref(),
        params.transfer_engine,
        params.known_peers,
    )
    .await
    {
        log_peer_sync_failed(err);
    }
}

async fn wait_for_shutdown<F: Future<Output = ()>>(shutdown: F, state: &RuntimeState) {
    let revert_timer = visibility_revert_loop(state, state.config_path.clone());
    tokio::select! {
        _ = shutdown => {}
        _ = revert_timer => {}
    }
}

fn log_invalid_quic_port(value: &str) {
    tracing::warn!(event = "quic_port_override_invalid", value = %value);
}

fn pinned_quic_port() -> Option<u16> {
    let raw = std::env::var("KEYNEAR_QUIC_PORT").ok()?;
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.contains('\0') {
        return None;
    }
    match trimmed.parse::<u16>() {
        Ok(port) if port > 0 => Some(port),
        _ => {
            log_invalid_quic_port(trimmed);
            None
        }
    }
}

fn log_keyring_fallback() {
    tracing::info!(
        event = "keyring_file_fallback_active",
        config_dir = %default_config_dir().display()
    );
}

fn log_keyring_unavailable(err: keynear_protocol::KeynearError) {
    tracing::warn!(event = "keyring_unavailable_degraded", error = %err);
}

fn log_peer_sync_failed(err: keynear_protocol::KeynearError) {
    tracing::warn!(event = "initial_peer_sync_failed", error = %err);
}
