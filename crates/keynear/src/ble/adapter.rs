use std::collections::BTreeMap;
use std::sync::Arc;

use bluer::adv::AdvertisementHandle;
use bluer::gatt::local::{ApplicationHandle, CharacteristicNotifier};
use bluer::{Adapter, AdapterEvent, Address, DiscoveryFilter, DiscoveryTransport, Session};
use futures_util::{Stream, StreamExt};
use keynear_core::{
    PairedDeviceStore, PairingPeerMetadata, SecretStore, KEYNEAR_BLE_SERVICE_UUID,
};
use keynear_protocol::{v1, DeviceRole, KeynearError, VisibilityMode};
use tokio::sync::{mpsc, watch, Mutex};

#[path = "adapter_rssi.rs"]
mod rssi;

use rssi::{ensure_rssi_watcher, handle_device_removed, RssiWatchTarget, RssiWatcherEntry};

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

struct GattSetup {
    responder: Arc<Mutex<GattPairingResponder>>,
    notifier_slot: Arc<Mutex<Option<CharacteristicNotifier>>>,
    pairing_event_tx: Option<mpsc::Sender<PairingUiEvent>>,
    ble_addr_map: Arc<Mutex<BTreeMap<String, String>>>,
}

struct DiscoveryContext<'a> {
    adapter: &'a Adapter,
    device_store: &'a PairedDeviceStore,
    secret_store: &'a dyn SecretStore,
    discovered_tx: &'a Option<mpsc::Sender<DiscoveredBlePeer>>,
    ble_addr_map: &'a Arc<Mutex<BTreeMap<String, String>>>,
    last_counters: &'a mut BTreeMap<String, u32>,
    rssi_watchers: &'a mut BTreeMap<Address, RssiWatcherEntry>,
}

struct EventLoopState<'a, S> {
    adapter: &'a Adapter,
    params: &'a mut BleDriverParams,
    setup: &'a GattSetup,
    adv_handle: Option<AdvertisementHandle>,
    events: S,
    last_counters: BTreeMap<String, u32>,
    rssi_watchers: BTreeMap<Address, RssiWatcherEntry>,
}

impl<S> EventLoopState<'_, S>
where
    S: Stream<Item = AdapterEvent> + Unpin,
{
    async fn run(mut self) -> Result<(), KeynearError> {
        while self.step().await? {}
        self.abort_watchers();
        drop(self.adv_handle);
        Ok(())
    }

    async fn step(&mut self) -> Result<bool, KeynearError> {
        tokio::select! {
            changed = self.params.visibility_rx.changed() => {
                if changed.is_err() {
                    return Ok(false);
                }
                self.refresh_advertisement().await?;
                Ok(true)
            }
            Some(approved) = recv_optional(&mut self.params.pairing_decision_rx) => {
                super::gatt::apply_pairing_decision(
                    &self.setup.responder, &self.setup.notifier_slot,
                    self.setup.pairing_event_tx.as_ref(), approved,
                ).await;
                Ok(true)
            }
            maybe_ev = self.events.next() => Ok(!self.dispatch_device_event(maybe_ev).await),
        }
    }

    async fn refresh_advertisement(&mut self) -> Result<(), KeynearError> {
        drop(self.adv_handle.take());
        let mode = *self.params.visibility_rx.borrow_and_update();
        self.adv_handle = register_advertisement(self.adapter, mode, self.params).await?;
        Ok(())
    }

    async fn dispatch_device_event(&mut self, maybe_ev: Option<AdapterEvent>) -> bool {
        let mut ctx = DiscoveryContext {
            adapter: self.adapter,
            device_store: &self.params.device_store,
            secret_store: self.params.secret_store.as_ref(),
            discovered_tx: &self.params.discovered_tx,
            ble_addr_map: &self.setup.ble_addr_map,
            last_counters: &mut self.last_counters,
            rssi_watchers: &mut self.rssi_watchers,
        };
        handle_device_event(&mut ctx, maybe_ev).await
    }

    fn abort_watchers(&mut self) {
        for (_, (_, _, handle)) in std::mem::take(&mut self.rssi_watchers) {
            handle.abort();
        }
    }
}

impl BlePeerDriver {
    pub async fn run_bluez(mut params: BleDriverParams) -> Result<(), KeynearError> {
        let adapter = init_adapter().await?;
        let setup = GattSetup::new(&params);
        let _gatt_handle = register_gatt(&adapter, &mut params, &setup).await?;
        let events = adapter
            .discover_devices_with_changes()
            .await
            .map_err(|e| KeynearError::Bluetooth(format!("le discovery start failed: {e}")))?;
        let initial_mode = *params.visibility_rx.borrow();
        let adv_handle = register_advertisement(&adapter, initial_mode, &params).await?;
        EventLoopState {
            adapter: &adapter,
            params: &mut params,
            setup: &setup,
            adv_handle,
            events,
            last_counters: BTreeMap::new(),
            rssi_watchers: BTreeMap::new(),
        }
        .run()
        .await
    }
}

