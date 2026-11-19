use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use keynear_core::{
    ClipboardEngine, ClipboardItem, ClipboardPolicy, MdnsDiscovery, OrionNodeConfig, OrionQuicNode,
    PairedDeviceStore, SecretStore, TransferEngine, TransferState,
};
use keynear_ipc::daemon_socket_path;
use keynear_protocol::{
    default_config_dir, DaemonConfig, KeynearError, VisibilityMode, EVERYONE_NEARBY_TIMEOUT_SECS,
};
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::{mpsc, watch, Mutex, Notify};
use tokio::task::JoinHandle;
use tokio::time::Instant;
use tracing::{info, warn};
use zeroize::Zeroizing;

use crate::ble::DEFAULT_CAPABILITY_FLAGS;
use crate::clipboard::{start_clipboard_monitor, ClipboardChange, ClipboardMonitorHandle};
use crate::ipc::{
    bind_and_serve_ipc, build_tray_state, handle_notification_action, handle_tray_command,
    IpcDispatchContext,
};
use crate::keyring_oo7::Oo7SecretStore;
use crate::notifications::NotificationsClient;
use crate::service::clipboard::{spawn_clipboard_loops, ClipboardLoopParams};
use crate::service::pairing::{
    spawn_bluez_loops, spawn_devices_poll_loop, spawn_mdns_browse_loop,
    sync_paired_devices_and_policies, BluezLoopParams,
};
use crate::service::transfer::{spawn_quic_accept_loop, sync_transfer_notifications};
use crate::tray::spawn_tray;

const TRAY_REFRESH_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Debug, Clone)]
pub struct RuntimeOptions {
    pub local_device_id: String,
    pub display_name: String,
    pub everyone_nearby_timeout: Duration,
    pub enable_bluez: bool,
    pub enable_mdns: bool,
    pub enable_ipc: bool,
    pub enable_tray: bool,
    pub enable_notifications: bool,
    pub enable_clipboard: bool,
    pub config_path: Option<PathBuf>,
    pub ipc_socket_path: Option<PathBuf>,
    pub quic_bind_addr: Option<SocketAddr>,
    pub mdns_port: Option<u16>,
}

impl Default for RuntimeOptions {
    fn default() -> Self {
        Self {
            local_device_id: default_local_device_id(),
            display_name: default_display_name(),
            everyone_nearby_timeout: Duration::from_secs(EVERYONE_NEARBY_TIMEOUT_SECS),
            enable_bluez: true,
            enable_mdns: true,
            enable_ipc: true,
            enable_tray: true,
            enable_notifications: true,
            enable_clipboard: true,
            config_path: None,
            ipc_socket_path: None,
            quic_bind_addr: None,
            mdns_port: None,
        }
    }
}

fn default_local_device_id() -> String {
    let raw = std::env::var("HOSTNAME")
        .or_else(|_| std::fs::read_to_string("/etc/hostname"))
        .unwrap_or_else(|_| "keynear-linux".to_string());
    let cleaned: String = raw
        .trim()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') {
                c
            } else {
                '-'
            }
        })
        .take(48)
        .collect();
    if cleaned.is_empty() {
        "keynear-linux".to_string()
    } else {
        cleaned
    }
}

