use std::collections::BTreeMap;

use keynear_core::{
    InMemorySecretStore, PairedBeacon, PairedBeaconState, PeerSecrets, SecretStore,
    MAX_BLE_SERVICE_DATA_BYTES,
};
use keynear_protocol::{DeviceRole, PairedDevice, VisibilityMode};

use super::advertiser::{
    build_advertisement_payload, decode_service_data_for_known_peers, DecodedBleBeacon,
};
use super::*;

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
