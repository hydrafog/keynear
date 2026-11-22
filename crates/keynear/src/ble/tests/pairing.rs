use std::fs;
use std::sync::Arc;

use keynear_core::{
    InMemorySecretStore, NoisePairingSession, PairingPeerMetadata, PairingRole, SecretStore,
    MAX_GATT_PAIRING_FRAME_BYTES,
};
use keynear_protocol::{DeviceRole, KeynearError};

use super::gatt::{GattInboundResult, GattPairingResponder};
use super::*;

fn test_responder(store: PairedDeviceStore, secrets: Arc<dyn SecretStore>) -> GattPairingResponder {
    GattPairingResponder::new(
        PairingPeerMetadata {
            device_id: "linux-workstation".to_string(),
            display_name: "Workstation".to_string(),
            device_role: DeviceRole::Desktop,
            capability_flags: DEFAULT_CAPABILITY_FLAGS,
        },
        secrets,
        store,
    )
}

fn test_initiator(private_key: &[u8; 32]) -> NoisePairingSession {
    NoisePairingSession::new_initiator(
        private_key,
        PairingPeerMetadata {
            device_id: "pixel-9".to_string(),
            display_name: "Pixel 9".to_string(),
            device_role: DeviceRole::Mobile,
            capability_flags: 0x03,
        },
    )
    .expect("initiator session")
}

fn run_handshake(
    responder: &mut GattPairingResponder,
    initiator: &mut NoisePairingSession,
) -> (Vec<u8>, Vec<u8>) {
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
    (resp_export, init_export)
}

#[test]
fn gatt_loopback_pairing_completes_and_persists_peer_and_secrets() {
    let (dir, device_store) = temp_devices_store("gatt-loopback");
    let resp_secrets = Arc::new(InMemorySecretStore::new());
    let init_secrets = InMemorySecretStore::new();
    let init_kp = init_secrets
        .get_or_create_static_keypair()
        .expect("init kp");
    let mut responder = test_responder(
        device_store.clone(),
        Arc::clone(&resp_secrets) as Arc<dyn SecretStore>,
    );
    let mut initiator = test_initiator(&init_kp.private_key);
    let (resp_export, init_export) = run_handshake(&mut responder, &mut initiator);

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
fn gatt_pairing_completes_when_initiator_approves_before_responder() {
    let (dir, device_store) = temp_devices_store("gatt-init-first");
    let resp_secrets = Arc::new(InMemorySecretStore::new());
    let init_secrets = InMemorySecretStore::new();
    let init_kp = init_secrets
        .get_or_create_static_keypair()
        .expect("init kp");
    let mut responder = test_responder(
        device_store.clone(),
        Arc::clone(&resp_secrets) as Arc<dyn SecretStore>,
    );
    let mut initiator = test_initiator(&init_kp.private_key);

    let m1 = initiator.start_handshake().expect("m1");
    let GattInboundResult::Outbound(m2) = responder
        .handle_tx_write(&m1, Some("11:22:33:44:55:66".to_string()))
        .expect("m2")
    else {
        panic!("expected M2 outbound frame");
    };
    let m3 = initiator.step(&m2).expect("m3").expect("m3 frame");
    let GattInboundResult::AwaitingApproval { .. } = responder
        .handle_tx_write(&m3, Some("11:22:33:44:55:66".to_string()))
        .expect("awaiting approval")
    else {
        panic!("expected AwaitingApproval");
    };

    let init_export = initiator.approve_and_build_export().expect("init approve");
    let buffered = responder
        .handle_tx_write(&init_export, Some("11:22:33:44:55:66".to_string()))
        .expect("buffer peer export");
    assert!(matches!(buffered, GattInboundResult::PeerExportBuffered));

    let resp_export = responder.approve().expect("resp approve");
    let resp_done = responder
        .complete_pending_export()
        .expect("complete buffered export")
        .expect("buffered export completed");
    let init_done = initiator
        .complete_with_peer_export(&resp_export)
        .expect("init complete");

    assert_eq!(init_done.reconnect_secret, resp_done.reconnect_secret);
    assert_eq!(device_store.load().expect("load devices").len(), 1);
    assert!(resp_secrets
        .get_peer_secrets("pixel-9")
        .expect("secrets")
        .is_some());

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn gatt_approve_without_active_session_returns_validation_error() {
    let (dir, device_store) = temp_devices_store("gatt-no-session");
    let resp_secrets = Arc::new(InMemorySecretStore::new());
    let mut responder = test_responder(
        device_store,
        Arc::clone(&resp_secrets) as Arc<dyn SecretStore>,
    );
    assert!(matches!(
        responder.approve(),
        Err(KeynearError::Validation {
            field: "pairingState",
            ..
        })
    ));
    assert!(responder.reject().is_ok());
    let _ = fs::remove_dir_all(&dir);
}
