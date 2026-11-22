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
