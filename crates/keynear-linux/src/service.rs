use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use keynear_core::{
    default_config_dir, parse_endpoint_id_bytes, v1, DaemonConfig, KeynearError, MdnsDiscovery,
    Oo7SecretStore, OrionNodeConfig, OrionQuicNode, PairedDeviceStore, SecretStore, TransferEngine,
    TransferState, VisibilityMode, EVERYONE_NEARBY_TIMEOUT_SECS,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::{mpsc, watch, Mutex, Notify};
use tokio::time::Instant;
use tracing::{info, warn};

use crate::ble::{BleDriverParams, BlePeerDriver, DEFAULT_CAPABILITY_FLAGS};
use crate::ipc::{
    daemon_socket_path, decode_request, encode_response, prepare_daemon_socket_dir,
    set_daemon_socket_permissions, DaemonStatus, IpcRequest, IpcResponse, MAX_IPC_FRAME_BYTES,
};
use crate::notifications::{NotificationAction, NotificationsClient};
use crate::tray::{spawn_settings_process, spawn_tray, TrayCommand, TrayState};

const DEVICES_POLL_INTERVAL: Duration = Duration::from_millis(200);
const TRAY_REFRESH_INTERVAL: Duration = Duration::from_secs(1);
const LAN_ENDPOINT_CHANNEL_SLOTS: usize = 32;

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
        let known_peers = Arc::new(Mutex::new(HashSet::new()));

        let quic_node = if let Some(ref secrets) = secret_store {
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
            Some(Arc::new(node))
        } else {
            warn!(
                event = "quic_unavailable_degraded",
                reason = "keyring_absent"
            );
            None
        };

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

        let mdns = if options.enable_mdns {
            let discovery = match options.mdns_port {
                Some(port) => MdnsDiscovery::new_with_port(port),
                None => MdnsDiscovery::new(),
            };
            if let Some(ref node) = quic_node {
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
        } else {
            None
        };

        let mut tasks = Vec::new();

        if let (Some(ref discovery), Some(ref node), Some(ref secrets)) =
            (&mdns, &quic_node, &secret_store)
        {
            if let Some(mut rx) = discovery.browse(&options.local_device_id) {
                let store = device_store.clone();
                let secrets = Arc::clone(secrets);
                let node = Arc::clone(node);
                tasks.push(tokio::spawn(async move {
                    while let Some(peer) = rx.recv().await {
                        let endpoint = peer.to_lan_endpoint();
                        if let Err(err) = apply_discovered_lan_endpoint(
                            &endpoint,
                            &store,
                            secrets.as_ref(),
                            &node,
                        ) {
                            warn!(
                                event = "mdns_peer_update_skipped",
                                peer_device_id = %endpoint.device_id,
                                error = %err
                            );
                        }
                    }
                }));
            }
        }

        if let Some(ref node) = quic_node {
            let accept_node = Arc::clone(node);
            let accept_engine = Arc::clone(&transfer_engine);
            let accept_store = device_store.clone();
            let accept_secrets = secret_store.clone();
            let accept_known = Arc::clone(&known_peers);
            tasks.push(tokio::spawn(async move {
                loop {
                    match accept_node.accept_authenticated().await {
                        Ok(Some((peer_device_id, conn))) => {
                            if let Err(err) = sync_paired_devices_and_policies(
                                &accept_store,
                                accept_secrets.as_deref(),
                                Some(accept_node.as_ref()),
                                &accept_engine,
                                &accept_known,
                            )
                            .await
                            {
                                warn!(event = "peer_sync_on_accept_failed", error = %err);
                            }
                            let conn_engine = Arc::clone(&accept_engine);
                            tokio::spawn(async move {
                                let _ = conn_engine
                                    .handle_incoming_connection(peer_device_id, conn)
                                    .await;
                            });
                        }
                        Ok(None) => break,
                        Err(err) => {
                            warn!(event = "quic_accept_failed", error = %err);
                        }
                    }
                }
            }));
        }

        {
            let poll_store = device_store.clone();
            let poll_secrets = secret_store.clone();
            let poll_node = quic_node.clone();
            let poll_engine = Arc::clone(&transfer_engine);
            let poll_known = Arc::clone(&known_peers);
            tasks.push(tokio::spawn(async move {
                let mut last_mtime = devices_file_mtime(&poll_store);
                loop {
                    tokio::time::sleep(DEVICES_POLL_INTERVAL).await;
                    let current = devices_file_mtime(&poll_store);
                    if current != last_mtime {
                        last_mtime = current;
                        if let Err(err) = sync_paired_devices_and_policies(
                            &poll_store,
                            poll_secrets.as_deref(),
                            poll_node.as_deref(),
                            &poll_engine,
                            &poll_known,
                        )
                        .await
                        {
                            warn!(event = "devices_reload_failed", error = %err);
                        }
                    }
                }
            }));
        }

        let (lan_watch_tx, lan_watch_rx) =
            watch::channel(quic_node.as_ref().map(|q| q.local_lan_endpoint()));
        if options.enable_bluez {
            if let Some(ref secrets) = secret_store {
                let (ble_lan_tx, mut ble_lan_rx) =
                    mpsc::channel::<v1::LanEndpoint>(LAN_ENDPOINT_CHANNEL_SLOTS);
                if let Some(ref node) = quic_node {
                    let store = device_store.clone();
                    let secrets = Arc::clone(secrets);
                    let node = Arc::clone(node);
                    tasks.push(tokio::spawn(async move {
                        while let Some(endpoint) = ble_lan_rx.recv().await {
                            if let Err(err) = apply_discovered_lan_endpoint(
                                &endpoint,
                                &store,
                                secrets.as_ref(),
                                &node,
                            ) {
                                warn!(
                                    event = "ble_lan_endpoint_ignored",
                                    peer_device_id = %endpoint.device_id,
                                    error = %err
                                );
                            }
                        }
                    }));
                }

                let local_id = options.local_device_id.clone();
                let display = options.display_name.clone();
                let rx = visibility_rx.clone();
                let secrets = Arc::clone(secrets);
                let store = device_store.clone();
                tasks.push(tokio::spawn(async move {
                    if let Err(err) = BlePeerDriver::run_bluez(BleDriverParams {
                        local_device_id: local_id,
                        display_name: display,
                        capability_flags: DEFAULT_CAPABILITY_FLAGS,
                        visibility_rx: rx,
                        secret_store: secrets,
                        device_store: store,
                        discovered_tx: None,
                        pairing_decision_rx: None,
                        pairing_event_tx: None,
                        local_lan_endpoint_rx: Some(lan_watch_rx),
                        lan_endpoint_tx: Some(ble_lan_tx),
                    })
                    .await
                    {
                        warn!(event = "ble_unavailable_degraded", error = %err);
                    }
                }));
            }
        }

        let mdns_degraded = mdns.as_ref().map(|m| m.is_degraded()).unwrap_or(false);
        let shared_ctx = IpcDispatchContext {
            local_device_id: options.local_device_id.clone(),
            visibility_tx: visibility_tx.clone(),
            visibility_rx: visibility_rx.clone(),
            config_state: Arc::clone(&config_state),
            config_path: config_path.clone(),
            everyone_nearby_timeout: options.everyone_nearby_timeout,
            everyone_deadline: Arc::clone(&everyone_deadline),
            deadline_notify: Arc::clone(&deadline_notify),
            refresh_notify: Arc::clone(&refresh_notify),
            device_store: device_store.clone(),
            secret_store: secret_store.clone(),
            quic_node: quic_node.clone(),
            transfer_engine: Arc::clone(&transfer_engine),
            known_peers: Arc::clone(&known_peers),
            mdns_degraded,
            mdns: mdns.clone(),
        };

        let mut bound_socket_path: Option<PathBuf> = None;
        if options.enable_ipc {
            let sock_path = options
                .ipc_socket_path
                .clone()
                .unwrap_or_else(daemon_socket_path);
            prepare_daemon_socket_dir(&sock_path)?;
            let listener = UnixListener::bind(&sock_path)?;
            set_daemon_socket_permissions(&sock_path)?;
            bound_socket_path = Some(sock_path.clone());
            info!(event = "ipc_listening", socket = %sock_path.display());

            let ctx = shared_ctx.clone();
            tasks.push(tokio::spawn(async move {
                loop {
                    match listener.accept().await {
                        Ok((stream, _)) => {
                            let req_ctx = ctx.clone();
                            tokio::spawn(async move {
                                if let Err(err) = handle_ipc_connection(stream, req_ctx).await {
                                    warn!(event = "ipc_request_failed", error = %err);
                                }
                            });
                        }
                        Err(err) => {
                            warn!(event = "ipc_accept_failed", error = %err);
                            break;
                        }
                    }
                }
            }));
        }

        let tray_handle = if options.enable_tray {
            let (tray_cmd_tx, mut tray_cmd_rx) = mpsc::unbounded_channel();
            let initial_tray_state = build_tray_state(&shared_ctx).await;
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

        if let Some(handle) = tray_handle {
            handle.shutdown().await;
        }
        drop(shared_ctx);
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

#[derive(Clone)]
struct IpcDispatchContext {
    local_device_id: String,
    visibility_tx: watch::Sender<VisibilityMode>,
    visibility_rx: watch::Receiver<VisibilityMode>,
    config_state: Arc<Mutex<DaemonConfig>>,
    config_path: Option<PathBuf>,
    everyone_nearby_timeout: Duration,
    everyone_deadline: Arc<Mutex<Option<Instant>>>,
    deadline_notify: Arc<Notify>,
    refresh_notify: Arc<Notify>,
    device_store: PairedDeviceStore,
    secret_store: Option<Arc<dyn SecretStore>>,
    quic_node: Option<Arc<OrionQuicNode>>,
    transfer_engine: Arc<TransferEngine>,
    known_peers: Arc<Mutex<HashSet<String>>>,
    mdns_degraded: bool,
    mdns: Option<Arc<MdnsDiscovery>>,
}

async fn handle_ipc_connection(
    mut stream: UnixStream,
    ctx: IpcDispatchContext,
) -> Result<(), KeynearError> {
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).await?;
    let frame_len = u32::from_be_bytes(len_buf) as usize;
    if frame_len == 0 {
        return Err(KeynearError::Protocol(
            "IPC request frame length must be positive".to_string(),
        ));
    }
    if frame_len > MAX_IPC_FRAME_BYTES {
        return Err(KeynearError::FrameTooLarge {
            size: frame_len,
            max: MAX_IPC_FRAME_BYTES,
        });
    }
    let mut payload = vec![0u8; frame_len];
    stream.read_exact(&mut payload).await?;

    let response = match decode_request(&payload) {
        Ok(req) => dispatch_ipc_request(req, &ctx).await,
        Err(err) => IpcResponse::Error {
            message: err.to_string(),
        },
    };

    let encoded = encode_response(&response)?;
    let resp_len = u32::try_from(encoded.len()).map_err(|_| KeynearError::FrameTooLarge {
        size: encoded.len(),
        max: MAX_IPC_FRAME_BYTES,
    })?;
    stream.write_all(&resp_len.to_be_bytes()).await?;
    stream.write_all(&encoded).await?;
    stream.flush().await?;
    Ok(())
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

async fn build_daemon_status(ctx: &IpcDispatchContext) -> Result<DaemonStatus, KeynearError> {
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

fn compute_remaining_secs(mode: VisibilityMode, deadline: Option<Instant>) -> Option<u64> {
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

async fn apply_visibility_change(mode: VisibilityMode, ctx: &IpcDispatchContext) {
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

async fn apply_daemon_config(
    new_config: &DaemonConfig,
    ctx: &IpcDispatchContext,
) -> Result<DaemonConfig, KeynearError> {
    new_config.validate()?;
    ctx.transfer_engine
        .set_auto_accept_from_paired(new_config.auto_accept_from_paired);
    ctx.transfer_engine
        .set_incoming_directory(PathBuf::from(&new_config.incoming_directory))?;
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

async fn build_tray_state(ctx: &IpcDispatchContext) -> TrayState {
    let visibility_mode = *ctx.visibility_rx.borrow();
    let deadline = *ctx.everyone_deadline.lock().await;
    let everyone_remaining_secs = compute_remaining_secs(visibility_mode, deadline);
    let auto_accept_from_paired = ctx.config_state.lock().await.auto_accept_from_paired;
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
        paired_devices,
        transfers,
        mdns_degraded,
    }
}

async fn handle_tray_command(cmd: TrayCommand, ctx: &IpcDispatchContext) {
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

async fn handle_notification_action(action: NotificationAction, ctx: &IpcDispatchContext) {
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

async fn sync_transfer_notifications(
    client: &NotificationsClient,
    ctx: &IpcDispatchContext,
    sent: &mut HashMap<String, (u32, TransferState, u64)>,
) {
    let devices = ctx.device_store.load().unwrap_or_default();
    let cfg = ctx.config_state.lock().await.clone();
    let transfers = ctx.transfer_engine.list_transfers();
    for session in transfers {
        let peer_name = devices
            .iter()
            .find(|d| d.device_id == session.peer_device_id)
            .map(|d| d.display_name.as_str())
            .unwrap_or(session.peer_device_id.as_str());
        if session.state == TransferState::Completed && !cfg.notify_auto_accept_complete {
            continue;
        }
        let prev = sent.get(&session.transfer_id).copied();
        let should_send = match prev {
            None => matches!(
                session.state,
                TransferState::PendingConsent | TransferState::Active
            ),
            Some((_, prev_state, prev_bytes)) => {
                prev_state != session.state
                    || (session.state == TransferState::Active
                        && session.transferred_bytes != prev_bytes)
            }
        };
        if !should_send {
            continue;
        }
        let replaces_id = prev.map(|(nid, _, _)| nid).unwrap_or(0);
        if let Some(nid) = client
            .notify_transfer_session(&session, peer_name, replaces_id)
            .await
        {
            sent.insert(
                session.transfer_id,
                (nid, session.state, session.transferred_bytes),
            );
        }
    }
}

async fn dispatch_ipc_request(req: IpcRequest, ctx: &IpcDispatchContext) -> IpcResponse {
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

pub async fn sync_paired_devices_and_policies(
    device_store: &PairedDeviceStore,
    secret_store: Option<&dyn SecretStore>,
    quic_node: Option<&OrionQuicNode>,
    transfer_engine: &TransferEngine,
    known_peers: &Mutex<HashSet<String>>,
) -> Result<(), KeynearError> {
    let devices = device_store.load()?;
    let current_ids: HashSet<String> = devices.iter().map(|d| d.device_id.clone()).collect();

    let removed_ids: Vec<String> = {
        let mut guard = known_peers.lock().await;
        let removed: Vec<String> = guard.difference(&current_ids).cloned().collect();
        *guard = current_ids;
        removed
    };

    for removed_id in removed_ids {
        transfer_engine.remove_peer_policy(&removed_id);
        if let Some(node) = quic_node {
            node.remove_peer(&removed_id)?;
        }
    }

    for dev in &devices {
        transfer_engine.set_peer_auto_accept(&dev.device_id, dev.auto_accept_files)?;
    }

    if let (Some(node), Some(secrets)) = (quic_node, secret_store) {
        node.sync_paired_devices(&devices, secrets)?;
    }

    Ok(())
}

pub fn apply_discovered_lan_endpoint(
    endpoint: &v1::LanEndpoint,
    device_store: &PairedDeviceStore,
    secret_store: &dyn SecretStore,
    quic_node: &OrionQuicNode,
) -> Result<(), KeynearError> {
    let Some(_paired) = device_store.get(&endpoint.device_id)? else {
        return Err(KeynearError::UnauthorizedPeer(format!(
            "ignoring LAN endpoint for unpaired peer '{}'",
            endpoint.device_id
        )));
    };
    let Some(secrets) = secret_store.get_peer_secrets(&endpoint.device_id)? else {
        return Err(KeynearError::UnauthorizedPeer(format!(
            "missing reconnect secret for peer '{}'",
            endpoint.device_id
        )));
    };

    let eid = parse_endpoint_id_bytes(&endpoint.endpoint_id)?;
    let addrs: Vec<SocketAddr> = endpoint
        .socket_addrs
        .iter()
        .filter_map(|raw| raw.trim().parse::<SocketAddr>().ok())
        .collect();
    if addrs.is_empty() {
        return Err(KeynearError::Validation {
            field: "socketAddrs",
            message: "LAN endpoint contains no valid socket addresses".to_string(),
        });
    }

    quic_node.register_peer(&endpoint.device_id, eid, addrs, secrets.reconnect_secret)?;

    let eid_hex = hex::encode(eid.as_bytes());
    let addrs_csv = endpoint.socket_addrs.join(",");
    device_store.update_lan_endpoint(&endpoint.device_id, &eid_hex, &addrs_csv)?;
    Ok(())
}

fn devices_file_mtime(device_store: &PairedDeviceStore) -> Option<SystemTime> {
    std::fs::metadata(device_store.path())
        .ok()
        .and_then(|meta| meta.modified().ok())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::UNIX_EPOCH;

    use keynear_core::{
        commit_paired_device, DeviceRole, InMemorySecretStore, PairedDevice, PeerSecrets,
    };
    use tokio::sync::oneshot;

    use super::*;
    use crate::ipc::send_request_to;

    fn temp_service_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("keynear-svc-{label}-{nanos}"));
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    #[tokio::test]
    async fn shutdown_loop_completes_once() {
        let dir = temp_service_dir("shutdown");
        let config = DaemonConfig {
            incoming_directory: dir.join("incoming").display().to_string(),
            ..Default::default()
        };
        let (tx, rx) = oneshot::channel::<()>();
        let completions = Arc::new(AtomicUsize::new(0));
        let completions_clone = Arc::clone(&completions);
        let options = RuntimeOptions {
            enable_bluez: false,
            enable_mdns: false,
            enable_ipc: false,
            enable_tray: false,
            enable_notifications: false,
            quic_bind_addr: Some("127.0.0.1:0".parse().expect("loopback")),
            ..Default::default()
        };

        let runtime_dir = dir.clone();
        let task = tokio::spawn(async move {
            let result = DaemonRuntime::run_with_deps(
                &config,
                options,
                PairedDeviceStore::new(runtime_dir.join("devices.toml")),
                Some(Arc::new(InMemorySecretStore::new())),
                async move {
                    let _ = rx.await;
                },
            )
            .await;
            completions_clone.fetch_add(1, Ordering::SeqCst);
            result
        });

        tx.send(()).expect("send shutdown signal");
        task.await
            .expect("join runtime task")
            .expect("runtime exits cleanly");
        assert_eq!(completions.load(Ordering::SeqCst), 1);
        let _ = fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn invalid_config_fails_before_awaiting_shutdown() {
        let config = DaemonConfig {
            incoming_directory: String::new(),
            ..Default::default()
        };
        let err = DaemonRuntime::run_with_shutdown_with_config_path(
            &config,
            None,
            std::future::pending::<()>(),
        )
        .await
        .expect_err("invalid config must fail immediately");
        assert!(matches!(
            err,
            KeynearError::Validation {
                field: "incomingDirectory",
                ..
            }
        ));
    }

    #[tokio::test]
    async fn degrades_gracefully_when_bluetooth_and_keyring_absent() {
        let dir = temp_service_dir("degraded");
        let config = DaemonConfig {
            visibility_mode: VisibilityMode::EveryoneNearby,
            incoming_directory: dir.join("incoming").display().to_string(),
            ..Default::default()
        };
        let (tx, rx) = oneshot::channel::<()>();
        let options = RuntimeOptions {
            everyone_nearby_timeout: Duration::from_millis(25),
            enable_bluez: true,
            enable_mdns: false,
            enable_ipc: false,
            enable_tray: false,
            enable_notifications: false,
            ..Default::default()
        };

        let runtime_dir = dir.clone();
        let task = tokio::spawn(async move {
            DaemonRuntime::run_with_deps(
                &config,
                options,
                PairedDeviceStore::new(runtime_dir.join("devices.toml")),
                None,
                async move {
                    let _ = rx.await;
                },
            )
            .await
        });

        tokio::time::sleep(Duration::from_millis(45)).await;
        tx.send(()).expect("shutdown signal");
        task.await
            .expect("join task")
            .expect("daemon degrades cleanly without bluetooth or keyring");
        let _ = fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn daemon_ipc_handles_status_send_list_consent_and_reload() {
        let dir = temp_service_dir("ipc-transfer");
        let alice_dir = dir.join("alice");
        let bob_dir = dir.join("bob");
        let alice_incoming = alice_dir.join("incoming");
        let bob_incoming = bob_dir.join("incoming");
        fs::create_dir_all(&alice_dir).expect("alice dir");
        fs::create_dir_all(&bob_incoming).expect("bob incoming");

        let alice_store = PairedDeviceStore::new(alice_dir.join("devices.toml"));
        let bob_store = PairedDeviceStore::new(bob_dir.join("devices.toml"));
        let alice_secrets = Arc::new(InMemorySecretStore::new());
        let bob_secrets = Arc::new(InMemorySecretStore::new());
        let shared_secret = [0x44u8; 32];

        let alice_sock = alice_dir.join("daemon.sock");
        let bob_sock = bob_dir.join("daemon.sock");

        let bob_config = DaemonConfig {
            incoming_directory: bob_incoming.display().to_string(),
            ..Default::default()
        };
        let alice_config = DaemonConfig {
            incoming_directory: alice_incoming.display().to_string(),
            ..Default::default()
        };

        let (bob_tx, bob_rx) = oneshot::channel::<()>();
        let bob_task = tokio::spawn({
            let bob_store = bob_store.clone();
            let bob_secrets = Arc::clone(&bob_secrets);
            let bob_sock = bob_sock.clone();
            async move {
                DaemonRuntime::run_with_deps(
                    &bob_config,
                    RuntimeOptions {
                        local_device_id: "bob-linux".to_string(),
                        display_name: "Bob".to_string(),
                        enable_bluez: false,
                        enable_mdns: false,
                        enable_ipc: true,
                        enable_tray: false,
                        enable_notifications: false,
                        ipc_socket_path: Some(bob_sock),
                        quic_bind_addr: Some("127.0.0.1:0".parse().expect("addr")),
                        ..Default::default()
                    },
                    bob_store,
                    Some(bob_secrets),
                    async move {
                        let _ = bob_rx.await;
                    },
                )
                .await
            }
        });

        let (alice_tx, alice_rx) = oneshot::channel::<()>();
        let alice_task = tokio::spawn({
            let alice_store = alice_store.clone();
            let alice_secrets = Arc::clone(&alice_secrets);
            let alice_sock = alice_sock.clone();
            async move {
                DaemonRuntime::run_with_deps(
                    &alice_config,
                    RuntimeOptions {
                        local_device_id: "alice-linux".to_string(),
                        display_name: "Alice".to_string(),
                        enable_bluez: false,
                        enable_mdns: false,
                        enable_ipc: true,
                        enable_tray: false,
                        enable_notifications: false,
                        ipc_socket_path: Some(alice_sock),
                        quic_bind_addr: Some("127.0.0.1:0".parse().expect("addr")),
                        ..Default::default()
                    },
                    alice_store,
                    Some(alice_secrets),
                    async move {
                        let _ = alice_rx.await;
                    },
                )
                .await
            }
        });

        let bob_status = wait_for_status(&bob_sock).await;
        let alice_status = wait_for_status(&alice_sock).await;
        assert_eq!(bob_status.device_id, "bob-linux");
        assert!(!bob_status.endpoint_id.is_empty());
        assert!(!bob_status.listen_addrs.is_empty());

        commit_paired_device(
            &alice_store,
            alice_secrets.as_ref(),
            PairedDevice {
                device_id: "bob-linux".to_string(),
                display_name: "Bob".to_string(),
                device_role: DeviceRole::Desktop,
                static_x25519_pubkey: "bb".repeat(32),
                last_ble_address: String::new(),
                last_lan_address: bob_status.listen_addrs.join(","),
                last_endpoint_id: bob_status.endpoint_id.clone(),
                capability_flags: DEFAULT_CAPABILITY_FLAGS,
                auto_accept_files: false,
                paired_at: "1".to_string(),
            },
            &PeerSecrets {
                irk: shared_secret,
                reconnect_secret: shared_secret,
            },
        )
        .expect("commit bob on alice");

        commit_paired_device(
            &bob_store,
            bob_secrets.as_ref(),
            PairedDevice {
                device_id: "alice-linux".to_string(),
                display_name: "Alice".to_string(),
                device_role: DeviceRole::Desktop,
                static_x25519_pubkey: "aa".repeat(32),
                last_ble_address: String::new(),
                last_lan_address: alice_status.listen_addrs.join(","),
                last_endpoint_id: alice_status.endpoint_id.clone(),
                capability_flags: DEFAULT_CAPABILITY_FLAGS,
                auto_accept_files: false,
                paired_at: "1".to_string(),
            },
            &PeerSecrets {
                irk: shared_secret,
                reconnect_secret: shared_secret,
            },
        )
        .expect("commit alice on bob");

        wait_for_paired_devices(&bob_sock, 1).await;
        wait_for_paired_devices(&alice_sock, 1).await;

        let sample_file = dir.join("note.txt");
        let payload = b"orion daemon ipc transfer payload";
        fs::write(&sample_file, payload).expect("write sample file");

        let send_sock = alice_sock.clone();
        let send_path = sample_file.clone();
        let mut send_handle = tokio::task::spawn_blocking(move || {
            send_request_to(
                &send_sock,
                &IpcRequest::SendFiles {
                    device_id: "bob-linux".to_string(),
                    paths: vec![send_path],
                },
            )
        });

        let pending_id = wait_for_pending_transfer(&bob_sock, &mut send_handle).await;
        let bob_sock_accept = bob_sock.clone();
        let accepted_resp = tokio::task::spawn_blocking(move || {
            send_request_to(
                &bob_sock_accept,
                &IpcRequest::AcceptTransfer {
                    transfer_id: pending_id,
                },
            )
        })
        .await
        .expect("join accept")
        .expect("accept ipc ok");
        assert!(matches!(accepted_resp, IpcResponse::Transfer(_)));

        let send_resp = send_handle.await.expect("join send").expect("send ipc ok");
        let IpcResponse::Transfer(sent_session) = send_resp else {
            panic!("expected completed transfer response, got {send_resp:?}");
        };
        assert_eq!(sent_session.state, TransferState::Completed);
        assert_eq!(
            fs::read(bob_incoming.join("note.txt")).expect("read received file"),
            payload
        );

        let _ = alice_tx.send(());
        let _ = bob_tx.send(());
        alice_task.await.expect("join alice").expect("alice exit");
        bob_task.await.expect("join bob").expect("bob exit");
        let _ = fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn daemon_ipc_handles_visibility_config_rename_and_device_auto_accept() {
        let dir = temp_service_dir("ipc-control");
        let incoming_dir = dir.join("incoming");
        let incoming_dir_updated = dir.join("incoming-updated");
        fs::create_dir_all(&incoming_dir).expect("create incoming dir");
        fs::create_dir_all(&incoming_dir_updated).expect("create updated incoming dir");

        let config_path = dir.join("config.toml");
        let sock_path = dir.join("daemon.sock");
        let device_store = PairedDeviceStore::new(dir.join("devices.toml"));
        let secret_store = Arc::new(InMemorySecretStore::new());

        let initial_config = DaemonConfig {
            incoming_directory: incoming_dir.display().to_string(),
            visibility_mode: VisibilityMode::Hidden,
            ..Default::default()
        };
        initial_config
            .save_to_path(&config_path)
            .expect("save initial config");

        commit_paired_device(
            &device_store,
            secret_store.as_ref(),
            PairedDevice {
                device_id: "pixel-9".to_string(),
                display_name: "Pixel 9".to_string(),
                device_role: DeviceRole::Mobile,
                static_x25519_pubkey: "cc".repeat(32),
                last_ble_address: String::new(),
                last_lan_address: "127.0.0.1:4242".to_string(),
                last_endpoint_id: "dd".repeat(32),
                capability_flags: DEFAULT_CAPABILITY_FLAGS,
                auto_accept_files: false,
                paired_at: "1".to_string(),
            },
            &PeerSecrets {
                irk: [0x21u8; 32],
                reconnect_secret: [0x31u8; 32],
            },
        )
        .expect("seed paired device");

        let (tx, rx) = oneshot::channel::<()>();
        let runtime_task = tokio::spawn({
            let store = device_store.clone();
            let secrets = Arc::clone(&secret_store);
            let sock = sock_path.clone();
            let cfg_path = config_path.clone();
            let cfg = initial_config.clone();
            async move {
                DaemonRuntime::run_with_deps(
                    &cfg,
                    RuntimeOptions {
                        local_device_id: "host-linux".to_string(),
                        display_name: "Host".to_string(),
                        everyone_nearby_timeout: Duration::from_secs(120),
                        enable_bluez: false,
                        enable_mdns: false,
                        enable_ipc: true,
                        enable_tray: false,
                        enable_notifications: false,
                        config_path: Some(cfg_path),
                        ipc_socket_path: Some(sock),
                        quic_bind_addr: Some("127.0.0.1:0".parse().expect("addr")),
                        ..Default::default()
                    },
                    store,
                    Some(secrets),
                    async move {
                        let _ = rx.await;
                    },
                )
                .await
            }
        });

        let initial_status = wait_for_status(&sock_path).await;
        assert_eq!(initial_status.visibility_mode, VisibilityMode::Hidden);
        assert_eq!(initial_status.everyone_remaining_secs, None);

        let everyone_resp = {
            let sock = sock_path.clone();
            tokio::task::spawn_blocking(move || {
                send_request_to(
                    &sock,
                    &IpcRequest::SetVisibility {
                        mode: VisibilityMode::EveryoneNearby,
                    },
                )
            })
            .await
            .expect("join set visibility")
            .expect("set visibility ok")
        };
        let IpcResponse::Status(everyone_status) = everyone_resp else {
            panic!("expected status response, got {everyone_resp:?}");
        };
        assert_eq!(
            everyone_status.visibility_mode,
            VisibilityMode::EveryoneNearby
        );
        assert!(everyone_status.everyone_remaining_secs.unwrap_or(0) > 0);
        let saved_after_vis =
            DaemonConfig::load_or_default(&config_path).expect("reload persisted config");
        assert_eq!(
            saved_after_vis.visibility_mode,
            VisibilityMode::EveryoneNearby
        );

        let paired_resp = {
            let sock = sock_path.clone();
            tokio::task::spawn_blocking(move || {
                send_request_to(
                    &sock,
                    &IpcRequest::SetVisibility {
                        mode: VisibilityMode::PairedOnly,
                    },
                )
            })
            .await
            .expect("join paired visibility")
            .expect("set paired visibility ok")
        };
        let IpcResponse::Status(paired_status) = paired_resp else {
            panic!("expected status response, got {paired_resp:?}");
        };
        assert_eq!(paired_status.visibility_mode, VisibilityMode::PairedOnly);
        assert_eq!(paired_status.everyone_remaining_secs, None);

        let devices_resp = {
            let sock = sock_path.clone();
            tokio::task::spawn_blocking(move || send_request_to(&sock, &IpcRequest::ListDevices))
                .await
                .expect("join list devices")
                .expect("list devices ok")
        };
        let IpcResponse::Devices(devices) = devices_resp else {
            panic!("expected devices response, got {devices_resp:?}");
        };
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].device_id, "pixel-9");

        let renamed_resp = {
            let sock = sock_path.clone();
            tokio::task::spawn_blocking(move || {
                send_request_to(
                    &sock,
                    &IpcRequest::RenameDevice {
                        device_id: "pixel-9".to_string(),
                        display_name: "Pixel Desk".to_string(),
                    },
                )
            })
            .await
            .expect("join rename device")
            .expect("rename device ok")
        };
        let IpcResponse::Device(renamed_device) = renamed_resp else {
            panic!("expected device response, got {renamed_resp:?}");
        };
        assert_eq!(renamed_device.display_name, "Pixel Desk");

        let auto_accept_resp = {
            let sock = sock_path.clone();
            tokio::task::spawn_blocking(move || {
                send_request_to(
                    &sock,
                    &IpcRequest::SetDeviceAutoAccept {
                        device_id: "pixel-9".to_string(),
                        enabled: true,
                    },
                )
            })
            .await
            .expect("join set device auto accept")
            .expect("set device auto accept ok")
        };
        let IpcResponse::Device(auto_device) = auto_accept_resp else {
            panic!("expected device response, got {auto_accept_resp:?}");
        };
        assert!(auto_device.auto_accept_files);

        let get_cfg_resp = {
            let sock = sock_path.clone();
            tokio::task::spawn_blocking(move || send_request_to(&sock, &IpcRequest::GetConfig))
                .await
                .expect("join get config")
                .expect("get config ok")
        };
        let IpcResponse::Config(mut updated_cfg) = get_cfg_resp else {
            panic!("expected config response, got {get_cfg_resp:?}");
        };
        updated_cfg.auto_accept_from_paired = true;
        updated_cfg.incoming_directory = incoming_dir_updated.display().to_string();

        let update_cfg_resp = {
            let sock = sock_path.clone();
            let cfg_req = updated_cfg.clone();
            tokio::task::spawn_blocking(move || {
                send_request_to(&sock, &IpcRequest::UpdateConfig { config: cfg_req })
            })
            .await
            .expect("join update config")
            .expect("update config ok")
        };
        let IpcResponse::Config(returned_cfg) = update_cfg_resp else {
            panic!("expected config response, got {update_cfg_resp:?}");
        };
        assert!(returned_cfg.auto_accept_from_paired);
        assert_eq!(
            returned_cfg.incoming_directory,
            incoming_dir_updated.display().to_string()
        );
        let persisted_cfg =
            DaemonConfig::load_or_default(&config_path).expect("reload updated cfg");
        assert!(persisted_cfg.auto_accept_from_paired);
        assert_eq!(
            persisted_cfg.incoming_directory,
            incoming_dir_updated.display().to_string()
        );

        let _ = tx.send(());
        runtime_task
            .await
            .expect("join runtime")
            .expect("runtime exit");
        let _ = fs::remove_dir_all(&dir);
    }

    async fn wait_for_status(sock_path: &Path) -> DaemonStatus {
        wait_for_status_with(sock_path, |_| true, "daemon status").await
    }

    async fn wait_for_paired_devices(sock_path: &Path, min_count: u32) -> DaemonStatus {
        let description = format!("paired_device_count >= {min_count}");
        wait_for_status_with(
            sock_path,
            |status| status.paired_device_count >= min_count,
            &description,
        )
        .await
    }

    async fn wait_for_status_with<F>(
        sock_path: &Path,
        mut predicate: F,
        description: &str,
    ) -> DaemonStatus
    where
        F: FnMut(&DaemonStatus) -> bool,
    {
        let mut last_err = None;
        for _ in 0..80 {
            let p = sock_path.to_path_buf();
            match tokio::task::spawn_blocking(move || send_request_to(&p, &IpcRequest::Status))
                .await
            {
                Ok(Ok(IpcResponse::Status(status))) if predicate(&status) => return status,
                Ok(Ok(IpcResponse::Status(_))) => {}
                Ok(Ok(other)) => last_err = Some(format!("unexpected IPC response: {other:?}")),
                Ok(Err(err)) => last_err = Some(err.to_string()),
                Err(err) => last_err = Some(err.to_string()),
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        panic!(
            "timed out waiting for {description} on {} (last error: {last_err:?})",
            sock_path.display()
        );
    }

    async fn wait_for_pending_transfer(
        sock_path: &Path,
        send_handle: &mut tokio::task::JoinHandle<Result<IpcResponse, KeynearError>>,
    ) -> String {
        for _ in 0..80 {
            let p = sock_path.to_path_buf();
            let list_fut = tokio::task::spawn_blocking(move || {
                send_request_to(&p, &IpcRequest::ListTransfers)
            });
            tokio::select! {
                send_res = &mut *send_handle => {
                    panic!("send task finished before PendingConsent appeared: {send_res:?}");
                }
                list_res = list_fut => {
                    if let Ok(Ok(IpcResponse::Transfers(list))) = list_res {
                        if let Some(session) = list
                            .into_iter()
                            .find(|s| s.state == TransferState::PendingConsent)
                        {
                            return session.transfer_id;
                        }
                    }
                }
            }
            tokio::select! {
                send_res = &mut *send_handle => {
                    panic!("send task finished while waiting for PendingConsent: {send_res:?}");
                }
                _ = tokio::time::sleep(Duration::from_millis(25)) => {}
            }
        }
        panic!("timed out waiting for pending consent transfer");
    }
}
