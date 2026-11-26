use std::collections::BTreeMap;

use bluer::{Adapter, Address, DeviceEvent, DeviceProperty};
use futures_util::StreamExt;
use keynear_core::PairedBeaconState;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use super::super::advertiser::DecodedBleBeacon;
use super::DiscoveredBlePeer;

pub(super) type RssiWatcherEntry = (String, PairedBeaconState, JoinHandle<()>);

pub(super) fn handle_device_removed(
    watchers: &mut BTreeMap<Address, RssiWatcherEntry>,
    discovered_tx: &Option<mpsc::Sender<DiscoveredBlePeer>>,
    addr: Address,
) {
    let Some((device_id, state, handle)) = watchers.remove(&addr) else {
        return;
    };
    handle.abort();
    if let Some(tx) = discovered_tx {
        let _ = tx.try_send(DiscoveredBlePeer {
            address: addr.to_string(),
            rssi: Some(-127),
            beacon: DecodedBleBeacon::Paired { device_id, state },
        });
    }
}

pub(super) struct RssiWatchTarget<'a> {
    pub addr: Address,
    pub device_id: &'a str,
    pub state: PairedBeaconState,
}

pub(super) fn ensure_rssi_watcher(
    adapter: &Adapter,
    watchers: &mut BTreeMap<Address, RssiWatcherEntry>,
    discovered_tx: &Option<mpsc::Sender<DiscoveredBlePeer>>,
    target: RssiWatchTarget<'_>,
) {
    let RssiWatchTarget {
        addr,
        device_id,
        state,
    } = target;
    if watchers.get(&addr).is_some_and(|(_, _, h)| !h.is_finished()) {
        return;
    }
    let Some(tx) = discovered_tx.clone() else {
        return;
    };
    let adapter = adapter.clone();
    let (watch_id, watch_state) = (device_id.to_string(), state.clone());
    let handle = tokio::spawn(async move {
        let Ok(device) = adapter.device(addr) else {
            return;
        };
        let Ok(mut evs) = device.events().await else {
            return;
        };
        while let Some(ev) = evs.next().await {
            if let DeviceEvent::PropertyChanged(DeviceProperty::Rssi(rssi)) = ev {
                let peer = DiscoveredBlePeer {
                    address: addr.to_string(),
                    rssi: Some(rssi),
                    beacon: DecodedBleBeacon::Paired {
                        device_id: watch_id.clone(),
                        state: watch_state.clone(),
                    },
                };
                if tx.try_send(peer).is_err() && tx.is_closed() {
                    break;
                }
            }
        }
    });
    watchers.insert(addr, (device_id.to_string(), state, handle));
}
