use std::sync::Arc;

use keynear_core::{PairedDeviceStore, RangingSample, RangingTechnology, SecretStore};
use keynear_protocol::{v1, KeynearError};
use tokio::sync::mpsc;
use tracing::warn;

use crate::ble::adapter::DiscoveredBlePeer;
use crate::ble::advertiser::DecodedBleBeacon;

use super::{apply_discovered_lan_endpoint, OrionQuicNode};

fn log_ble_lan_ignored(peer_device_id: &str, err: &KeynearError) {
    warn!(
        event = "ble_lan_endpoint_ignored",
        peer_device_id = %peer_device_id,
        error = %err
    );
}

pub(crate) async fn forward_ble_lan_endpoints(
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

pub(crate) async fn forward_ranging_samples(
    mut discovered_rx: mpsc::Receiver<DiscoveredBlePeer>,
    ranging_tx: mpsc::Sender<RangingSample>,
) {
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
}