async fn init_adapter() -> Result<Adapter, KeynearError> {
    let bt_err = |msg: &str, err: bluer::Error| KeynearError::Bluetooth(format!("{msg}: {err}"));
    let session = Session::new().await.map_err(|e| bt_err("dbus bluez session failed", e))?;
    let adapter = session.default_adapter().await.map_err(|e| bt_err("no default bluetooth adapter", e))?;
    if !adapter.is_powered().await.map_err(|e| bt_err("adapter power query failed", e))? {
        return Err(KeynearError::Bluetooth("default bluetooth adapter is powered off".to_string()));
    }
    let filter = DiscoveryFilter {
        transport: DiscoveryTransport::Le,
        duplicate_data: true,
        ..Default::default()
    };
    adapter.set_discovery_filter(filter).await.map_err(|e| bt_err("discovery filter failed", e))?;
    Ok(adapter)
}

impl GattSetup {
    fn new(params: &BleDriverParams) -> Self {
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
        Self {
            responder,
            notifier_slot: Arc::new(Mutex::new(None)),
            pairing_event_tx: params.pairing_event_tx.clone(),
            ble_addr_map: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }
}

async fn register_gatt(
    adapter: &Adapter,
    params: &mut BleDriverParams,
    setup: &GattSetup,
) -> Result<ApplicationHandle, KeynearError> {
    let app = build_gatt_application(GattApplicationContext {
        responder: Arc::clone(&setup.responder),
        notifier_slot: Arc::clone(&setup.notifier_slot),
        pairing_event_tx: params.pairing_event_tx.take(),
        secret_store: Arc::clone(&params.secret_store),
        device_store: params.device_store.clone(),
        local_lan_endpoint_rx: params.local_lan_endpoint_rx.clone(),
        lan_endpoint_tx: params.lan_endpoint_tx.clone(),
        ble_addr_map: Arc::clone(&setup.ble_addr_map),
    });
    adapter
        .serve_gatt_application(app)
        .await
        .map_err(|e| KeynearError::Bluetooth(format!("gatt registration failed: {e}")))
}

async fn register_advertisement(
    adapter: &Adapter,
    mode: VisibilityMode,
    params: &BleDriverParams,
) -> Result<Option<AdvertisementHandle>, KeynearError> {
    register_visibility_advertisement(
        adapter, mode, &params.local_device_id, &params.display_name,
        params.capability_flags, params.secret_store.as_ref(), &params.device_store,
    )
    .await
}

async fn handle_device_event(
    ctx: &mut DiscoveryContext<'_>,
    maybe_ev: Option<AdapterEvent>,
) -> bool {
    match maybe_ev {
        None => true,
        Some(AdapterEvent::DeviceRemoved(addr)) => {
            handle_device_removed(ctx.rssi_watchers, ctx.discovered_tx, addr);
            false
        }
        Some(AdapterEvent::DeviceAdded(addr)) => {
            let Some(peer) = inspect_discovered_device(ctx, addr).await else {
                return false;
            };
            if let DecodedBleBeacon::Paired { ref device_id, ref state } = peer.beacon {
                ctx.ble_addr_map
                    .lock()
                    .await
                    .insert(peer.address.to_ascii_lowercase(), device_id.clone());
                ensure_rssi_watcher(
                    ctx.adapter,
                    ctx.rssi_watchers,
                    ctx.discovered_tx,
                    RssiWatchTarget {
                        addr,
                        device_id,
                        state: state.clone(),
                    },
                );
            }
            if let Some(tx) = ctx.discovered_tx {
                let _ = tx.try_send(peer);
            }
            false
        }
        Some(_) => false,
    }
}

async fn inspect_discovered_device(
    ctx: &mut DiscoveryContext<'_>,
    addr: Address,
) -> Option<DiscoveredBlePeer> {
    let device = ctx.adapter.device(addr).ok()?;
    let service_data_map = device.service_data().await.ok().flatten()?;
    let bytes = service_data_map.get(&KEYNEAR_BLE_SERVICE_UUID)?;
    let paired_devices = ctx.device_store.load().unwrap_or_default();
    let decoded = decode_service_data_for_known_peers(
        bytes, &paired_devices, ctx.secret_store, &mut *ctx.last_counters,
    ).ok()??;
    let rssi = device.rssi().await.ok().flatten();
    Some(DiscoveredBlePeer { address: addr.to_string(), rssi, beacon: decoded })
}

async fn recv_optional<T>(rx: &mut Option<mpsc::Receiver<T>>) -> Option<T> {
    match rx {
        Some(inner) => inner.recv().await,
        None => std::future::pending().await,
    }
}
