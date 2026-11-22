use std::collections::BTreeMap;
use std::fs;
use std::sync::Arc;

use keynear_core::{
    commit_paired_device, InMemorySecretStore, LanEndpointCodec, PairingPeerMetadata, PeerSecrets,
    KEYNEAR_BLE_SERVICE_UUID, LAN_ENDPOINT_CHAR_UUID,
};
use keynear_protocol::{v1, DeviceRole, KeynearError, PairedDevice};
use tokio::sync::Mutex;
use uuid::Uuid;

use super::gatt::{
    build_gatt_application, decode_lan_endpoint_from_known_peers, encode_lan_endpoint_for_peer,
    EncodeEndpointParams, GattApplicationContext, GattPairingResponder, PAIRING_RX_CHAR_UUID,
    PAIRING_TX_CHAR_UUID,
};
use super::*;

fn seed_test_devices(store: &PairedDeviceStore, secrets: &InMemorySecretStore) {
    let pixel_secret = [0x55u8; 32];
    let tablet_secret = [0x66u8; 32];

    commit_paired_device(
        store,
        secrets,
        PairedDevice {
            device_id: "pixel-9".to_string(),
            display_name: "Pixel 9".to_string(),
            device_role: DeviceRole::Mobile,
            static_x25519_pubkey: "aa".repeat(32),
            last_ble_address: "11:22:33:44:55:66".to_string(),
            last_lan_address: String::new(),
            last_endpoint_id: String::new(),
            capability_flags: 0x07,
            auto_accept_files: false,
            paired_at: "1".to_string(),
        },
        &PeerSecrets {
            irk: pixel_secret,
            reconnect_secret: pixel_secret,
        },
    )
    .expect("commit pixel");

    commit_paired_device(
        store,
        secrets,
        PairedDevice {
            device_id: "galaxy-tab".to_string(),
            display_name: "Galaxy Tab".to_string(),
            device_role: DeviceRole::Mobile,
            static_x25519_pubkey: "bb".repeat(32),
            last_ble_address: "AA:BB:CC:DD:EE:FF".to_string(),
            last_lan_address: String::new(),
            last_endpoint_id: String::new(),
            capability_flags: 0x07,
            auto_accept_files: false,
            paired_at: "2".to_string(),
        },
        &PeerSecrets {
            irk: tablet_secret,
            reconnect_secret: tablet_secret,
        },
    )
    .expect("commit tablet");
}

fn check_endpoint_roundtrip(
    store: &PairedDeviceStore,
    secrets: &InMemorySecretStore,
    pixel_secret: [u8; 32],
) {
    let local_ep = v1::LanEndpoint {
        device_id: "linux-workstation".to_string(),
        endpoint_id: vec![0x11; 32],
        socket_addrs: vec!["192.168.1.20:42000".to_string()],
        capability_flags: DEFAULT_CAPABILITY_FLAGS,
        issued_at_ms: 1_700_000_000_000,
    };
    let encoded = encode_lan_endpoint_for_peer(EncodeEndpointParams {
        endpoint: &local_ep,
        known_device_id: None,
        peer_ble_address: Some("11:22:33:44:55:66"),
        device_store: store,
        secret_store: secrets,
    })
    .expect("encode for matched peer");
    let decoded_by_peer = LanEndpointCodec::decode(&encoded, "linux-workstation", &pixel_secret)
        .expect("peer decodes local endpoint");
    assert_eq!(decoded_by_peer, local_ep);

    let unrecognized = encode_lan_endpoint_for_peer(EncodeEndpointParams {
        endpoint: &local_ep,
        known_device_id: None,
        peer_ble_address: Some("FF:FF:FF:FF:FF:FF"),
        device_store: store,
        secret_store: secrets,
    });
    assert!(matches!(
        unrecognized,
        Err(KeynearError::UnauthorizedPeer(_))
    ));
}

fn check_decode_roundtrip(
    store: &PairedDeviceStore,
    secrets: &InMemorySecretStore,
    pixel_secret: [u8; 32],
) {
    let inbound_ep = v1::LanEndpoint {
        device_id: "pixel-9".to_string(),
        endpoint_id: vec![0x22; 32],
        socket_addrs: vec!["192.168.1.33:42001".to_string()],
        capability_flags: 0x03,
        issued_at_ms: 1_700_000_000_123,
    };
    let peer_wire = LanEndpointCodec::encode(&inbound_ep, &pixel_secret).expect("encode inbound");
    let decoded_inbound = decode_lan_endpoint_from_known_peers(&peer_wire, store, secrets)
        .expect("decode known peer write");
    assert_eq!(decoded_inbound, inbound_ep);

    let rogue_ep = v1::LanEndpoint {
        device_id: "rogue-device".to_string(),
        endpoint_id: vec![0x99; 32],
        socket_addrs: vec!["192.168.1.99:42002".to_string()],
        capability_flags: 0x01,
        issued_at_ms: 1_700_000_000_999,
    };
    let rogue_wire = LanEndpointCodec::encode(&rogue_ep, &[0x99; 32]).expect("encode rogue");
    assert!(matches!(
        decode_lan_endpoint_from_known_peers(&rogue_wire, store, secrets),
        Err(KeynearError::UnauthorizedPeer(_))
    ));
}

fn check_gatt_application(
    dir: &std::path::Path,
    store: PairedDeviceStore,
    secrets: InMemorySecretStore,
) {
    let responder = Arc::new(Mutex::new(GattPairingResponder::new(
        PairingPeerMetadata {
            device_id: "linux-workstation".to_string(),
            display_name: "Workstation".to_string(),
            device_role: DeviceRole::Desktop,
            capability_flags: DEFAULT_CAPABILITY_FLAGS,
        },
        Arc::new(secrets),
        store,
    )));
    let app = build_gatt_application(GattApplicationContext {
        responder,
        notifier_slot: Arc::new(Mutex::new(None)),
        pairing_event_tx: None,
        secret_store: Arc::new(InMemorySecretStore::new()),
        device_store: PairedDeviceStore::new(dir.join("devices.toml")),
        local_lan_endpoint_rx: None,
        lan_endpoint_tx: None,
        ble_addr_map: Arc::new(Mutex::new(BTreeMap::new())),
    });
    assert_eq!(app.services.len(), 1);
    assert_eq!(app.services[0].uuid, KEYNEAR_BLE_SERVICE_UUID);
    let uuids: Vec<Uuid> = app.services[0]
        .characteristics
        .iter()
        .map(|c| c.uuid)
        .collect();
    assert!(uuids.contains(&PAIRING_TX_CHAR_UUID));
    assert!(uuids.contains(&PAIRING_RX_CHAR_UUID));
    assert!(uuids.contains(&LAN_ENDPOINT_CHAR_UUID));
}

#[test]
fn lan_endpoint_gatt_characteristic_roundtrips_and_rejects_unpaired() {
    let (dir, device_store) = temp_devices_store("lan-char");
    let secrets = InMemorySecretStore::new();
    let pixel_secret = [0x55u8; 32];
    seed_test_devices(&device_store, &secrets);
    check_endpoint_roundtrip(&device_store, &secrets, pixel_secret);
    check_decode_roundtrip(&device_store, &secrets, pixel_secret);
    check_gatt_application(&dir, device_store, secrets);
    let _ = fs::remove_dir_all(&dir);
}
