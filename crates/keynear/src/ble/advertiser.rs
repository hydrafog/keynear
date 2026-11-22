use std::collections::{BTreeMap, BTreeSet};

use bluer::adv::{Advertisement, AdvertisementHandle};
use bluer::Adapter;
use keynear_core::{
    EveryoneNearbyBeacon, PairedBeacon, PairedBeaconState, PairedDevice, PairedDeviceStore,
    SecretStore, KEYNEAR_BLE_SERVICE_UUID, MAX_BLE_SERVICE_DATA_BYTES,
};
use keynear_protocol::{DeviceRole, KeynearError, VisibilityMode};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodedBleBeacon {
    EveryoneNearby(EveryoneNearbyBeacon),
    Paired {
        device_id: String,
        state: PairedBeaconState,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BleAdvertisementPayload {
    pub service_uuid: uuid::Uuid,
    pub service_data: Vec<u8>,
    pub visibility: VisibilityMode,
}

#[allow(clippy::too_many_arguments)]
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

#[allow(clippy::too_many_arguments)]
pub async fn register_visibility_advertisement(
    adapter: &Adapter,
    visibility: VisibilityMode,
    local_device_id: &str,
    display_name: &str,
    capability_flags: u32,
    secret_store: &dyn SecretStore,
    device_store: &PairedDeviceStore,
) -> Result<Option<AdvertisementHandle>, KeynearError> {
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
        discoverable: Some(visibility == VisibilityMode::EveryoneNearby),
        ..Default::default()
    };

    let handle = adapter
        .advertise(adv)
        .await
        .map_err(|err| KeynearError::Bluetooth(format!("le advertise failed: {err}")))?;
    Ok(Some(handle))
}
