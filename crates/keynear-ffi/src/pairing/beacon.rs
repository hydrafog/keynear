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

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_endpoint() -> FfiLanEndpoint {
        FfiLanEndpoint {
            device_id: "pixel-9".to_string(),
            endpoint_id: vec![0xAA; 32],
            socket_addrs: vec!["192.168.1.10:5000".to_string()],
            capability_flags: 3,
            issued_at_ms: 1_763_000_000_000,
        }
    }

    #[test]
    fn derive_presence_token_bytes_impl_matches_core_derivation() {
        let token = derive_presence_token_bytes_impl("pixel-9".to_string(), 7, vec![0x55; 32])
            .expect("derive token");
        let expected =
            keynear_core::derive_presence_token("pixel-9", 7, &[0x55; 32]).expect("core token");
        assert_eq!(token, expected.to_vec());
        assert!(matches!(
            derive_presence_token_bytes_impl("pixel-9".to_string(), 7, vec![0u8; 5]),
            Err(FfiError::Validation { .. })
        ));
    }

    #[test]
    fn lan_endpoint_impl_roundtrips_and_rejects_invalid_inputs() {
        let endpoint = sample_endpoint();
        let key = vec![0x33; 32];
        let encoded =
            encode_lan_endpoint_impl(endpoint.clone(), key.clone()).expect("encode endpoint");
        assert_eq!(
            decode_lan_endpoint_impl(encoded.clone(), "pixel-9".to_string(), key.clone())
                .expect("decode endpoint"),
            endpoint
        );

        assert!(matches!(
            encode_lan_endpoint_impl(
                FfiLanEndpoint {
                    endpoint_id: vec![0xAA; 31],
                    ..endpoint
                },
                key.clone()
            ),
            Err(FfiError::Validation { .. })
        ));
        assert!(matches!(
            decode_lan_endpoint_impl(encoded, "other-dev".to_string(), key.clone()),
            Err(FfiError::Crypto { .. })
        ));
        assert!(matches!(
            decode_lan_endpoint_impl(vec![0u8; 2], "pixel-9".to_string(), key),
            Err(FfiError::Validation { .. })
        ));
    }
}
