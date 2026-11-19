use std::collections::BTreeMap;
use std::sync::Arc;

use bluer::gatt::local::{
    Application, Characteristic, CharacteristicNotifier, CharacteristicNotify,
    CharacteristicNotifyMethod, CharacteristicRead, CharacteristicWrite, CharacteristicWriteMethod,
    ReqError, Service,
};
use keynear_core::{
    commit_paired_device, LanEndpointCodec, NoisePairingSession, PairedDeviceStore,
    PairingCompleted, PairingPeerMetadata, PairingState, PeerSecrets, SecretStore,
    KEYNEAR_BLE_SERVICE_UUID, LAN_ENDPOINT_CHAR_UUID, MAX_CONTROL_FRAME_BYTES,
    MAX_GATT_PAIRING_FRAME_BYTES,
};
use keynear_protocol::{v1, KeynearError, PairedDevice};
use tokio::sync::{mpsc, watch, Mutex};
use uuid::Uuid;
use zeroize::Zeroizing;

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

pub(super) struct GattApplicationContext {
    pub responder: Arc<Mutex<GattPairingResponder>>,
    pub notifier_slot: Arc<Mutex<Option<CharacteristicNotifier>>>,
    pub pairing_event_tx: Option<mpsc::Sender<PairingUiEvent>>,
    pub secret_store: Arc<dyn SecretStore>,
    pub device_store: PairedDeviceStore,
    pub local_lan_endpoint_rx: Option<watch::Receiver<Option<v1::LanEndpoint>>>,
    pub lan_endpoint_tx: Option<mpsc::Sender<v1::LanEndpoint>>,
    pub ble_addr_map: Arc<Mutex<BTreeMap<String, String>>>,
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

pub(super) fn build_gatt_application(ctx: GattApplicationContext) -> Application {
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
