use super::*;
use keynear_protocol::model::DeviceRole;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_dir(label: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("keynear-keyring-{label}-{nanos}"))
}

#[test]
fn atomic_devices_store_rollback_and_monotonic_counter() {
    let dir = temp_dir("rollback");
    fs::create_dir_all(&dir).expect("create temp dir");
    let blocking_path = dir.join("not-a-dir");
    fs::write(&blocking_path, "block").expect("create blocking file");

    let bad_store = PairedDeviceStore::new(blocking_path.join("devices.toml"));
    let secrets = InMemorySecretStore::new();

    let dev = PairedDevice {
        device_id: "pixel-9".to_string(),
        display_name: "Pixel 9".to_string(),
        device_role: DeviceRole::Mobile,
        static_x25519_pubkey: "abcd".to_string(),
        last_ble_address: "11:22:33:44:55:66".to_string(),
        last_lan_address: String::new(),
        last_endpoint_id: String::new(),
        capability_flags: 1,
        auto_accept_files: false,
        paired_at: "2026-11-13T00:00:00Z".to_string(),
    };
    let peer_secrets = PeerSecrets {
        irk: [0x11; 32],
        reconnect_secret: [0x22; 32],
    };

    assert!(commit_paired_device(&bad_store, &secrets, dev.clone(), &peer_secrets).is_err());
    assert!(secrets
        .get_peer_secrets("pixel-9")
        .expect("query after rollback")
        .is_none());

    let good_store = PairedDeviceStore::new(dir.join("devices.toml"));
    let list =
        commit_paired_device(&good_store, &secrets, dev, &peer_secrets).expect("commit succeeds");
    assert_eq!(list.len(), 1);
    assert_eq!(
        secrets.get_peer_secrets("pixel-9").unwrap(),
        Some(peer_secrets)
    );

    let c1 = secrets.next_presence_counter().expect("c1");
    let c2 = secrets.next_presence_counter().expect("c2");
    assert!(c2 > c1);

    assert!(remove_paired_device(&good_store, &secrets, "pixel-9").expect("remove succeeds"));
    assert!(good_store.load().unwrap().is_empty());
    assert!(secrets.get_peer_secrets("pixel-9").unwrap().is_none());

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn secret_types_redact_debug_output() {
    let kp = StaticKeypairRecord::generate();
    let kp_dbg = format!("{kp:?}");
    assert!(!kp_dbg.contains(&hex::encode(kp.private_key)));
    assert!(kp_dbg.contains("[REDACTED]"));

    let ps = PeerSecrets {
        irk: [0xAA; 32],
        reconnect_secret: [0xBB; 32],
    };
    let ps_dbg = format!("{ps:?}");
    assert!(!ps_dbg.contains(&hex::encode(ps.irk)));
    assert!(!ps_dbg.contains(&hex::encode(ps.reconnect_secret)));
    assert!(ps_dbg.contains("[REDACTED]"));
}

#[test]
fn in_memory_quic_secret_key_is_stable() {
    let store = InMemorySecretStore::new();
    let first = store.get_or_create_quic_secret_key().expect("create key");
    let second = store.get_or_create_quic_secret_key().expect("reload key");
    assert_eq!(*first, *second);
    assert_ne!(*first, [0u8; 32]);
}
