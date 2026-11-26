use super::*;

use keynear_core::derive_presence_token;
use keynear_protocol::{DeviceRole, PairedDevice, PairedDeviceStore};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_devices_path() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock is after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("keynear-ffi-handle-{nanos}.toml"))
}

fn sample_lan_endpoint() -> FfiLanEndpoint {
    FfiLanEndpoint {
        device_id: "pixel-9".to_string(),
        endpoint_id: vec![0xAA; 32],
        socket_addrs: vec!["192.168.1.10:5000".to_string()],
        capability_flags: 3,
        issued_at_ms: 1_763_000_000_000,
    }
}

#[test]
fn derive_presence_token_bytes_matches_core_derivation() {
    let handle = CoreHandle::new();
    let token = handle
        .derive_presence_token_bytes("pixel-9".to_string(), 7, vec![0x55; 32])
        .expect("derive token");
    assert_eq!(
        token,
        derive_presence_token("pixel-9", 7, &[0x55; 32])
            .expect("core token")
            .to_vec()
    );
    assert_eq!(
        handle
            .derive_presence_token_bytes("pixel-9".to_string(), 8, vec![0x55; 32])
            .expect("derive next token"),
        derive_presence_token("pixel-9", 8, &[0x55; 32])
            .expect("core next token")
            .to_vec()
    );
    assert!(matches!(
        handle.derive_presence_token_bytes("pixel-9".to_string(), 7, vec![0u8; 5]),
        Err(FfiError::Validation { .. })
    ));
}

#[test]
fn set_auto_accept_files_updates_persisted_device() {
    let path = unique_devices_path();
    let store = PairedDeviceStore::new(path.clone());
    store
        .upsert(PairedDevice {
            device_id: "pixel-9".to_string(),
            display_name: "Pixel 9".to_string(),
            device_role: DeviceRole::Mobile,
            static_x25519_pubkey: "pubkey-hex".to_string(),
            last_ble_address: "AA:BB:CC:DD:EE:FF".to_string(),
            last_lan_address: String::new(),
            last_endpoint_id: String::new(),
            capability_flags: 1,
            auto_accept_files: false,
            paired_at: "2026-11-13T00:00:00Z".to_string(),
        })
        .expect("seed device");

    let handle = CoreHandle::new();
    let path_text = path.to_string_lossy().to_string();
    let updated = handle
        .set_auto_accept_files(path_text.clone(), "pixel-9".to_string(), true)
        .expect("set auto accept");
    assert!(updated.auto_accept_files);
    assert!(
        store
            .load()
            .expect("reload")
            .first()
            .expect("device present")
            .auto_accept_files
    );
    assert!(matches!(
        handle.set_auto_accept_files(path_text, "absent".to_string(), true),
        Err(FfiError::Validation { .. })
    ));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn lan_endpoint_roundtrip_and_rejects_invalid_inputs() {
    let handle = CoreHandle::new();
    let endpoint = sample_lan_endpoint();
    let key = vec![0x33; 32];
    let encoded = handle
        .encode_lan_endpoint(endpoint.clone(), key.clone())
        .expect("encode endpoint");
    assert_eq!(
        handle
            .decode_lan_endpoint(encoded.clone(), "pixel-9".to_string(), key.clone())
            .expect("decode endpoint"),
        endpoint
    );
    assert!(matches!(
        handle.encode_lan_endpoint(
            FfiLanEndpoint {
                endpoint_id: vec![0xAA; 31],
                ..endpoint.clone()
            },
            key.clone()
        ),
        Err(FfiError::Validation { .. })
    ));
    assert!(matches!(
        handle.encode_lan_endpoint(endpoint, vec![0u8; 5]),
        Err(FfiError::Validation { .. })
    ));
    assert!(matches!(
        handle.decode_lan_endpoint(encoded, "other-dev".to_string(), key.clone()),
        Err(FfiError::Crypto { .. })
    ));
    assert!(matches!(
        handle.decode_lan_endpoint(vec![0u8; 2], "pixel-9".to_string(), key),
        Err(FfiError::Validation { .. })
    ));
}
