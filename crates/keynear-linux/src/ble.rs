use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use bluer::adv::Advertisement;
use bluer::gatt::local::{
    Application, Characteristic, CharacteristicNotifier, CharacteristicNotify,
    CharacteristicNotifyMethod, CharacteristicRead, CharacteristicWrite, CharacteristicWriteMethod,
    ReqError, Service,
};
use bluer::{Adapter, AdapterEvent, Address, DiscoveryFilter, DiscoveryTransport, Session};
use futures_util::StreamExt;
use keynear_core::{
    commit_paired_device, v1, DeviceRole, EveryoneNearbyBeacon, KeynearError, LanEndpointCodec,
    NoisePairingSession, PairedBeacon, PairedBeaconState, PairedDevice, PairedDeviceStore,
    PairingCompleted, PairingPeerMetadata, PairingState, PeerSecrets, SecretStore, VisibilityMode,
    KEYNEAR_BLE_SERVICE_UUID, LAN_ENDPOINT_CHAR_UUID, MAX_BLE_SERVICE_DATA_BYTES,
    MAX_CONTROL_FRAME_BYTES, MAX_GATT_PAIRING_FRAME_BYTES,
};
use tokio::sync::{mpsc, watch, Mutex};
use uuid::Uuid;
use zeroize::Zeroizing;

