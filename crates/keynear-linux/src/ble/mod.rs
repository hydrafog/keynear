pub mod adapter;
pub mod advertiser;
pub mod gatt;

pub use adapter::{BleDriverParams, BlePeerDriver};

pub const DEFAULT_CAPABILITY_FLAGS: u32 = 0x07;

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;
    use std::sync::Arc;
    use std::time::{SystemTime, UNIX_EPOCH};

    use keynear_core::{
        commit_paired_device, InMemorySecretStore, LanEndpointCodec, NoisePairingSession,
        PairedBeacon, PairedBeaconState, PairedDeviceStore, PairingPeerMetadata, PairingRole,
        PeerSecrets, SecretStore, KEYNEAR_BLE_SERVICE_UUID, LAN_ENDPOINT_CHAR_UUID,
        MAX_BLE_SERVICE_DATA_BYTES, MAX_GATT_PAIRING_FRAME_BYTES,
    };
    use keynear_protocol::{v1, DeviceRole, KeynearError, PairedDevice, VisibilityMode};
    use tokio::sync::Mutex;
    use uuid::Uuid;

    use super::advertiser::{
        build_advertisement_payload, decode_service_data_for_known_peers, DecodedBleBeacon,
    };
    use super::gatt::{
        build_gatt_application, decode_lan_endpoint_from_known_peers, encode_lan_endpoint_for_peer,
        GattApplicationContext, GattInboundResult, GattPairingResponder, PAIRING_RX_CHAR_UUID,
        PAIRING_TX_CHAR_UUID,
    };
    use super::*;

    fn temp_devices_store(label: &str) -> (std::path::PathBuf, PairedDeviceStore) {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("keynear-linux-{label}-{nonce}"));
        fs::create_dir_all(&dir).expect("tempdir");
        let store = PairedDeviceStore::new(dir.join("devices.toml"));
        (dir, store)
    }

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
    fn gatt_loopback_pairing_completes_and_persists_peer_and_secrets() {
        let (dir, device_store) = temp_devices_store("gatt-loopback");
        let resp_secrets = Arc::new(InMemorySecretStore::new());
        let init_secrets = InMemorySecretStore::new();
        let init_kp = init_secrets
            .get_or_create_static_keypair()
            .expect("init kp");

        let mut responder = GattPairingResponder::new(
            PairingPeerMetadata {
                device_id: "linux-workstation".to_string(),
                display_name: "Workstation".to_string(),
                device_role: DeviceRole::Desktop,
                capability_flags: DEFAULT_CAPABILITY_FLAGS,
            },
            Arc::clone(&resp_secrets) as Arc<dyn SecretStore>,
            device_store.clone(),
        );

        let mut initiator = NoisePairingSession::new_initiator(
            &init_kp.private_key,
            PairingPeerMetadata {
                device_id: "pixel-9".to_string(),
                display_name: "Pixel 9".to_string(),
                device_role: DeviceRole::Mobile,
                capability_flags: 0x03,
            },
        )
        .expect("initiator session");

        let m1 = initiator.start_handshake().expect("m1");
        let GattInboundResult::Outbound(m2) = responder
            .handle_tx_write(&m1, Some("11:22:33:44:55:66".to_string()))
            .expect("m2")
        else {
            panic!("expected M2 outbound frame");
        };
        let m3 = initiator.step(&m2).expect("m3").expect("m3 frame");
        let GattInboundResult::AwaitingApproval { verification_code } = responder
            .handle_tx_write(&m3, Some("11:22:33:44:55:66".to_string()))
            .expect("awaiting approval")
        else {
            panic!("expected AwaitingApproval");
        };

        assert_eq!(
            verification_code,
            initiator.formatted_verification_code().expect("init code")
        );
        assert_eq!(initiator.role(), PairingRole::Initiator);

        let resp_export = responder.approve().expect("resp approve");
        let init_export = initiator.approve_and_build_export().expect("init approve");

        let init_done = initiator
            .complete_with_peer_export(&resp_export)
            .expect("init complete");
        let GattInboundResult::Completed(resp_done) = responder
            .handle_tx_write(&init_export, Some("11:22:33:44:55:66".to_string()))
            .expect("resp complete")
        else {
            panic!("expected Completed");
        };

        assert_eq!(init_done.reconnect_secret, resp_done.reconnect_secret);
        let devices = device_store.load().expect("load devices");
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].device_id, "pixel-9");
        assert_eq!(devices[0].last_ble_address, "11:22:33:44:55:66");

        let stored_secrets = resp_secrets
            .get_peer_secrets("pixel-9")
            .expect("get peer secrets")
            .expect("peer secrets stored");
        assert_eq!(stored_secrets.reconnect_secret, init_done.reconnect_secret);

        let oversized = vec![0u8; MAX_GATT_PAIRING_FRAME_BYTES + 1];
        assert!(matches!(
            responder.handle_tx_write(&oversized, None),
            Err(KeynearError::Validation {
                field: "gattFrame",
                ..
            })
        ));

        let _ = fs::remove_dir_all(&dir);
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

    #[test]
    fn lan_endpoint_gatt_characteristic_roundtrips_and_rejects_unpaired() {
        let (dir, device_store) = temp_devices_store("lan-char");
        let secrets = InMemorySecretStore::new();
        let pixel_secret = [0x55u8; 32];
        let tablet_secret = [0x66u8; 32];

        commit_paired_device(
            &device_store,
            &secrets,
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
            &device_store,
            &secrets,
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

        let local_ep = v1::LanEndpoint {
            device_id: "linux-workstation".to_string(),
            endpoint_id: vec![0x11; 32],
            socket_addrs: vec!["192.168.1.20:42000".to_string()],
            capability_flags: DEFAULT_CAPABILITY_FLAGS,
            issued_at_ms: 1_700_000_000_000,
        };
        let encoded = encode_lan_endpoint_for_peer(
            &local_ep,
            None,
            Some("11:22:33:44:55:66"),
            &device_store,
            &secrets,
        )
        .expect("encode for matched peer");
        let decoded_by_peer =
            LanEndpointCodec::decode(&encoded, "linux-workstation", &pixel_secret)
                .expect("peer decodes local endpoint");
        assert_eq!(decoded_by_peer, local_ep);

        let unrecognized = encode_lan_endpoint_for_peer(
            &local_ep,
            None,
            Some("FF:FF:FF:FF:FF:FF"),
            &device_store,
            &secrets,
        );
        assert!(matches!(
            unrecognized,
            Err(KeynearError::UnauthorizedPeer(_))
        ));

        let inbound_ep = v1::LanEndpoint {
            device_id: "pixel-9".to_string(),
            endpoint_id: vec![0x22; 32],
            socket_addrs: vec!["192.168.1.33:42001".to_string()],
            capability_flags: 0x03,
            issued_at_ms: 1_700_000_000_123,
        };
        let peer_wire =
            LanEndpointCodec::encode(&inbound_ep, &pixel_secret).expect("encode inbound");
        let decoded_inbound =
            decode_lan_endpoint_from_known_peers(&peer_wire, &device_store, &secrets)
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
            decode_lan_endpoint_from_known_peers(&rogue_wire, &device_store, &secrets),
            Err(KeynearError::UnauthorizedPeer(_))
        ));

        let responder = Arc::new(Mutex::new(GattPairingResponder::new(
            PairingPeerMetadata {
                device_id: "linux-workstation".to_string(),
                display_name: "Workstation".to_string(),
                device_role: DeviceRole::Desktop,
                capability_flags: DEFAULT_CAPABILITY_FLAGS,
            },
            Arc::new(secrets),
            device_store,
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

        let _ = fs::remove_dir_all(&dir);
    }
}
