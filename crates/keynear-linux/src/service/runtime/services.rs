use std::collections::HashSet;
use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::Arc;

use keynear_core::{
    ClipboardEngine, ClipboardItem, ClipboardPolicy, MdnsDiscovery, OrionNodeConfig, OrionQuicNode,
    PairedDeviceStore, SecretStore, TransferEngine,
};
use keynear_protocol::KeynearError;
use tokio::sync::{mpsc, Mutex};
use tracing::{info, warn};
use zeroize::Zeroizing;

use crate::ble::DEFAULT_CAPABILITY_FLAGS;
use crate::clipboard::{start_clipboard_monitor, ClipboardChange, ClipboardMonitorHandle};

use super::{RuntimeOptions, RuntimeState};

pub(crate) struct RuntimeServices {
    pub device_store: PairedDeviceStore,
    pub secret_store: Option<Arc<dyn SecretStore>>,
    pub transfer_engine: Arc<TransferEngine>,
    pub incoming_clip_rx: Option<mpsc::UnboundedReceiver<ClipboardItem>>,
    pub clipboard_engine: Arc<ClipboardEngine>,
    pub clip_change_rx: Option<mpsc::UnboundedReceiver<ClipboardChange>>,
    pub clipboard_handle: ClipboardMonitorHandle,
    pub latest_otp: Arc<Mutex<Option<Zeroizing<String>>>>,
    pub known_peers: Arc<Mutex<HashSet<String>>>,
    pub quic_node: Option<Arc<OrionQuicNode>>,
    pub mdns: Option<Arc<MdnsDiscovery>>,
}

type ClipboardInit = (
    Arc<TransferEngine>,
    mpsc::UnboundedReceiver<ClipboardItem>,
    Arc<ClipboardEngine>,
    mpsc::UnboundedReceiver<ClipboardChange>,
    ClipboardMonitorHandle,
    Arc<Mutex<Option<Zeroizing<String>>>>,
    Arc<Mutex<HashSet<String>>>,
);

pub(super) fn init_clipboard(
    config: &keynear_protocol::DaemonConfig,
    options: &RuntimeOptions,
    state: &RuntimeState,
) -> ClipboardInit {
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
        start_clipboard_monitor(state.config_watch_rx.clone(), clip_change_tx)
    } else {
        ClipboardMonitorHandle::headless()
    };
    info!(
        event = "clipboard_monitor_ready",
        backend = clipboard_handle.backend().as_str()
    );
    (
        transfer_engine,
        incoming_clip_rx,
        clipboard_engine,
        clip_change_rx,
        clipboard_handle,
        Arc::new(Mutex::new(None)),
        Arc::new(Mutex::new(HashSet::new())),
    )
}

pub(super) async fn bind_quic_node(
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

pub(super) fn setup_mdns(
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
        register_mdns(&discovery, options, node);
    }
    Some(Arc::new(discovery))
}

fn register_mdns(discovery: &MdnsDiscovery, options: &RuntimeOptions, node: &OrionQuicNode) {
    let (port, addrs) = endpoint_addrs(node);
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

impl RuntimeServices {
    pub(super) fn mdns_degraded(&self) -> bool {
        self.mdns.as_ref().map(|m| m.is_degraded()).unwrap_or(false)
    }
}

fn endpoint_addrs(node: &OrionQuicNode) -> (u16, Vec<IpAddr>) {
    let mut addrs: Vec<IpAddr> = Vec::new();
    let mut port: u16 = 0;
    for sock in node.endpoint_addr().ip_addrs() {
        if port == 0 {
            port = sock.port();
        }
        if !addrs.contains(&sock.ip()) {
            addrs.push(sock.ip());
        }
    }
    (port, addrs)
}
