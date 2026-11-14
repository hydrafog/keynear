use std::collections::HashSet;
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
use tokio::sync::{mpsc, watch, Mutex};
use tracing::{info, warn};

use crate::ble::{BleDriverParams, BlePeerDriver, DEFAULT_CAPABILITY_FLAGS};
use crate::ipc::{
    daemon_socket_path, decode_request, encode_response, prepare_daemon_socket_dir,
    set_daemon_socket_permissions, DaemonStatus, IpcRequest, IpcResponse, MAX_IPC_FRAME_BYTES,
};

const DEVICES_POLL_INTERVAL: Duration = Duration::from_secs(1);
const LAN_ENDPOINT_CHANNEL_SLOTS: usize = 32;

#[derive(Debug, Clone)]
pub struct RuntimeOptions {
    pub local_device_id: String,
    pub display_name: String,
    pub everyone_nearby_timeout: Duration,
    pub enable_bluez: bool,
    pub enable_mdns: bool,
    pub enable_ipc: bool,
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
    pub async fn run_with_shutdown<F>(
        config: &DaemonConfig,
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

        Self::run_with_deps(
            config,
            RuntimeOptions::default(),
            device_store,
            secret_store,
            shutdown,
        )
        .await
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

            let ctx = IpcDispatchContext {
                local_device_id: options.local_device_id.clone(),
                visibility_rx: visibility_rx.clone(),
                device_store: device_store.clone(),
                secret_store: secret_store.clone(),
                quic_node: quic_node.clone(),
                transfer_engine: Arc::clone(&transfer_engine),
                known_peers: Arc::clone(&known_peers),
                mdns_degraded: mdns.as_ref().map(|m| m.is_degraded()).unwrap_or(false),
                mdns: mdns.clone(),
            };
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

        let revert_timer = async {
            if initial_mode == VisibilityMode::EveryoneNearby {
                tokio::time::sleep(options.everyone_nearby_timeout).await;
                let _ = visibility_tx.send(VisibilityMode::PairedOnly);
                info!(event = "visibility_auto_reverted", to = ?VisibilityMode::PairedOnly);
                std::future::pending::<()>().await;
            } else {
                std::future::pending::<()>().await;
            }
        };

        tokio::select! {
            _ = shutdown => {}
            _ = revert_timer => {}
        }

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

    pub async fn run_until_shutdown(config: &DaemonConfig) -> Result<(), KeynearError> {
        let mut sigterm = signal(SignalKind::terminate())?;
        let shutdown = async {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {}
                _ = sigterm.recv() => {}
            }
        };
        Self::run_with_shutdown(config, shutdown).await
    }
}

#[derive(Clone)]
struct IpcDispatchContext {
    local_device_id: String,
    visibility_rx: watch::Receiver<VisibilityMode>,
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

async fn dispatch_ipc_request(req: IpcRequest, ctx: &IpcDispatchContext) -> IpcResponse {
    match req {
        IpcRequest::Status => {
            let devices = ctx.device_store.load().unwrap_or_default();
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
            IpcResponse::Status(DaemonStatus {
                device_id: ctx.local_device_id.clone(),
                endpoint_id,
                listen_addrs,
                visibility_mode: *ctx.visibility_rx.borrow(),
                mdns_degraded,
                active_transfer_count: active_count,
                paired_device_count: devices.len() as u32,
            })
        }
        IpcRequest::ListTransfers => IpcResponse::Transfers(ctx.transfer_engine.list_transfers()),
        IpcRequest::AcceptTransfer { transfer_id } => {
            match ctx.transfer_engine.respond_consent(&transfer_id, true) {
                Ok(session) => IpcResponse::Transfer(session),
                Err(err) => IpcResponse::Error {
                    message: err.to_string(),
                },
            }
        }
        IpcRequest::DeclineTransfer { transfer_id } => {
            match ctx.transfer_engine.respond_consent(&transfer_id, false) {
                Ok(session) => IpcResponse::Transfer(session),
                Err(err) => IpcResponse::Error {
                    message: err.to_string(),
                },
            }
        }
        IpcRequest::CancelTransfer { transfer_id } => {
            match ctx.transfer_engine.cancel_transfer(&transfer_id) {
                Ok(session) => IpcResponse::Transfer(session),
                Err(err) => IpcResponse::Error {
                    message: err.to_string(),
                },
            }
        }
        IpcRequest::SendFiles { device_id, paths } => {
            if let Err(err) = sync_paired_devices_and_policies(
                &ctx.device_store,
                ctx.secret_store.as_deref(),
                ctx.quic_node.as_deref(),
                &ctx.transfer_engine,
                &ctx.known_peers,
            )
            .await
            {
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
                Ok(session) => IpcResponse::Transfer(session),
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
        let err = DaemonRuntime::run_with_shutdown(&config, std::future::pending::<()>())
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

        // Wait for bob's 1s reload tick so alice is in bob's allowlist before dialing.
        tokio::time::sleep(Duration::from_millis(1150)).await;

        let sample_file = dir.join("note.txt");
        let payload = b"orion daemon ipc transfer payload";
        fs::write(&sample_file, payload).expect("write sample file");

        let send_sock = alice_sock.clone();
        let send_path = sample_file.clone();
        let send_handle = tokio::task::spawn_blocking(move || {
            send_request_to(
                &send_sock,
                &IpcRequest::SendFiles {
                    device_id: "bob-linux".to_string(),
                    paths: vec![send_path],
                },
            )
        });

        let pending_id = wait_for_pending_transfer(&bob_sock).await;
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

    async fn wait_for_status(sock_path: &Path) -> DaemonStatus {
        for _ in 0..50 {
            let p = sock_path.to_path_buf();
            if let Ok(Ok(IpcResponse::Status(status))) =
                tokio::task::spawn_blocking(move || send_request_to(&p, &IpcRequest::Status)).await
            {
                return status;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        panic!(
            "timed out waiting for daemon socket {}",
            sock_path.display()
        );
    }

    async fn wait_for_pending_transfer(sock_path: &Path) -> String {
        for _ in 0..80 {
            let p = sock_path.to_path_buf();
            if let Ok(Ok(IpcResponse::Transfers(list))) =
                tokio::task::spawn_blocking(move || send_request_to(&p, &IpcRequest::ListTransfers))
                    .await
            {
                if let Some(session) = list
                    .into_iter()
                    .find(|s| s.state == TransferState::PendingConsent)
                {
                    return session.transfer_id;
                }
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        panic!("timed out waiting for pending consent transfer");
    }
}
