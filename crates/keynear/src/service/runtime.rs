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
use tokio::signal::unix::{signal, SignalKind};
use tokio::task::JoinHandle;

use crate::keyring_oo7::Oo7SecretStore;
use crate::service::pairing::sync_paired_devices_and_policies;

pub(crate) use context::build_shared_ctx;
use init::{init_services, InitParams};
use loops::{spawn_all_loops, visibility_revert_loop, LoopParams};
pub(crate) use services::RuntimeServices;
use shutdown::{shutdown_all, ShutdownParams};
use state::init_state;
pub(crate) use state::RuntimeState;
use ui::spawn_ipc_and_ui;

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
        let RuntimeDeps {
            device_store,
            secret_store,
        } = deps;
        config.validate()?;
        let state = init_state(config, &options);
        crate::vega::initialize(config).await;
        let _fuse_probe_guard = crate::fuse::initialize();
        let mut services = init_services(InitParams {
            config,
            options: &options,
            state: &state,
            device_store: &device_store,
            secret_store: secret_store.as_ref(),
        })
        .await?;
        let mut tasks: Vec<JoinHandle<()>> = Vec::new();
        let audio_tx = super::audio_control::spawn_audio_control(&mut tasks);
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
                &mut tasks,
            );
        } else {
            tracing::warn!(event = "continuity_outbound_transport_unavailable");
        }
        let lan_watch_tx = spawn_all_loops(
            LoopParams {
                options: &options,
                device_store: &device_store,
                secret_store: secret_store.as_ref(),
                state: &state,
                services: &mut services,
            },
            &mut tasks,
        );

        let (shared_ctx, bound_socket_path, tray_handle) =
            spawn_ipc_and_ui(&options, &state, &services, &mut tasks).await?;
        let revert_timer = visibility_revert_loop(&state, state.config_path.clone());
        tokio::select! {
            _ = shutdown => {}
            _ = revert_timer => {}
        }

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

fn pinned_quic_port() -> Option<u16> {
    let raw = std::env::var("KEYNEAR_QUIC_PORT").ok()?;
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.contains('\0') {
        return None;
    }
    match trimmed.parse::<u16>() {
        Ok(port) if port > 0 => Some(port),
        _ => {
            tracing::warn!(event = "quic_port_override_invalid", value = %trimmed);
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
