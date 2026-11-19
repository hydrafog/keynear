use std::collections::BTreeMap;
use std::sync::Arc;

use bluer::gatt::local::CharacteristicNotifier;
use bluer::{Adapter, AdapterEvent, Address, DiscoveryFilter, DiscoveryTransport, Session};
use futures_util::StreamExt;
use keynear_core::{PairedDeviceStore, PairingPeerMetadata, SecretStore, KEYNEAR_BLE_SERVICE_UUID};
use keynear_protocol::{v1, DeviceRole, KeynearError, VisibilityMode};
use tokio::sync::{mpsc, watch, Mutex};

use super::advertiser::{
    decode_service_data_for_known_peers, register_visibility_advertisement, DecodedBleBeacon,
};
use super::gatt::{
    build_gatt_application, GattApplicationContext, GattPairingResponder, PairingUiEvent,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredBlePeer {
    pub address: String,
    pub rssi: Option<i16>,
    pub beacon: DecodedBleBeacon,
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
