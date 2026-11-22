use std::collections::BTreeMap;
use std::sync::Arc;

use bluer::gatt::local::{
    Application, Characteristic, CharacteristicNotify, CharacteristicNotifyMethod,
    CharacteristicRead, CharacteristicWrite, CharacteristicWriteMethod, ReqError, Service,
};
use keynear_core::{
    SecretStore, KEYNEAR_BLE_SERVICE_UUID, LAN_ENDPOINT_CHAR_UUID, MAX_CONTROL_FRAME_BYTES,
    MAX_GATT_PAIRING_FRAME_BYTES,
};
use keynear_protocol::KeynearError;
use tokio::sync::{mpsc, watch, Mutex};

use super::endpoint::{
    decode_lan_endpoint_from_known_peers, encode_lan_endpoint_for_peer, EncodeEndpointParams,
};
use super::{
    GattApplicationContext, GattInboundResult, PairingUiEvent, PAIRING_RX_CHAR_UUID,
    PAIRING_TX_CHAR_UUID,
};

type NotifierSlot = Arc<Mutex<Option<bluer::gatt::local::CharacteristicNotifier>>>;
type AddrMap = Arc<Mutex<BTreeMap<String, String>>>;

fn pairing_tx_write(
    responder: Arc<Mutex<super::GattPairingResponder>>,
    notifier_slot: NotifierSlot,
    event_tx: Option<mpsc::Sender<PairingUiEvent>>,
) -> CharacteristicWrite {
    CharacteristicWrite {
        write: true,
        write_without_response: false,
        method: CharacteristicWriteMethod::Fun(Box::new(move |value, req| {
            let responder = Arc::clone(&responder);
            let notifier_slot = Arc::clone(&notifier_slot);
            let event_tx = event_tx.clone();
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
                        if let Some(notifier) = notifier_slot.lock().await.as_mut() {
                            let _ = notifier.notify(frame).await;
                        }
                        Ok(())
                    }
                    Ok(GattInboundResult::AwaitingApproval { verification_code }) => {
                        if let Some(tx) = event_tx {
                            let _ =
                                tx.try_send(PairingUiEvent::VerificationReady(verification_code));
                        }
                        Ok(())
                    }
                    Ok(GattInboundResult::PeerExportBuffered) => Ok(()),
                    Ok(GattInboundResult::Completed(completed)) => {
                        if let Some(tx) = event_tx {
                            let _ = tx.try_send(PairingUiEvent::Completed {
                                peer_device_id: completed.peer_metadata.device_id.clone(),
                            });
                        }
                        Ok(())
                    }
                    Err(KeynearError::Validation { .. }) => Err(ReqError::InvalidValueLength),
                    Err(_) => Err(ReqError::Failed),
                }
            })
        })),
        ..Default::default()
    }
}

fn pairing_rx_notify(notifier_slot: NotifierSlot) -> CharacteristicNotify {
    CharacteristicNotify {
        notify: true,
        method: CharacteristicNotifyMethod::Fun(Box::new(move |notifier| {
            let slot = Arc::clone(&notifier_slot);
            Box::pin(async move {
                *slot.lock().await = Some(notifier);
            })
        })),
        ..Default::default()
    }
}