pub const PAIRING_TX_CHAR_UUID: Uuid = Uuid::from_u128(0x0000_FE6E_0000_1000_8000_0080_5F9B_34FB);
pub const PAIRING_RX_CHAR_UUID: Uuid = Uuid::from_u128(0x0000_FE6F_0000_1000_8000_0080_5F9B_34FB);
pub const DEFAULT_CAPABILITY_FLAGS: u32 = 0x07;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodedBleBeacon {
    EveryoneNearby(EveryoneNearbyBeacon),
    Paired {
        device_id: String,
        state: PairedBeaconState,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredBlePeer {
    pub address: String,
    pub rssi: Option<i16>,
    pub beacon: DecodedBleBeacon,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BleAdvertisementPayload {
    pub service_uuid: Uuid,
    pub service_data: Vec<u8>,
    pub visibility: VisibilityMode,
}

pub fn build_advertisement_payload(
    visibility: VisibilityMode,
    local_device_id: &str,
    display_name: &str,
    capability_flags: u32,
    counter: u32,
    paired_reconnect_key: Option<&[u8; 32]>,
) -> Result<Option<BleAdvertisementPayload>, KeynearError> {
    match visibility {
        VisibilityMode::Hidden => Ok(None),
        VisibilityMode::EveryoneNearby => {
            let beacon = EveryoneNearbyBeacon {
                role: DeviceRole::Desktop,
                capability_flags,
                display_name: display_name.to_string(),
            };
            let service_data = beacon.encode()?;
            if service_data.len() > MAX_BLE_SERVICE_DATA_BYTES {
                return Err(KeynearError::Validation {
                    field: "serviceData",
                    message: "everyone-nearby beacon exceeds 24-byte budget".to_string(),
                });
            }
            Ok(Some(BleAdvertisementPayload {
                service_uuid: KEYNEAR_BLE_SERVICE_UUID,
                service_data,
                visibility,
            }))
        }
        VisibilityMode::PairedOnly => {
            let Some(key) = paired_reconnect_key else {
                return Ok(None);
            };
            let encoded = PairedBeacon::encode(local_device_id, counter, key, true, None)?;
            Ok(Some(BleAdvertisementPayload {
                service_uuid: KEYNEAR_BLE_SERVICE_UUID,
                service_data: encoded.to_vec(),
                visibility,
            }))
        }
    }
}

pub fn decode_service_data_for_known_peers(
    service_data: &[u8],
    paired_devices: &[PairedDevice],
    secret_store: &dyn SecretStore,
    last_counters: &mut BTreeMap<String, u32>,
) -> Result<Option<DecodedBleBeacon>, KeynearError> {
    if service_data.is_empty() || service_data.len() > MAX_BLE_SERVICE_DATA_BYTES {
        return Ok(None);
    }

    if let Ok(beacon) = EveryoneNearbyBeacon::decode(service_data) {
        return Ok(Some(DecodedBleBeacon::EveryoneNearby(beacon)));
    }

    for peer in paired_devices {
        let Some(secrets) = secret_store.get_peer_secrets(&peer.device_id)? else {
            continue;
        };
        let last_seen = last_counters.get(&peer.device_id).copied();
        let expected_counter = service_data
            .get(6..10)
            .and_then(|slice| <[u8; 4]>::try_from(slice).ok())
            .map(u32::from_be_bytes)
            .unwrap_or(0);
        if let Ok(state) = PairedBeacon::decode_and_verify(
            service_data,
            &peer.device_id,
            expected_counter,
            last_seen,
            &secrets.reconnect_secret,
        ) {
            last_counters.insert(peer.device_id.clone(), state.counter);
            return Ok(Some(DecodedBleBeacon::Paired {
                device_id: peer.device_id.clone(),
                state,
            }));
        }
    }

    Ok(None)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PairingUiEvent {
    VerificationReady(String),
    Completed { peer_device_id: String },
}

pub struct BleDriverParams {
    pub local_device_id: String,
    pub display_name: String,
    pub capability_flags: u32,
    pub visibility_rx: watch::Receiver<VisibilityMode>,
    pub secret_store: Arc<dyn SecretStore>,
    pub device_store: PairedDeviceStore,
    pub discovered_tx: Option<mpsc::Sender<DiscoveredBlePeer>>,
    pub pairing_decision_rx: Option<mpsc::Receiver<bool>>,
    pub pairing_event_tx: Option<mpsc::Sender<PairingUiEvent>>,
    pub local_lan_endpoint_rx: Option<watch::Receiver<Option<v1::LanEndpoint>>>,
    pub lan_endpoint_tx: Option<mpsc::Sender<v1::LanEndpoint>>,
}

#[derive(Debug, Clone)]
pub enum GattInboundResult {
    Outbound(Vec<u8>),
    AwaitingApproval { verification_code: String },
    Completed(PairingCompleted),
}

pub struct GattPairingResponder {
    local_metadata: PairingPeerMetadata,
    secret_store: Arc<dyn SecretStore>,
    device_store: PairedDeviceStore,
    session: Option<NoisePairingSession>,
    local_export: Option<Vec<u8>>,
    peer_ble_address: Option<String>,
}

impl GattPairingResponder {
    pub fn new(
        local_metadata: PairingPeerMetadata,
        secret_store: Arc<dyn SecretStore>,
        device_store: PairedDeviceStore,
    ) -> Self {
        Self {
            local_metadata,
            secret_store,
            device_store,
            session: None,
            local_export: None,
            peer_ble_address: None,
        }
    }

    pub fn verification_code(&self) -> Option<String> {
        self.session
            .as_ref()
            .and_then(|s| s.formatted_verification_code().ok())
    }

    pub fn begin_session(&mut self, peer_address: Option<String>) -> Result<(), KeynearError> {
        let kp = self.secret_store.get_or_create_static_keypair()?;
        let priv_bytes = Zeroizing::new(kp.private_key);
        let session = NoisePairingSession::new_responder(&priv_bytes, self.local_metadata.clone())?;
        self.session = Some(session);
        self.local_export = None;
        self.peer_ble_address = peer_address;
        Ok(())
    }

    pub fn handle_tx_write(
        &mut self,
        frame: &[u8],
        peer_address: Option<String>,
    ) -> Result<GattInboundResult, KeynearError> {
        if frame.is_empty() || frame.len() > MAX_GATT_PAIRING_FRAME_BYTES {
            return Err(KeynearError::Validation {
                field: "gattFrame",
                message: format!(
                    "pairing frame length {} must be within 1..={}",
                    frame.len(),
                    MAX_GATT_PAIRING_FRAME_BYTES
                ),
            });
        }

        if self.session.is_none() {
            self.begin_session(peer_address.clone())?;
        }

        let session = self
            .session
            .as_mut()
            .ok_or_else(|| KeynearError::Protocol("missing pairing session".to_string()))?;

        match session.state() {
            PairingState::HandshakeM1 | PairingState::HandshakeM3 => {
                let outbound = session.step(frame)?;
                if let Some(m2) = outbound {
                    return Ok(GattInboundResult::Outbound(m2));
                }
                let code = session.formatted_verification_code()?;
                Ok(GattInboundResult::AwaitingApproval {
                    verification_code: self.verification_code().unwrap_or(code),
                })
            }
            PairingState::Approved => {
                let completed = session.complete_with_peer_export(frame)?;
                let device = PairedDevice {
                    device_id: completed.peer_metadata.device_id.clone(),
                    display_name: completed.peer_metadata.display_name.clone(),
                    device_role: completed.peer_metadata.device_role,
                    static_x25519_pubkey: hex::encode(completed.peer_static_pubkey),
                    last_ble_address: self
                        .peer_ble_address
                        .clone()
                        .or(peer_address)
                        .unwrap_or_default(),
                    last_lan_address: String::new(),
                    last_endpoint_id: String::new(),
                    capability_flags: completed.peer_metadata.capability_flags,
                    auto_accept_files: false,
                    paired_at: format!(
                        "{}",
                        std::time::UNIX_EPOCH
                            .elapsed()
                            .map(|d| d.as_secs())
                            .unwrap_or(0)
                    ),
                };
                let secrets = PeerSecrets {
                    irk: completed.reconnect_secret,
                    reconnect_secret: completed.reconnect_secret,
                };
                commit_paired_device(
                    &self.device_store,
                    self.secret_store.as_ref(),
                    device,
                    &secrets,
                )?;
                self.session = None;
                self.local_export = None;
                Ok(GattInboundResult::Completed(completed))
            }
            PairingState::Expired => {
                self.session = None;
                self.local_export = None;
                Err(KeynearError::PairingExpired)
            }
            PairingState::Rejected => {
                self.session = None;
                self.local_export = None;
                Err(KeynearError::PairingRejected)
            }
            _ => Err(KeynearError::Validation {
                field: "pairingState",
                message: "unexpected GATT pairing frame for current state".to_string(),
            }),
        }
    }

    pub fn approve(&mut self) -> Result<Vec<u8>, KeynearError> {
        let session = self
            .session
            .as_mut()
            .ok_or_else(|| KeynearError::Validation {
                field: "pairingState",
                message: "no pending pairing session to approve".to_string(),
            })?;
        let export = session.approve_and_build_export()?;
        self.local_export = Some(export.clone());
        Ok(export)
    }

    pub fn reject(&mut self) -> Result<(), KeynearError> {
        if let Some(mut session) = self.session.take() {
            let res = session.reject();
            self.local_export = None;
            return res;
        }
        Ok(())
    }
}

pub struct BlePeerDriver;

impl BlePeerDriver {
    pub async fn run_bluez(mut params: BleDriverParams) -> Result<(), KeynearError> {
        let session = Session::new()
            .await
            .map_err(|err| KeynearError::Bluetooth(format!("dbus bluez session failed: {err}")))?;
        let adapter = session.default_adapter().await.map_err(|err| {
            KeynearError::Bluetooth(format!("no default bluetooth adapter: {err}"))
        })?;

        let powered = adapter
            .is_powered()
            .await
            .map_err(|err| KeynearError::Bluetooth(format!("adapter power query failed: {err}")))?;
        if !powered {
            return Err(KeynearError::Bluetooth(
                "default bluetooth adapter is powered off".to_string(),
            ));
        }

        let responder = Arc::new(Mutex::new(GattPairingResponder::new(
            PairingPeerMetadata {
                device_id: params.local_device_id.clone(),
                display_name: params.display_name.clone(),
                device_role: DeviceRole::Desktop,
                capability_flags: params.capability_flags,
            },
            Arc::clone(&params.secret_store),
            params.device_store.clone(),
        )));
        let notifier_slot: Arc<Mutex<Option<CharacteristicNotifier>>> = Arc::new(Mutex::new(None));
        let ble_addr_map: Arc<Mutex<BTreeMap<String, String>>> =
            Arc::new(Mutex::new(BTreeMap::new()));

        let app = build_gatt_application(GattApplicationContext {
            responder: Arc::clone(&responder),
            notifier_slot: Arc::clone(&notifier_slot),
            pairing_event_tx: params.pairing_event_tx.take(),
            secret_store: Arc::clone(&params.secret_store),
            device_store: params.device_store.clone(),
            local_lan_endpoint_rx: params.local_lan_endpoint_rx.clone(),
            lan_endpoint_tx: params.lan_endpoint_tx.clone(),
            ble_addr_map: Arc::clone(&ble_addr_map),
        });
        let _gatt_handle = adapter
            .serve_gatt_application(app)
            .await
            .map_err(|err| KeynearError::Bluetooth(format!("gatt registration failed: {err}")))?;

        let filter = DiscoveryFilter {
            transport: DiscoveryTransport::Le,
            duplicate_data: true,
            ..Default::default()
        };
        adapter
            .set_discovery_filter(filter)
            .await
            .map_err(|err| KeynearError::Bluetooth(format!("discovery filter failed: {err}")))?;

        let mut events = adapter
            .discover_devices_with_changes()
            .await
            .map_err(|err| KeynearError::Bluetooth(format!("le discovery start failed: {err}")))?;

        let initial_visibility = *params.visibility_rx.borrow();
        let mut adv_handle = register_visibility_advertisement(
            &adapter,
            initial_visibility,
            &params.local_device_id,
            &params.display_name,
            params.capability_flags,
            params.secret_store.as_ref(),
            &params.device_store,
        )
        .await?;

        let mut last_counters = BTreeMap::new();

        loop {
            tokio::select! {
                changed = params.visibility_rx.changed() => {
                    if changed.is_err() {
                        break;
                    }
                    drop(adv_handle.take());
                    let mode = *params.visibility_rx.borrow_and_update();
                    adv_handle = register_visibility_advertisement(
                        &adapter,
                        mode,
                        &params.local_device_id,
                        &params.display_name,
                        params.capability_flags,
                        params.secret_store.as_ref(),
                        &params.device_store,
                    ).await?;
                }
                Some(approved) = recv_optional(&mut params.pairing_decision_rx) => {
                    if approved {
                        if let Ok(frame) = responder.lock().await.approve() {
                            if let Some(notifier) = notifier_slot.lock().await.as_mut() {
                                let _ = notifier.notify(frame).await;
                            }
                        }
                    } else {
                        let _ = responder.lock().await.reject();
                    }
                }
                maybe_ev = events.next() => {
                    let Some(ev) = maybe_ev else {
                        break;
                    };
                    if let AdapterEvent::DeviceAdded(addr) = ev {
                        if let Some(peer) = inspect_discovered_device(
                            &adapter,
                            addr,
                            &params.device_store,
                            params.secret_store.as_ref(),
                            &mut last_counters,
                        ).await {
                            if let DecodedBleBeacon::Paired { ref device_id, .. } = peer.beacon {
                                ble_addr_map
                                    .lock()
                                    .await
                                    .insert(peer.address.to_ascii_lowercase(), device_id.clone());
                            }
                            if let Some(ref tx) = params.discovered_tx {
                                let _ = tx.try_send(peer);
                            }
                        }
                    }
                }
            }
        }

        drop(adv_handle);
        Ok(())
    }
}

async fn register_visibility_advertisement(
    adapter: &Adapter,
    visibility: VisibilityMode,
    local_device_id: &str,
    display_name: &str,
    capability_flags: u32,
    secret_store: &dyn SecretStore,
    device_store: &PairedDeviceStore,
) -> Result<Option<bluer::adv::AdvertisementHandle>, KeynearError> {
    let first_peer_key = if visibility == VisibilityMode::PairedOnly {
        let peers = device_store.load().unwrap_or_default();
        peers
            .first()
            .and_then(|p| secret_store.get_peer_secrets(&p.device_id).ok().flatten())
            .map(|s| s.reconnect_secret)
    } else {
        None
    };

    let counter = if visibility == VisibilityMode::PairedOnly && first_peer_key.is_some() {
        secret_store.next_presence_counter().unwrap_or(1)
    } else {
        0
    };

    let Some(payload) = build_advertisement_payload(
        visibility,
        local_device_id,
        display_name,
        capability_flags,
        counter,
        first_peer_key.as_ref(),
    )?
    else {
        return Ok(None);
    };

    let mut service_uuids = BTreeSet::new();
    service_uuids.insert(payload.service_uuid);
    let mut service_data = BTreeMap::new();
    service_data.insert(payload.service_uuid, payload.service_data);

    let adv = Advertisement {
        advertisement_type: bluer::adv::Type::Peripheral,
        service_uuids,
        service_data,
        discoverable: Some(false),
        ..Default::default()
    };

    let handle = adapter
        .advertise(adv)
        .await
        .map_err(|err| KeynearError::Bluetooth(format!("le advertise failed: {err}")))?;
    Ok(Some(handle))
}

async fn inspect_discovered_device(
    adapter: &Adapter,
    addr: Address,
    device_store: &PairedDeviceStore,
    secret_store: &dyn SecretStore,
    last_counters: &mut BTreeMap<String, u32>,
) -> Option<DiscoveredBlePeer> {
    let device = adapter.device(addr).ok()?;
    let service_data_map = device.service_data().await.ok().flatten()?;
    let bytes = service_data_map.get(&KEYNEAR_BLE_SERVICE_UUID)?;
    let paired_devices = device_store.load().unwrap_or_default();
    let decoded =
        decode_service_data_for_known_peers(bytes, &paired_devices, secret_store, last_counters)
            .ok()??;
    let rssi = device.rssi().await.ok().flatten();
    Some(DiscoveredBlePeer {
        address: addr.to_string(),
        rssi,
        beacon: decoded,
    })
}

async fn recv_optional<T>(rx: &mut Option<mpsc::Receiver<T>>) -> Option<T> {
    match rx {
        Some(inner) => inner.recv().await,
        None => std::future::pending().await,
    }
}

struct GattApplicationContext {
    responder: Arc<Mutex<GattPairingResponder>>,
    notifier_slot: Arc<Mutex<Option<CharacteristicNotifier>>>,
    pairing_event_tx: Option<mpsc::Sender<PairingUiEvent>>,
    secret_store: Arc<dyn SecretStore>,
    device_store: PairedDeviceStore,
    local_lan_endpoint_rx: Option<watch::Receiver<Option<v1::LanEndpoint>>>,
    lan_endpoint_tx: Option<mpsc::Sender<v1::LanEndpoint>>,
    ble_addr_map: Arc<Mutex<BTreeMap<String, String>>>,
}

pub fn encode_lan_endpoint_for_peer(
    endpoint: &v1::LanEndpoint,
    known_device_id: Option<&str>,
    peer_ble_address: Option<&str>,
    device_store: &PairedDeviceStore,
    secret_store: &dyn SecretStore,
) -> Result<Vec<u8>, KeynearError> {
    let devices = device_store.load()?;
    if devices.is_empty() {
        return Err(KeynearError::UnauthorizedPeer(
            "no paired devices registered for LAN endpoint exchange".to_string(),
        ));
    }

    let matched = known_device_id
        .and_then(|id| devices.iter().find(|d| d.device_id == id))
        .or_else(|| {
            peer_ble_address
                .map(str::trim)
                .filter(|addr| !addr.is_empty())
                .and_then(|addr| {
                    devices.iter().find(|d| {
                        !d.last_ble_address.trim().is_empty()
                            && d.last_ble_address.eq_ignore_ascii_case(addr)
                    })
                })
        })
        .or_else(|| {
            if devices.len() == 1 {
                devices.first()
            } else {
                None
            }
        })
        .ok_or_else(|| {
            KeynearError::UnauthorizedPeer(
                "unrecognized BLE peer address for LAN endpoint read".to_string(),
            )
        })?;

    let secrets = secret_store
        .get_peer_secrets(&matched.device_id)?
        .ok_or_else(|| {
            KeynearError::UnauthorizedPeer(format!(
                "missing pairwise reconnect secret for '{}'",
                matched.device_id
            ))
        })?;

    LanEndpointCodec::encode(endpoint, &secrets.reconnect_secret)
}

pub fn decode_lan_endpoint_from_known_peers(
    encoded: &[u8],
    device_store: &PairedDeviceStore,
    secret_store: &dyn SecretStore,
) -> Result<v1::LanEndpoint, KeynearError> {
    if encoded.is_empty() || encoded.len() > MAX_CONTROL_FRAME_BYTES {
        return Err(KeynearError::Validation {
            field: "lanEndpoint",
            message: format!(
                "encrypted LAN endpoint length {} must be within 1..={MAX_CONTROL_FRAME_BYTES}",
                encoded.len()
            ),
        });
    }
    let devices = device_store.load()?;
    let mut keys: Vec<(String, [u8; 32])> = Vec::new();
    for dev in devices {
        if let Some(secrets) = secret_store.get_peer_secrets(&dev.device_id)? {
            keys.push((dev.device_id, secrets.reconnect_secret));
        }
    }
    LanEndpointCodec::decode_any(
        encoded,
        keys.iter().map(|(id, secret)| (id.as_str(), secret)),
    )
}

fn build_gatt_application(ctx: GattApplicationContext) -> Application {
    let tx_responder = Arc::clone(&ctx.responder);
    let tx_notifier = Arc::clone(&ctx.notifier_slot);
    let rx_notifier = Arc::clone(&ctx.notifier_slot);
    let pairing_event_tx = ctx.pairing_event_tx;

    let read_secrets = Arc::clone(&ctx.secret_store);
    let read_devices = ctx.device_store.clone();
    let read_lan_rx = ctx.local_lan_endpoint_rx.clone();
    let read_addr_map = Arc::clone(&ctx.ble_addr_map);

    let write_secrets = Arc::clone(&ctx.secret_store);
    let write_devices = ctx.device_store;
    let write_lan_tx = ctx.lan_endpoint_tx;
    let write_addr_map = Arc::clone(&ctx.ble_addr_map);

    Application {
        services: vec![Service {
            uuid: KEYNEAR_BLE_SERVICE_UUID,
            primary: true,
            characteristics: vec![
                Characteristic {
                    uuid: PAIRING_TX_CHAR_UUID,
                    write: Some(CharacteristicWrite {
                        write: true,
                        write_without_response: false,
                        method: CharacteristicWriteMethod::Fun(Box::new(move |value, req| {
                            let responder = Arc::clone(&tx_responder);
                            let notifier_slot = Arc::clone(&tx_notifier);
                            let event_tx = pairing_event_tx.clone();
                            let peer_addr = req.device_address.to_string();
                            Box::pin(async move {
                                if value.is_empty() || value.len() > MAX_GATT_PAIRING_FRAME_BYTES {
                                    return Err(ReqError::InvalidValueLength);
                                }
                                let result = {
                                    let mut guard = responder.lock().await;
                                    guard.handle_tx_write(&value, Some(peer_addr))
                                };
                                match result {
                                    Ok(GattInboundResult::Outbound(frame)) => {
                                        if let Some(notifier) = notifier_slot.lock().await.as_mut()
                                        {
                                            let _ = notifier.notify(frame).await;
                                        }
                                        Ok(())
                                    }
                                    Ok(GattInboundResult::AwaitingApproval {
                                        verification_code,
                                    }) => {
                                        if let Some(tx) = event_tx {
                                            let _ = tx.try_send(PairingUiEvent::VerificationReady(
                                                verification_code,
                                            ));
                                        }
                                        Ok(())
                                    }
                                    Ok(GattInboundResult::Completed(completed)) => {
                                        if let Some(tx) = event_tx {
                                            let _ = tx.try_send(PairingUiEvent::Completed {
                                                peer_device_id: completed
                                                    .peer_metadata
                                                    .device_id
                                                    .clone(),
                                            });
                                        }
                                        Ok(())
                                    }
                                    Err(KeynearError::Validation { .. }) => {
                                        Err(ReqError::InvalidValueLength)
                                    }
                                    Err(_) => Err(ReqError::Failed),
                                }
                            })
                        })),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                Characteristic {
                    uuid: PAIRING_RX_CHAR_UUID,
                    notify: Some(CharacteristicNotify {
                        notify: true,
                        method: CharacteristicNotifyMethod::Fun(Box::new(move |notifier| {
                            let slot = Arc::clone(&rx_notifier);
                            Box::pin(async move {
                                *slot.lock().await = Some(notifier);
                            })
                        })),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                Characteristic {
                    uuid: LAN_ENDPOINT_CHAR_UUID,
                    read: Some(CharacteristicRead {
                        read: true,
                        fun: Box::new(move |req| {
                            let secrets = Arc::clone(&read_secrets);
                            let devices = read_devices.clone();
                            let lan_rx = read_lan_rx.clone();
                            let addr_map = Arc::clone(&read_addr_map);
                            let peer_addr = req.device_address.to_string();
                            Box::pin(async move {
                                let Some(endpoint) =
                                    lan_rx.as_ref().and_then(|rx| rx.borrow().clone())
                                else {
                                    return Err(ReqError::Failed);
                                };
                                let known_id = addr_map
                                    .lock()
                                    .await
                                    .get(&peer_addr.to_ascii_lowercase())
                                    .cloned();
                                match encode_lan_endpoint_for_peer(
                                    &endpoint,
                                    known_id.as_deref(),
                                    Some(&peer_addr),
                                    &devices,
                                    secrets.as_ref(),
                                ) {
                                    Ok(bytes) => Ok(bytes),
                                    Err(KeynearError::UnauthorizedPeer(_)) => {
                                        Err(ReqError::NotPermitted)
                                    }
                                    Err(_) => Err(ReqError::Failed),
                                }
                            })
                        }),
                        ..Default::default()
                    }),
                    write: Some(CharacteristicWrite {
                        write: true,
                        write_without_response: false,
                        method: CharacteristicWriteMethod::Fun(Box::new(move |value, req| {
                            let secrets = Arc::clone(&write_secrets);
                            let devices = write_devices.clone();
                            let lan_tx = write_lan_tx.clone();
                            let addr_map = Arc::clone(&write_addr_map);
                            let peer_addr = req.device_address.to_string();
                            Box::pin(async move {
                                if value.is_empty() || value.len() > MAX_CONTROL_FRAME_BYTES {
                                    return Err(ReqError::InvalidValueLength);
                                }
                                match decode_lan_endpoint_from_known_peers(
                                    &value,
                                    &devices,
                                    secrets.as_ref(),
                                ) {
                                    Ok(endpoint) => {
                                        addr_map.lock().await.insert(
                                            peer_addr.to_ascii_lowercase(),
                                            endpoint.device_id.clone(),
                                        );
                                        if let Some(tx) = lan_tx {
                                            let _ = tx.try_send(endpoint);
                                        }
                                        Ok(())
                                    }
                                    Err(KeynearError::Validation { .. }) => {
                                        Err(ReqError::InvalidValueLength)
                                    }
                                    Err(KeynearError::UnauthorizedPeer(_)) => {
                                        Err(ReqError::NotPermitted)
                                    }
                                    Err(_) => Err(ReqError::Failed),
                                }
                            })
                        })),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
            ],
            ..Default::default()
        }],
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use keynear_core::{InMemorySecretStore, PairingRole};

    use super::*;

    fn temp_devices_store(label: &str) -> (std::path::PathBuf, PairedDeviceStore) {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("keynear-linux-{label}-{nonce}"));
        fs::create_dir_all(&dir).expect("tempdir");
        let store = PairedDeviceStore::new(dir.join("devices.toml"));
        (dir, store)
    }

    #[test]
    fn visibility_advertisement_payload_enforces_modes_and_budget() {
        assert!(build_advertisement_payload(
            VisibilityMode::Hidden,
            "linux-1",
            "Workstation",
            DEFAULT_CAPABILITY_FLAGS,
            1,
            None,
        )
        .expect("hidden ok")
        .is_none());

        assert!(build_advertisement_payload(
            VisibilityMode::PairedOnly,
            "linux-1",
            "Workstation",
            DEFAULT_CAPABILITY_FLAGS,
            1,
            None,
        )
        .expect("unpaired paired-only omits beacon")
        .is_none());

        let everyone = build_advertisement_payload(
            VisibilityMode::EveryoneNearby,
            "linux-1",
            "Workstation",
            DEFAULT_CAPABILITY_FLAGS,
            1,
            None,
        )
        .expect("everyone ok")
        .expect("everyone payload");
        assert!(everyone.service_data.len() <= MAX_BLE_SERVICE_DATA_BYTES);

        let key = [0x42u8; 32];
        let paired = build_advertisement_payload(
            VisibilityMode::PairedOnly,
            "linux-1",
            "Workstation",
            DEFAULT_CAPABILITY_FLAGS,
            7,
            Some(&key),
        )
        .expect("paired ok")
        .expect("paired payload");
        assert_eq!(paired.service_data.len(), MAX_BLE_SERVICE_DATA_BYTES);
    }

    #[test]
    fn gatt_loopback_pairing_completes_and_persists_peer_and_secrets() {
        let (dir, device_store) = temp_devices_store("gatt-loopback");
        let resp_secrets = Arc::new(InMemorySecretStore::new());
        let init_secrets = InMemorySecretStore::new();
        let init_kp = init_secrets
            .get_or_create_static_keypair()
            .expect("init kp");

        let mut responder = GattPairingResponder::new(
            PairingPeerMetadata {
                device_id: "linux-workstation".to_string(),
                display_name: "Workstation".to_string(),
                device_role: DeviceRole::Desktop,
                capability_flags: DEFAULT_CAPABILITY_FLAGS,
            },
            Arc::clone(&resp_secrets) as Arc<dyn SecretStore>,
            device_store.clone(),
        );

        let mut initiator = NoisePairingSession::new_initiator(
            &init_kp.private_key,
            PairingPeerMetadata {
                device_id: "pixel-9".to_string(),
                display_name: "Pixel 9".to_string(),
                device_role: DeviceRole::Mobile,
                capability_flags: 0x03,
            },
        )
        .expect("initiator session");

        let m1 = initiator.start_handshake().expect("m1");
        let GattInboundResult::Outbound(m2) = responder
            .handle_tx_write(&m1, Some("11:22:33:44:55:66".to_string()))
            .expect("m2")
        else {
            panic!("expected M2 outbound frame");
        };
        let m3 = initiator.step(&m2).expect("m3").expect("m3 frame");
        let GattInboundResult::AwaitingApproval { verification_code } = responder
            .handle_tx_write(&m3, Some("11:22:33:44:55:66".to_string()))
            .expect("awaiting approval")
        else {
            panic!("expected AwaitingApproval");
        };

        assert_eq!(
            verification_code,
            initiator.formatted_verification_code().expect("init code")
        );
        assert_eq!(initiator.role(), PairingRole::Initiator);

        let resp_export = responder.approve().expect("resp approve");
        let init_export = initiator.approve_and_build_export().expect("init approve");

        let init_done = initiator
            .complete_with_peer_export(&resp_export)
            .expect("init complete");
        let GattInboundResult::Completed(resp_done) = responder
            .handle_tx_write(&init_export, Some("11:22:33:44:55:66".to_string()))
            .expect("resp complete")
        else {
            panic!("expected Completed");
        };

        assert_eq!(init_done.reconnect_secret, resp_done.reconnect_secret);
        let devices = device_store.load().expect("load devices");
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].device_id, "pixel-9");
        assert_eq!(devices[0].last_ble_address, "11:22:33:44:55:66");

        let stored_secrets = resp_secrets
            .get_peer_secrets("pixel-9")
            .expect("get peer secrets")
            .expect("peer secrets stored");
        assert_eq!(stored_secrets.reconnect_secret, init_done.reconnect_secret);

        let oversized = vec![0u8; MAX_GATT_PAIRING_FRAME_BYTES + 1];
        assert!(matches!(
            responder.handle_tx_write(&oversized, None),
            Err(KeynearError::Validation {
                field: "gattFrame",
                ..
            })
        ));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn decode_service_data_handles_paired_and_everyone_nearby() {
        let secrets = InMemorySecretStore::new();
        let key = [0x77u8; 32];
        secrets
            .put_peer_secrets(
                "pixel-9",
                &PeerSecrets {
                    irk: key,
                    reconnect_secret: key,
                },
            )
            .expect("put secrets");
        let devices = vec![PairedDevice {
            device_id: "pixel-9".to_string(),
            display_name: "Pixel 9".to_string(),
            device_role: DeviceRole::Mobile,
            static_x25519_pubkey: "cd".repeat(32),
            last_ble_address: String::new(),
            last_lan_address: String::new(),
            last_endpoint_id: String::new(),
            capability_flags: 0x03,
            auto_accept_files: false,
            paired_at: "1".to_string(),
        }];

        let mut counters = BTreeMap::new();
        let paired_raw = PairedBeacon::encode("pixel-9", 3, &key, true, Some(91)).expect("encode");
        let decoded =
            decode_service_data_for_known_peers(&paired_raw, &devices, &secrets, &mut counters)
                .expect("decode ok")
                .expect("matched peer");
        assert!(matches!(
            decoded,
            DecodedBleBeacon::Paired { ref device_id, state: PairedBeaconState { counter: 3, available: true, battery_level: Some(91) } } if device_id == "pixel-9"
        ));

        let replayed =
            decode_service_data_for_known_peers(&paired_raw, &devices, &secrets, &mut counters)
                .expect("decode ok");
        assert!(replayed.is_none());
    }

    #[test]
    fn lan_endpoint_gatt_characteristic_roundtrips_and_rejects_unpaired() {
        let (dir, device_store) = temp_devices_store("lan-char");
        let secrets = InMemorySecretStore::new();
        let pixel_secret = [0x55u8; 32];
        let tablet_secret = [0x66u8; 32];

        commit_paired_device(
            &device_store,
            &secrets,
            PairedDevice {
                device_id: "pixel-9".to_string(),
                display_name: "Pixel 9".to_string(),
                device_role: DeviceRole::Mobile,
                static_x25519_pubkey: "aa".repeat(32),
                last_ble_address: "11:22:33:44:55:66".to_string(),
                last_lan_address: String::new(),
                last_endpoint_id: String::new(),
                capability_flags: 0x07,
                auto_accept_files: false,
                paired_at: "1".to_string(),
            },
            &PeerSecrets {
                irk: pixel_secret,
                reconnect_secret: pixel_secret,
            },
        )
        .expect("commit pixel");

        commit_paired_device(
            &device_store,
            &secrets,
            PairedDevice {
                device_id: "galaxy-tab".to_string(),
                display_name: "Galaxy Tab".to_string(),
                device_role: DeviceRole::Mobile,
                static_x25519_pubkey: "bb".repeat(32),
                last_ble_address: "AA:BB:CC:DD:EE:FF".to_string(),
                last_lan_address: String::new(),
                last_endpoint_id: String::new(),
                capability_flags: 0x07,
                auto_accept_files: false,
                paired_at: "2".to_string(),
            },
            &PeerSecrets {
                irk: tablet_secret,
                reconnect_secret: tablet_secret,
            },
        )
        .expect("commit tablet");

        let local_ep = v1::LanEndpoint {
            device_id: "linux-workstation".to_string(),
            endpoint_id: vec![0x11; 32],
            socket_addrs: vec!["192.168.1.20:42000".to_string()],
            capability_flags: DEFAULT_CAPABILITY_FLAGS,
            issued_at_ms: 1_700_000_000_000,
        };
        let encoded = encode_lan_endpoint_for_peer(
            &local_ep,
            None,
            Some("11:22:33:44:55:66"),
            &device_store,
            &secrets,
        )
        .expect("encode for matched peer");
        let decoded_by_peer =
            LanEndpointCodec::decode(&encoded, "linux-workstation", &pixel_secret)
                .expect("peer decodes local endpoint");
        assert_eq!(decoded_by_peer, local_ep);

        let unrecognized = encode_lan_endpoint_for_peer(
            &local_ep,
            None,
            Some("FF:FF:FF:FF:FF:FF"),
            &device_store,
            &secrets,
        );
        assert!(matches!(
            unrecognized,
            Err(KeynearError::UnauthorizedPeer(_))
        ));

        let inbound_ep = v1::LanEndpoint {
            device_id: "pixel-9".to_string(),
            endpoint_id: vec![0x22; 32],
            socket_addrs: vec!["192.168.1.33:42001".to_string()],
            capability_flags: 0x03,
            issued_at_ms: 1_700_000_000_123,
        };
        let peer_wire =
            LanEndpointCodec::encode(&inbound_ep, &pixel_secret).expect("encode inbound");
        let decoded_inbound =
            decode_lan_endpoint_from_known_peers(&peer_wire, &device_store, &secrets)
                .expect("decode known peer write");
        assert_eq!(decoded_inbound, inbound_ep);

        let rogue_ep = v1::LanEndpoint {
            device_id: "rogue-device".to_string(),
            endpoint_id: vec![0x99; 32],
            socket_addrs: vec!["192.168.1.99:42002".to_string()],
            capability_flags: 0x01,
            issued_at_ms: 1_700_000_000_999,
        };
        let rogue_wire = LanEndpointCodec::encode(&rogue_ep, &[0x99; 32]).expect("encode rogue");
        assert!(matches!(
            decode_lan_endpoint_from_known_peers(&rogue_wire, &device_store, &secrets),
            Err(KeynearError::UnauthorizedPeer(_))
        ));

        let responder = Arc::new(Mutex::new(GattPairingResponder::new(
            PairingPeerMetadata {
                device_id: "linux-workstation".to_string(),
                display_name: "Workstation".to_string(),
                device_role: DeviceRole::Desktop,
                capability_flags: DEFAULT_CAPABILITY_FLAGS,
            },
            Arc::new(secrets),
            device_store,
        )));
        let app = build_gatt_application(GattApplicationContext {
            responder,
            notifier_slot: Arc::new(Mutex::new(None)),
            pairing_event_tx: None,
            secret_store: Arc::new(InMemorySecretStore::new()),
            device_store: PairedDeviceStore::new(dir.join("devices.toml")),
            local_lan_endpoint_rx: None,
            lan_endpoint_tx: None,
            ble_addr_map: Arc::new(Mutex::new(BTreeMap::new())),
        });
        assert_eq!(app.services.len(), 1);
        assert_eq!(app.services[0].uuid, KEYNEAR_BLE_SERVICE_UUID);
        let uuids: Vec<Uuid> = app.services[0]
            .characteristics
            .iter()
            .map(|c| c.uuid)
            .collect();
        assert!(uuids.contains(&PAIRING_TX_CHAR_UUID));
        assert!(uuids.contains(&PAIRING_RX_CHAR_UUID));
        assert!(uuids.contains(&LAN_ENDPOINT_CHAR_UUID));

        let _ = fs::remove_dir_all(&dir);
    }
}
