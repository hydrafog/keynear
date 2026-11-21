use super::*;

fn test_key() -> [u8; 32] {
    [0x42; 32]
}

#[test]
fn beacon_fits_31_byte_ble_budget_and_rejects_oversized() {
    let key = test_key();
    let encoded =
        PairedBeacon::encode("pixel-9", 1, &key, true, Some(84)).expect("encode paired beacon");
    assert_eq!(encoded.len(), 24);
    assert!(encoded.len() + LEGACY_ADV_OVERHEAD_BYTES <= MAX_LEGACY_ADV_BYTES);

    let decoded = PairedBeacon::decode_and_verify(&encoded, "pixel-9", 1, None, &key)
        .expect("decode valid paired beacon");
    assert_eq!(
        decoded,
        PairedBeaconState {
            counter: 1,
            available: true,
            battery_level: Some(84),
        }
    );

    let mut oversized = vec![0u8; MAX_BLE_SERVICE_DATA_BYTES + 1];
    oversized[..24].copy_from_slice(&encoded);
    assert!(matches!(
        PairedBeacon::decode_and_verify(&oversized, "pixel-9", 1, None, &key),
        Err(KeynearError::Validation {
            field: "serviceData",
            ..
        })
    ));

    let long_name = "Workstation-Desktop-UTF8-😀😀😀-VeryLongName";
    let open = EveryoneNearbyBeacon {
        role: DeviceRole::Desktop,
        capability_flags: 0x07,
        display_name: long_name.to_string(),
    };
    let open_bytes = open.encode().expect("encode everyone-nearby beacon");
    assert!(open_bytes.len() <= MAX_BLE_SERVICE_DATA_BYTES);
    assert!(open_bytes.len() + LEGACY_ADV_OVERHEAD_BYTES <= MAX_LEGACY_ADV_BYTES);
    let decoded_open =
        EveryoneNearbyBeacon::decode(&open_bytes).expect("decode everyone-nearby beacon");
    assert_eq!(decoded_open.role, DeviceRole::Desktop);
    assert_eq!(decoded_open.capability_flags, 0x07);
    assert!(long_name.starts_with(&decoded_open.display_name));

    assert!(EveryoneNearbyBeacon::decode(&oversized).is_err());
}

#[test]
fn rejects_stale_replayed_and_tampered_tokens() {
    let key = test_key();
    let encoded =
        PairedBeacon::encode("thinkpad-x1", 10, &key, true, None).expect("encode paired beacon");

    assert!(matches!(
        PairedBeacon::decode_and_verify(&encoded, "thinkpad-x1", 10, Some(10), &key),
        Err(KeynearError::Crypto(_))
    ));

    assert!(matches!(
        PairedBeacon::decode_and_verify(&encoded, "thinkpad-x1", 30, None, &key),
        Err(KeynearError::Crypto(_))
    ));

    let mut tampered_token = encoded;
    tampered_token[0] ^= 0x01;
    assert!(matches!(
        PairedBeacon::decode_and_verify(&tampered_token, "thinkpad-x1", 10, None, &key),
        Err(KeynearError::Crypto(_))
    ));

    let mut tampered_ciphertext = encoded;
    tampered_ciphertext[10] ^= 0xFF;
    assert!(matches!(
        PairedBeacon::decode_and_verify(&tampered_ciphertext, "thinkpad-x1", 10, None, &key),
        Err(KeynearError::Crypto(_))
    ));

    let mut tampered_tag = encoded;
    tampered_tag[23] ^= 0x80;
    assert!(matches!(
        PairedBeacon::decode_and_verify(&tampered_tag, "thinkpad-x1", 10, None, &key),
        Err(KeynearError::Crypto(_))
    ));
}

#[test]
fn lan_endpoint_codec_roundtrips_and_rejects_tampering() {
    let key = [0x77; 32];
    let payload = v1::LanEndpoint {
        device_id: "pixel-9".to_string(),
        endpoint_id: vec![0x33; 32],
        socket_addrs: vec!["192.168.1.50:49000".to_string()],
        capability_flags: 1,
        issued_at_ms: 1_763_000_000_123,
    };

    let encoded = LanEndpointCodec::encode(&payload, &key).expect("encode lan endpoint");
    let encoded_2 = LanEndpointCodec::encode(&payload, &key).expect("encode again");
    assert_ne!(
        encoded[1..13],
        encoded_2[1..13],
        "fresh random nonce must be used per encryption"
    );

    let decoded = LanEndpointCodec::decode(&encoded, "pixel-9", &key).expect("decode");
    assert_eq!(decoded, payload);

    let wrong_key = [0x88; 32];
    assert!(matches!(
        LanEndpointCodec::decode(&encoded, "pixel-9", &wrong_key),
        Err(KeynearError::Crypto(_))
    ));
    assert!(matches!(
        LanEndpointCodec::decode(&encoded, "other-device", &key),
        Err(KeynearError::Crypto(_))
    ));

    let mut tampered = encoded.clone();
    let last = tampered.len() - 1;
    tampered[last] ^= 0x01;
    assert!(matches!(
        LanEndpointCodec::decode(&tampered, "pixel-9", &key),
        Err(KeynearError::Crypto(_))
    ));

    let matched =
        LanEndpointCodec::decode_any(&encoded, [("other-device", &wrong_key), ("pixel-9", &key)])
            .expect("decode_any matches pixel-9");
    assert_eq!(matched, payload);
}