fn lan_endpoint_read(
    secrets: Arc<dyn SecretStore>,
    devices: keynear_core::PairedDeviceStore,
    lan_rx: Option<watch::Receiver<Option<keynear_protocol::v1::LanEndpoint>>>,
    addr_map: AddrMap,
) -> CharacteristicRead {
    CharacteristicRead {
        read: true,
        fun: Box::new(move |req| {
            let secrets = Arc::clone(&secrets);
            let devices = devices.clone();
            let lan_rx = lan_rx.clone();
            let addr_map = Arc::clone(&addr_map);
            let peer_addr = req.device_address.to_string();
            Box::pin(async move {
                let Some(endpoint) = lan_rx.as_ref().and_then(|rx| rx.borrow().clone()) else {
                    return Err(ReqError::Failed);
                };
                let known_id = addr_map
                    .lock()
                    .await
                    .get(&peer_addr.to_ascii_lowercase())
                    .cloned();
                match encode_lan_endpoint_for_peer(EncodeEndpointParams {
                    endpoint: &endpoint,
                    known_device_id: known_id.as_deref(),
                    peer_ble_address: Some(&peer_addr),
                    device_store: &devices,
                    secret_store: secrets.as_ref(),
                }) {
                    Ok(bytes) => Ok(bytes),
                    Err(KeynearError::UnauthorizedPeer(_)) => Err(ReqError::NotPermitted),
                    Err(_) => Err(ReqError::Failed),
                }
            })
        }),
        ..Default::default()
    }
}

fn lan_endpoint_write(
    secrets: Arc<dyn SecretStore>,
    devices: keynear_core::PairedDeviceStore,
    lan_tx: Option<mpsc::Sender<keynear_protocol::v1::LanEndpoint>>,
    addr_map: AddrMap,
) -> CharacteristicWrite {
    CharacteristicWrite {
        write: true,
        write_without_response: false,
        method: CharacteristicWriteMethod::Fun(Box::new(move |value, req| {
            let secrets = Arc::clone(&secrets);
            let devices = devices.clone();
            let lan_tx = lan_tx.clone();
            let addr_map = Arc::clone(&addr_map);
            let peer_addr = req.device_address.to_string();
            Box::pin(async move {
                if value.is_empty() || value.len() > MAX_CONTROL_FRAME_BYTES {
                    return Err(ReqError::InvalidValueLength);
                }
                match decode_lan_endpoint_from_known_peers(&value, &devices, secrets.as_ref()) {
                    Ok(endpoint) => {
                        addr_map
                            .lock()
                            .await
                            .insert(peer_addr.to_ascii_lowercase(), endpoint.device_id.clone());
                        if let Some(tx) = lan_tx {
                            let _ = tx.try_send(endpoint);
                        }
                        Ok(())
                    }
                    Err(KeynearError::Validation { .. }) => Err(ReqError::InvalidValueLength),
                    Err(KeynearError::UnauthorizedPeer(_)) => Err(ReqError::NotPermitted),
                    Err(_) => Err(ReqError::Failed),
                }
            })
        })),
        ..Default::default()
    }
}

pub(crate) fn build_gatt_application(ctx: GattApplicationContext) -> Application {
    let tx_write = pairing_tx_write(
        ctx.responder,
        Arc::clone(&ctx.notifier_slot),
        ctx.pairing_event_tx,
    );
    let rx_notify = pairing_rx_notify(ctx.notifier_slot);
    let lan_read = lan_endpoint_read(
        ctx.secret_store.clone(),
        ctx.device_store.clone(),
        ctx.local_lan_endpoint_rx,
        ctx.ble_addr_map.clone(),
    );
    let lan_write = lan_endpoint_write(
        ctx.secret_store,
        ctx.device_store,
        ctx.lan_endpoint_tx,
        ctx.ble_addr_map,
    );

    Application {
        services: vec![Service {
            uuid: KEYNEAR_BLE_SERVICE_UUID,
            primary: true,
            characteristics: vec![
                Characteristic {
                    uuid: PAIRING_TX_CHAR_UUID,
                    write: Some(tx_write),
                    ..Default::default()
                },
                Characteristic {
                    uuid: PAIRING_RX_CHAR_UUID,
                    notify: Some(rx_notify),
                    ..Default::default()
                },
                Characteristic {
                    uuid: LAN_ENDPOINT_CHAR_UUID,
                    read: Some(lan_read),
                    write: Some(lan_write),
                    ..Default::default()
                },
            ],
            ..Default::default()
        }],
        ..Default::default()
    }
}
