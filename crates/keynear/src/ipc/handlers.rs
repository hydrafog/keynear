mod actions;
mod config;
mod dispatch;
mod status;

pub(crate) use actions::{handle_notification_action, handle_tray_command};
pub(crate) use dispatch::dispatch_ipc_request;
#[cfg(test)]
pub(crate) use status::build_daemon_status;
pub(crate) use status::build_tray_state;

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use keynear_core::{
    ClipboardEngine, MdnsDiscovery, OrionQuicNode, PairedDeviceStore, SecretStore, TransferEngine,
};
use keynear_protocol::DaemonConfig;
use tokio::sync::{mpsc, watch, Mutex, Notify};
use tokio::time::Instant;
use tracing::{info, warn};
use zeroize::Zeroizing;

use crate::clipboard::ClipboardMonitorHandle;

const OUTBOUND_QUEUE_CAPACITY: usize = 256;

pub(crate) struct OutboundEnvelope {
    pub label: &'static str,
    pub bytes: Vec<u8>,
}

pub(crate) struct OutboundQueue {
    tx: mpsc::Sender<OutboundEnvelope>,
}

impl OutboundQueue {
    fn enqueue(&self, label: &'static str, bytes: Vec<u8>) {
        match self.tx.try_send(OutboundEnvelope { label, bytes }) {
            Ok(()) => info!(event = "continuity_outbound_enqueued", kind = label),
            Err(mpsc::error::TrySendError::Full(_)) => {
                warn!(event = "continuity_outbound_queue_full", kind = label);
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                warn!(event = "continuity_outbound_queue_closed", kind = label);
            }
        }
    }
}

static OUTBOUND_QUEUE: OnceLock<OutboundQueue> = OnceLock::new();

pub(crate) fn install_outbound_queue() -> Option<mpsc::Receiver<OutboundEnvelope>> {
    let (tx, rx) = mpsc::channel(OUTBOUND_QUEUE_CAPACITY);
    OUTBOUND_QUEUE.set(OutboundQueue { tx }).ok()?;
    Some(rx)
}

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
    pub pairing_decision_tx: Option<mpsc::Sender<bool>>,
}

impl IpcDispatchContext {
    pub(crate) fn outbound_queue(&self) -> Option<&'static OutboundQueue> {
        OUTBOUND_QUEUE.get()
    }
}
