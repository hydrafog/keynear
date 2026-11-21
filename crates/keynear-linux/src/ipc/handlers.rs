mod actions;
mod config;
mod dispatch;
mod status;

pub(crate) use actions::{handle_notification_action, handle_tray_command};
pub(crate) use dispatch::dispatch_ipc_request;
#[allow(unused_imports)]
pub(crate) use status::{build_daemon_status, build_tray_state};

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use keynear_core::{
    ClipboardEngine, MdnsDiscovery, OrionQuicNode, PairedDeviceStore, SecretStore, TransferEngine,
};
use keynear_protocol::DaemonConfig;
use tokio::sync::{watch, Mutex, Notify};
use tokio::time::Instant;
use zeroize::Zeroizing;

use crate::clipboard::ClipboardMonitorHandle;

#[derive(Clone)]
pub(crate) struct IpcDispatchContext {
    pub local_device_id: String,
    pub visibility_tx: watch::Sender<keynear_protocol::VisibilityMode>,
    pub visibility_rx: watch::Receiver<keynear_protocol::VisibilityMode>,
    pub config_state: Arc<Mutex<DaemonConfig>>,
    pub config_watch_tx: watch::Sender<DaemonConfig>,
    pub config_path: Option<PathBuf>,
    pub everyone_nearby_timeout: Duration,
    pub everyone_deadline: Arc<Mutex<Option<Instant>>>,
    pub deadline_notify: Arc<Notify>,
    pub refresh_notify: Arc<Notify>,
    pub device_store: PairedDeviceStore,
    pub secret_store: Option<Arc<dyn SecretStore>>,
    pub quic_node: Option<Arc<OrionQuicNode>>,
    pub transfer_engine: Arc<TransferEngine>,
    pub clipboard_engine: Arc<ClipboardEngine>,
    pub clipboard_handle: ClipboardMonitorHandle,
    pub latest_otp: Arc<Mutex<Option<Zeroizing<String>>>>,
    pub known_peers: Arc<Mutex<HashSet<String>>>,
    pub mdns_degraded: bool,
    pub mdns: Option<Arc<MdnsDiscovery>>,
}
