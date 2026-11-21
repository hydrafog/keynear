use crate::pairing::*;

pub(crate) fn encode_everyone_nearby_beacon_impl(
    role: FfiDeviceRole,
    capability_flags: u32,
    display_name: String,
) -> Result<Vec<u8>, FfiError> {
    let beacon = EveryoneNearbyBeacon {
        role: DeviceRole::from(role),
        capability_flags,
        display_name,
    };
    beacon.encode().map_err(FfiError::from)
}

pub(crate) fn decode_everyone_nearby_beacon_impl(
    service_data: Vec<u8>,
) -> Result<FfiEveryoneNearbyBeacon, FfiError> {
    let decoded = EveryoneNearbyBeacon::decode(&service_data)?;
    Ok(FfiEveryoneNearbyBeacon::from(decoded))
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn encode_paired_beacon_impl(
    device_id: String,
    counter: u32,
    reconnect_key: Vec<u8>,
    available: bool,
    battery_level: Option<u8>,
) -> Result<Vec<u8>, FfiError> {
    let key = to_key_32(&reconnect_key, "reconnectKey")?;
    let encoded = PairedBeacon::encode(&device_id, counter, &key, available, battery_level)?;
    Ok(encoded.to_vec())
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn decode_paired_beacon_impl(
    service_data: Vec<u8>,
    device_id: String,
    expected_counter: u32,
    last_accepted_counter: Option<u32>,
    reconnect_key: Vec<u8>,
) -> Result<FfiPairedBeaconState, FfiError> {
    let key = to_key_32(&reconnect_key, "reconnectKey")?;
    let decoded = PairedBeacon::decode_and_verify(
        &service_data,
        &device_id,
        expected_counter,
        last_accepted_counter,
        &key,
    )?;
    Ok(FfiPairedBeaconState::from(decoded))
}

pub(crate) fn derive_presence_token_bytes_impl(
    device_id: String,
    counter: u32,
    reconnect_key: Vec<u8>,
) -> Result<Vec<u8>, FfiError> {
    let key = to_key_32(&reconnect_key, "reconnectKey")?;
    let token = derive_presence_token(&device_id, counter, &key)?;
    Ok(token.to_vec())
}

pub(crate) fn encode_lan_endpoint_impl(
    endpoint: FfiLanEndpoint,
    reconnect_key: Vec<u8>,
) -> Result<Vec<u8>, FfiError> {
    let key = to_key_32(&reconnect_key, "reconnectKey")?;
    let proto_ep = v1::LanEndpoint::from(endpoint);
    LanEndpointCodec::encode(&proto_ep, &key).map_err(FfiError::from)
}

pub(crate) fn decode_lan_endpoint_impl(
    encoded: Vec<u8>,
    expected_device_id: String,
    reconnect_key: Vec<u8>,
) -> Result<FfiLanEndpoint, FfiError> {
    let key = to_key_32(&reconnect_key, "reconnectKey")?;
    let decoded = LanEndpointCodec::decode(&encoded, &expected_device_id, &key)?;
    Ok(FfiLanEndpoint::from(decoded))
}
