mod application;
mod endpoint;
mod responder;

use std::collections::BTreeMap;
use std::sync::Arc;

use bluer::gatt::local::CharacteristicNotifier;
use keynear_core::PairedDeviceStore;
use tokio::sync::{mpsc, watch, Mutex};
use uuid::Uuid;

pub(crate) use application::build_gatt_application;
#[cfg(test)]
pub(crate) use endpoint::{
    decode_lan_endpoint_from_known_peers, encode_lan_endpoint_for_peer, EncodeEndpointParams,
};
pub(crate) use responder::GattPairingResponder;

pub const PAIRING_TX_CHAR_UUID: Uuid = Uuid::from_u128(0x0000_FE6E_0000_1000_8000_0080_5F9B_34FB);
pub const PAIRING_RX_CHAR_UUID: Uuid = Uuid::from_u128(0x0000_FE6F_0000_1000_8000_0080_5F9B_34FB);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PairingUiEvent {
    VerificationReady(String),
    Completed { peer_device_id: String },
}

#[derive(Debug, Clone)]
pub enum GattInboundResult {
    Outbound(Vec<u8>),
    AwaitingApproval { verification_code: String },
    PeerExportBuffered,
    Completed(keynear_core::PairingCompleted),
}

pub(crate) struct GattApplicationContext {
    pub responder: Arc<Mutex<GattPairingResponder>>,
    pub notifier_slot: Arc<Mutex<Option<CharacteristicNotifier>>>,
    pub pairing_event_tx: Option<mpsc::Sender<PairingUiEvent>>,
    pub secret_store: Arc<dyn keynear_core::SecretStore>,
    pub device_store: PairedDeviceStore,
    pub local_lan_endpoint_rx: Option<watch::Receiver<Option<keynear_protocol::v1::LanEndpoint>>>,
    pub lan_endpoint_tx: Option<mpsc::Sender<keynear_protocol::v1::LanEndpoint>>,
    pub ble_addr_map: Arc<Mutex<BTreeMap<String, String>>>,
}

fn log_pairing_approve_failed(err: &keynear_protocol::KeynearError) {
    tracing::warn!(event = "pairing_approve_failed", error = %err);
}

fn log_buffered_export_failed(err: &keynear_protocol::KeynearError) {
    tracing::warn!(event = "pairing_buffered_export_failed", error = %err);
}

pub(crate) async fn apply_pairing_decision(
    responder: &Mutex<GattPairingResponder>,
    notifier_slot: &Mutex<Option<CharacteristicNotifier>>,
    pairing_event_tx: Option<&mpsc::Sender<PairingUiEvent>>,
    approved: bool,
) {
    if !approved {
        let _ = responder.lock().await.reject();
        return;
    }
    let frame = match responder.lock().await.approve() {
        Ok(frame) => frame,
        Err(err) => {
            log_pairing_approve_failed(&err);
            return;
        }
    };
    if let Some(notifier) = notifier_slot.lock().await.as_mut() {
        let _ = notifier.notify(frame).await;
    }
    match responder.lock().await.complete_pending_export() {
        Ok(Some(done)) => {
            if let Some(tx) = pairing_event_tx {
                let id = done.peer_metadata.device_id.clone();
                let _ = tx.try_send(PairingUiEvent::Completed { peer_device_id: id });
            }
        }
        Ok(None) => {}
        Err(err) => log_buffered_export_failed(&err),
    }
}