fn default_display_name() -> String {
    std::env::var("HOSTNAME")
        .or_else(|_| std::fs::read_to_string("/etc/hostname"))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Keynear Linux".to_string())
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
                    info!(
                        event = "keyring_file_fallback_active",
                        config_dir = %default_config_dir().display()
                    );
                }
                Some(Arc::new(store))
            }
            Err(err) => {
                warn!(event = "keyring_unavailable_degraded", error = %err);
                None
            }
        };

        let options = RuntimeOptions {
            config_path,
            ..RuntimeOptions::default()
        };
        Self::run_with_deps(config, options, device_store, secret_store, shutdown).await
    }

    pub async fn run_with_deps<F>(
        config: &DaemonConfig,
        options: RuntimeOptions,
        device_store: PairedDeviceStore,
        secret_store: Option<Arc<dyn SecretStore>>,
        shutdown: F,
    ) -> Result<(), KeynearError>
    where
        F: Future<Output = ()>,
    {
        config.validate()?;
        let (visibility_tx, visibility_rx) = watch::channel(config.visibility_mode);
        let (config_watch_tx, config_watch_rx) = watch::channel(config.clone());
        let initial_mode = config.visibility_mode;
        let config_state = Arc::new(Mutex::new(config.clone()));
        let config_path = options.config_path.clone();
        let everyone_deadline = Arc::new(Mutex::new(
            if initial_mode == VisibilityMode::EveryoneNearby {
                Some(Instant::now() + options.everyone_nearby_timeout)
            } else {
                None
            },
        ));
        let deadline_notify = Arc::new(Notify::new());
        let refresh_notify = Arc::new(Notify::new());
        info!(
            event = "daemon_started",
            visibility = ?initial_mode,
            incoming_dir = %config.incoming_directory
        );

        let transfer_engine = Arc::new(TransferEngine::new(
            PathBuf::from(&config.incoming_directory),
            config.auto_accept_from_paired,
        ));
        let (incoming_clip_tx, incoming_clip_rx) = mpsc::unbounded_channel::<ClipboardItem>();
        let clipboard_engine = Arc::new(ClipboardEngine::with_incoming_sender(
            ClipboardPolicy::from(config),
            incoming_clip_tx,
        ));
        let (clip_change_tx, clip_change_rx) = mpsc::unbounded_channel::<ClipboardChange>();
        let clipboard_handle = if options.enable_clipboard {
            start_clipboard_monitor(config_watch_rx, clip_change_tx)
        } else {
            ClipboardMonitorHandle::headless()
        };
        info!(
            event = "clipboard_monitor_ready",
            backend = clipboard_handle.backend().as_str()
        );
        let latest_otp: Arc<Mutex<Option<Zeroizing<String>>>> = Arc::new(Mutex::new(None));
        let known_peers = Arc::new(Mutex::new(HashSet::new()));

        let quic_node = bind_quic_node(&options, secret_store.as_ref()).await?;

        if let Err(err) = sync_paired_devices_and_policies(
            &device_store,
            secret_store.as_deref(),
            quic_node.as_deref(),
            &transfer_engine,
            &known_peers,
        )
        .await
        {
            warn!(event = "initial_peer_sync_failed", error = %err);
        }

        let mdns = setup_mdns(&options, quic_node.as_ref());
        let mut tasks: Vec<JoinHandle<()>> = Vec::new();

        spawn_mdns_browse_loop(
            mdns.as_ref(),
            quic_node.as_ref(),
            secret_store.as_ref(),
            &device_store,
            &options.local_device_id,
            &mut tasks,
        );

        if let Some(ref node) = quic_node {
            spawn_quic_accept_loop(
                node,
                &device_store,
                secret_store.as_ref(),
                &transfer_engine,
                &clipboard_engine,
                &known_peers,
                &mut tasks,
            );
        }

        spawn_clipboard_loops(
            ClipboardLoopParams {
                quic_node: quic_node.as_ref(),
                clipboard_engine: &clipboard_engine,
                device_store: &device_store,
                clipboard_handle: &clipboard_handle,
                latest_otp: &latest_otp,
                refresh_notify: &refresh_notify,
                clip_change_rx,
                incoming_clip_rx,
            },
            &mut tasks,
        );

        spawn_devices_poll_loop(
            &device_store,
            secret_store.as_ref(),
            quic_node.as_ref(),
            &transfer_engine,
            &known_peers,
            &mut tasks,
        );

        let (lan_watch_tx, lan_watch_rx) =
            watch::channel(quic_node.as_ref().map(|q| q.local_lan_endpoint()));
        spawn_bluez_loops(
            BluezLoopParams {
                enable_bluez: options.enable_bluez,
                local_device_id: &options.local_device_id,
                display_name: &options.display_name,
                visibility_rx: &visibility_rx,
                lan_watch_rx,
                device_store: &device_store,
                secret_store: secret_store.as_ref(),
                quic_node: quic_node.as_ref(),
            },
            &mut tasks,
        );

        let mdns_degraded = mdns.as_ref().map(|m| m.is_degraded()).unwrap_or(false);
        let shared_ctx = IpcDispatchContext {
            local_device_id: options.local_device_id.clone(),
            visibility_tx: visibility_tx.clone(),
            visibility_rx: visibility_rx.clone(),
            config_state: Arc::clone(&config_state),
            config_watch_tx: config_watch_tx.clone(),
            config_path: config_path.clone(),
            everyone_nearby_timeout: options.everyone_nearby_timeout,
            everyone_deadline: Arc::clone(&everyone_deadline),
            deadline_notify: Arc::clone(&deadline_notify),
            refresh_notify: Arc::clone(&refresh_notify),
            device_store: device_store.clone(),
            secret_store: secret_store.clone(),
            quic_node: quic_node.clone(),
            transfer_engine: Arc::clone(&transfer_engine),
            clipboard_engine: Arc::clone(&clipboard_engine),
            clipboard_handle: clipboard_handle.clone(),
            latest_otp: Arc::clone(&latest_otp),
            known_peers: Arc::clone(&known_peers),
            mdns_degraded,
            mdns: mdns.clone(),
        };

        let bound_socket_path = if options.enable_ipc {
            let sock_path = options
                .ipc_socket_path
                .clone()
                .unwrap_or_else(daemon_socket_path);
            Some(bind_and_serve_ipc(sock_path, shared_ctx.clone(), &mut tasks).await?)
        } else {
            None
        };

        let tray_handle = setup_ui_loops(&options, &shared_ctx, &mut tasks).await;

        let revert_timer = async {
            loop {
                let deadline = *everyone_deadline.lock().await;
                match deadline {
                    Some(target) => {
                        tokio::select! {
                            _ = tokio::time::sleep_until(target) => {
                                *everyone_deadline.lock().await = None;
                                if *visibility_rx.borrow() == VisibilityMode::EveryoneNearby {
                                    let _ = visibility_tx.send(VisibilityMode::PairedOnly);
                                    let mut cfg = config_state.lock().await;
                                    cfg.visibility_mode = VisibilityMode::PairedOnly;
                                    if let Some(ref path) = config_path {
                                        let _ = cfg.save_to_path(path);
                                    }
                                    drop(cfg);
                                    refresh_notify.notify_waiters();
                                    info!(event = "visibility_auto_reverted", to = ?VisibilityMode::PairedOnly);
                                }
                            }
                            _ = deadline_notify.notified() => {}
                        }
                    }
                    None => {
                        deadline_notify.notified().await;
                    }
                }
            }
        };

        tokio::select! {
            _ = shutdown => {}
            _ = revert_timer => {}
        }

        clipboard_handle.shutdown();
        if let Some(handle) = tray_handle {
            handle.shutdown().await;
        }
        drop(shared_ctx);
        drop(config_watch_tx);
        drop(lan_watch_tx);
        drop(visibility_tx);
        for handle in tasks {
            handle.abort();
            let _ = handle.await;
        }
        if let Some(discovery) = mdns {
            let _ = tokio::task::spawn_blocking(move || discovery.shutdown()).await;
        }
        if let Some(node) = quic_node {
            node.close().await;
        }
        if let Some(sock_path) = bound_socket_path {
            let _ = std::fs::remove_file(&sock_path);
        }

        info!(event = "daemon_stopped");
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

async fn bind_quic_node(
    options: &RuntimeOptions,
    secret_store: Option<&Arc<dyn SecretStore>>,
) -> Result<Option<Arc<OrionQuicNode>>, KeynearError> {
    let Some(secrets) = secret_store else {
        warn!(
            event = "quic_unavailable_degraded",
            reason = "keyring_absent"
        );
        return Ok(None);
    };
    let quic_secret_key = secrets.get_or_create_quic_secret_key()?;
    let node = OrionQuicNode::bind(
        OrionNodeConfig {
            device_id: options.local_device_id.clone(),
            display_name: options.display_name.clone(),
            capability_flags: DEFAULT_CAPABILITY_FLAGS,
            quic_secret_key,
        },
        options.quic_bind_addr,
    )
    .await?;
    Ok(Some(Arc::new(node)))
}

fn setup_mdns(
    options: &RuntimeOptions,
    quic_node: Option<&Arc<OrionQuicNode>>,
) -> Option<Arc<MdnsDiscovery>> {
    if !options.enable_mdns {
        return None;
    }
    let discovery = match options.mdns_port {
        Some(port) => MdnsDiscovery::new_with_port(port),
        None => MdnsDiscovery::new(),
    };
    if let Some(node) = quic_node {
        let endpoint_addr = node.endpoint_addr();
        let mut addrs: Vec<IpAddr> = Vec::new();
        let mut port: u16 = 0;
        for sock in endpoint_addr.ip_addrs() {
            if port == 0 {
                port = sock.port();
            }
            if !addrs.contains(&sock.ip()) {
                addrs.push(sock.ip());
            }
        }
        let eid_bytes = *node.endpoint_id().as_bytes();
        if let Err(err) = discovery.register(
            &options.local_device_id,
            &eid_bytes,
            port,
            &addrs,
            DEFAULT_CAPABILITY_FLAGS,
        ) {
            warn!(event = "mdns_register_failed", error = %err);
        }
    }
    Some(Arc::new(discovery))
}

async fn setup_ui_loops(
    options: &RuntimeOptions,
    shared_ctx: &IpcDispatchContext,
    tasks: &mut Vec<JoinHandle<()>>,
) -> Option<ksni::Handle<crate::tray::KeynearTray>> {
    let tray_handle = if options.enable_tray {
        let (tray_cmd_tx, mut tray_cmd_rx) = mpsc::unbounded_channel();
        let initial_tray_state = build_tray_state(shared_ctx).await;
        let handle = spawn_tray(initial_tray_state, tray_cmd_tx).await;
        let cmd_ctx = shared_ctx.clone();
        tasks.push(tokio::spawn(async move {
            while let Some(cmd) = tray_cmd_rx.recv().await {
                handle_tray_command(cmd, &cmd_ctx).await;
            }
        }));
        handle
    } else {
        None
    };

    let notif_client = if options.enable_notifications {
        let (action_tx, mut action_rx) = mpsc::unbounded_channel();
        if let Some((client, listener_handle)) =
            NotificationsClient::connect_and_listen(action_tx).await
        {
            tasks.push(listener_handle);
            let action_ctx = shared_ctx.clone();
            tasks.push(tokio::spawn(async move {
                while let Some(action) = action_rx.recv().await {
                    handle_notification_action(action, &action_ctx).await;
                }
            }));
            Some(client)
        } else {
            None
        }
    } else {
        None
    };

    if tray_handle.is_some() || notif_client.is_some() {
        let sync_ctx = shared_ctx.clone();
        let sync_tray = tray_handle.clone();
        let sync_notifs = notif_client.clone();
        tasks.push(tokio::spawn(async move {
            let mut interval = tokio::time::interval(TRAY_REFRESH_INTERVAL);
            let mut sent_notifs: HashMap<String, (u32, TransferState, u64)> = HashMap::new();
            loop {
                tokio::select! {
                    _ = interval.tick() => {}
                    _ = sync_ctx.refresh_notify.notified() => {}
                }
                if let Some(ref handle) = sync_tray {
                    let next_state = build_tray_state(&sync_ctx).await;
                    let _ = handle.update(move |tray| tray.state = next_state).await;
                }
                if let Some(ref client) = sync_notifs {
                    sync_transfer_notifications(client, &sync_ctx, &mut sent_notifs).await;
                }
            }
        }));
    }

    tray_handle
}
