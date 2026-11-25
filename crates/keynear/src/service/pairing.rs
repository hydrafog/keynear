use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use keynear_core::{
    parse_endpoint_id_bytes, MdnsDiscovery, OrionQuicNode, PairedDeviceStore, RangingSample,
    RangingTechnology, SecretStore, TransferEngine,
};
use keynear_protocol::{v1, KeynearError, VisibilityMode};
use tokio::sync::{mpsc, watch, Mutex};
use tokio::task::JoinHandle;
use tracing::{info, warn};

use crate::ble::advertiser::DecodedBleBeacon;
use crate::ble::gatt::PairingUiEvent;
use crate::ble::adapter::DiscoveredBlePeer;
use crate::ble::{BleDriverParams, BlePeerDriver, DEFAULT_CAPABILITY_FLAGS};
use crate::ipc::IpcDispatchContext;
use crate::notifications::NotificationsClient;

const DEVICES_POLL_INTERVAL: Duration = Duration::from_millis(200);
const LAN_ENDPOINT_CHANNEL_SLOTS: usize = 32;

#[allow(clippy::too_many_arguments)]
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

#[allow(clippy::too_many_arguments)]
pub(crate) fn spawn_mdns_browse_loop(
    mdns: Option<&Arc<MdnsDiscovery>>,
    quic_node: Option<&Arc<OrionQuicNode>>,
    secret_store: Option<&Arc<dyn SecretStore>>,
    device_store: &PairedDeviceStore,
    local_device_id: &str,
    tasks: &mut Vec<JoinHandle<()>>,
) {
    if let (Some(discovery), Some(node), Some(secrets)) = (mdns, quic_node, secret_store) {
        if let Some(mut rx) = discovery.browse(local_device_id) {
            let store = device_store.clone();
            let secrets = Arc::clone(secrets);
            let node = Arc::clone(node);
            tasks.push(tokio::spawn(async move {
                while let Some(peer) = rx.recv().await {
                    let endpoint = peer.to_lan_endpoint();
                    if let Err(err) =
                        apply_discovered_lan_endpoint(&endpoint, &store, secrets.as_ref(), &node)
                    {
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
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn spawn_devices_poll_loop(
    device_store: &PairedDeviceStore,
    secret_store: Option<&Arc<dyn SecretStore>>,
    quic_node: Option<&Arc<OrionQuicNode>>,
    transfer_engine: &Arc<TransferEngine>,
    known_peers: &Arc<Mutex<HashSet<String>>>,
    tasks: &mut Vec<JoinHandle<()>>,
) {
    let poll_store = device_store.clone();
    let poll_secrets = secret_store.cloned();
    let poll_node = quic_node.cloned();
    let poll_engine = Arc::clone(transfer_engine);
    let poll_known = Arc::clone(known_peers);
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

pub(crate) struct BluezLoopParams<'a> {
    pub(crate) enable_bluez: bool,
    pub(crate) local_device_id: &'a str,
    pub(crate) display_name: &'a str,
    pub(crate) visibility_rx: &'a watch::Receiver<VisibilityMode>,
    pub(crate) lan_watch_rx: watch::Receiver<Option<v1::LanEndpoint>>,
    pub(crate) device_store: &'a PairedDeviceStore,
    pub(crate) secret_store: Option<&'a Arc<dyn SecretStore>>,
    pub(crate) quic_node: Option<&'a Arc<OrionQuicNode>>,
    pub(crate) pairing_decision_rx: Option<mpsc::Receiver<bool>>,
    pub(crate) pairing_event_tx: Option<mpsc::Sender<PairingUiEvent>>,
    pub(crate) ranging_tx: Option<mpsc::Sender<RangingSample>>,
}

fn log_ble_lan_ignored(peer_device_id: &str, err: &KeynearError) {
    warn!(
        event = "ble_lan_endpoint_ignored",
        peer_device_id = %peer_device_id,
        error = %err
    );
}

async fn forward_ble_lan_endpoints(
    mut ble_lan_rx: mpsc::Receiver<v1::LanEndpoint>,
    store: PairedDeviceStore,
    secrets: Arc<dyn SecretStore>,
    node: Arc<OrionQuicNode>,
) {
    while let Some(endpoint) = ble_lan_rx.recv().await {
        if let Err(err) = apply_discovered_lan_endpoint(&endpoint, &store, secrets.as_ref(), &node)
        {
            log_ble_lan_ignored(&endpoint.device_id, &err);
        }
    }
}

pub(crate) fn spawn_bluez_loops(params: BluezLoopParams<'_>, tasks: &mut Vec<JoinHandle<()>>) {
    if !params.enable_bluez {
        return;
    }
    let Some(secrets) = params.secret_store else {
        return;
    };
    let (ble_lan_tx, ble_lan_rx) = mpsc::channel::<v1::LanEndpoint>(LAN_ENDPOINT_CHANNEL_SLOTS);
    let (discovered_tx, mut discovered_rx) = mpsc::channel::<DiscoveredBlePeer>(LAN_ENDPOINT_CHANNEL_SLOTS);
    if let Some(ranging_tx) = params.ranging_tx {
        tasks.push(tokio::spawn(async move {
            let mut sequence = 0u64;
            while let Some(peer) = discovered_rx.recv().await {
                let DecodedBleBeacon::Paired { device_id, .. } = &peer.beacon else {
                    continue;
                };
                let Some(rssi) = peer.rssi else {
                    continue;
                };
                sequence += 1;
                let sample = RangingSample {
                    device_id: device_id.clone(),
                    technology: RangingTechnology::BleRssi,
                    distance_meters: None,
                    rssi_dbm: Some(i32::from(rssi)),
                    sequence,
                };
                if ranging_tx.send(sample).await.is_err() {
                    break;
                }
            }
        }));
    }
    if let Some(node) = params.quic_node {
        let store = params.device_store.clone();
        let secrets = Arc::clone(secrets);
        let node = Arc::clone(node);
        tasks.push(tokio::spawn(forward_ble_lan_endpoints(
            ble_lan_rx, store, secrets, node,
        )));
    }
    let driver = BleDriverParams {
        local_device_id: params.local_device_id.to_string(),
        display_name: params.display_name.to_string(),
        capability_flags: DEFAULT_CAPABILITY_FLAGS,
        visibility_rx: params.visibility_rx.clone(),
        secret_store: Arc::clone(secrets),
        device_store: params.device_store.clone(),
        discovered_tx: Some(discovered_tx),
        pairing_decision_rx: params.pairing_decision_rx,
        pairing_event_tx: params.pairing_event_tx,
        local_lan_endpoint_rx: Some(params.lan_watch_rx),
        lan_endpoint_tx: Some(ble_lan_tx),
    };
    tasks.push(tokio::spawn(async move {
        if let Err(err) = BlePeerDriver::run_bluez(driver).await {
            warn!(event = "ble_unavailable_degraded", error = %err);
        }
    }));
}

fn log_pairing_info(event: &'static str, value: &str) {
    info!(event, value = %value);
}

fn log_pairing_sync_err(err: &KeynearError) {
    warn!(event = "pairing_post_sync_failed", error = %err);
}

async fn on_pairing_completed(
    peer_device_id: &str,
    notif_client: Option<&NotificationsClient>,
    ctx: &IpcDispatchContext,
) {
    log_pairing_info("pairing_completed", peer_device_id);
    if let Err(err) = sync_paired_devices_and_policies(
        &ctx.device_store,
        ctx.secret_store.as_deref(),
        ctx.quic_node.as_deref(),
        &ctx.transfer_engine,
        &ctx.known_peers,
    )
    .await
    {
        log_pairing_sync_err(&err);
    }
    ctx.refresh_notify.notify_waiters();
    if let Some(client) = notif_client {
        client.notify_pairing_completed(peer_device_id).await;
    }
}

pub(crate) fn spawn_pairing_event_loop(
    mut pairing_event_rx: mpsc::Receiver<PairingUiEvent>,
    notif_client: Option<NotificationsClient>,
    ctx: IpcDispatchContext,
    tasks: &mut Vec<JoinHandle<()>>,
) {
    tasks.push(tokio::spawn(async move {
        while let Some(event) = pairing_event_rx.recv().await {
            match event {
                PairingUiEvent::VerificationReady(code) => {
                    log_pairing_info("pairing_verification_ready", &code);
                    if let Some(ref client) = notif_client {
                        client.notify_pairing_verification(&code).await;
                    }
                }
                PairingUiEvent::Completed { peer_device_id } => {
                    on_pairing_completed(&peer_device_id, notif_client.as_ref(), &ctx).await;
                }
            }
        }
    }));
}
