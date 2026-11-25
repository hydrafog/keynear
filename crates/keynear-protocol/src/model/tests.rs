use super::*;
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_temp_path(name: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock is after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("keynear-{name}-{nanos}.toml"))
}

fn sample_device(id: &str, name: &str) -> PairedDevice {
    PairedDevice {
        device_id: id.to_string(),
        display_name: name.to_string(),
        device_role: DeviceRole::Desktop,
        static_x25519_pubkey: "pubkey-hex".to_string(),
        last_ble_address: "AA:BB:CC:DD:EE:FF".to_string(),
        last_lan_address: "192.168.1.10:5000".to_string(),
        last_endpoint_id: String::new(),
        capability_flags: 1,
        auto_accept_files: true,
        paired_at: "2026-11-13T00:00:00Z".to_string(),
    }
}

#[test]
fn config_missing_returns_default() {
    let missing = unique_temp_path("missing");
    let loaded = DaemonConfig::load_or_default(&missing).expect("missing file uses default");
    assert_eq!(loaded, DaemonConfig::default());
}

#[test]
fn config_malformed_returns_error() {
    let path = unique_temp_path("malformed");
    fs::write(&path, "visibilityMode = [not valid toml").expect("write temp file");
    let err = DaemonConfig::load_or_default(&path).expect_err("malformed toml must fail");
    let _ = fs::remove_file(&path);
    assert!(matches!(err, KeynearError::Config(_)));
}

#[test]
fn config_empty_incoming_directory_fails_validation() {
    let path = unique_temp_path("empty-dir");
    fs::write(&path, "incomingDirectory = \"   \"\n").expect("write temp file");
    let err = DaemonConfig::load_or_default(&path).expect_err("empty dir must fail");
    let _ = fs::remove_file(&path);
    assert!(matches!(
        err,
        KeynearError::Validation {
            field: "incomingDirectory",
            ..
        }
    ));
}

#[test]
fn secret_fields_absent_from_serialized_paired_device() {
    let device = sample_device("dev-1", "Workstation");
    let encoded = toml::to_string(&device).expect("serialize paired device");
    assert!(!encoded.contains("irk"));
    assert!(!encoded.contains("reconnectSecret"));
    assert!(!encoded.contains("reconnect_secret"));
    assert!(!encoded.contains("private_key"));
    assert!(!encoded.contains("quic_secret"));
}

#[test]
fn paired_device_store_updates_auto_accept_and_lan_endpoint() {
    let path = unique_temp_path("orion-updates");
    let store = PairedDeviceStore::new(&path);
    let mut dev = sample_device("dev-orion", "Pixel");
    dev.auto_accept_files = false;
    store.upsert(dev).expect("upsert");

    let updated = store
        .set_auto_accept_files("dev-orion", true)
        .expect("set auto accept");
    assert!(updated.auto_accept_files);

    let updated = store
        .update_lan_endpoint("dev-orion", "0123abcd", "192.168.1.44:54321")
        .expect("update lan endpoint");
    assert_eq!(updated.last_endpoint_id, "0123abcd");
    assert_eq!(updated.last_lan_address, "192.168.1.44:54321");

    let _ = fs::remove_file(&path);
}

#[test]
fn paired_device_store_roundtrip_and_malformed_rejection() {
    let path = unique_temp_path("devices");
    let store = PairedDeviceStore::new(&path);

    assert!(store
        .load()
        .expect("missing devices file returns empty")
        .is_empty());

    let updated = store
        .upsert(sample_device("dev-b", "Pixel"))
        .expect("upsert first device");
    assert_eq!(updated.len(), 1);

    let updated = store
        .upsert(sample_device("dev-a", "ThinkPad"))
        .expect("upsert second device");
    assert_eq!(updated.len(), 2);
    assert_eq!(updated[0].device_id, "dev-a");
    assert_eq!(updated[1].device_id, "dev-b");

    let raw = fs::read_to_string(&path).expect("read saved devices.toml");
    assert!(!raw.contains("irk"));
    assert!(!raw.contains("reconnectSecret"));
    assert!(!raw.contains("private_key"));

    assert!(store.remove("dev-a").expect("remove existing device"));
    assert!(!store
        .remove("dev-a")
        .expect("remove missing device is false"));
    assert_eq!(store.load().expect("reload after remove").len(), 1);

    fs::write(&path, "devices = [invalid").expect("corrupt devices.toml");
    assert!(matches!(store.load(), Err(KeynearError::Config(_))));
    let _ = fs::remove_file(&path);
}

#[test]
fn sanitize_device_id_rejects_traversal_and_empty() {
    assert!(sanitize_device_id("").is_err());
    assert!(sanitize_device_id("   ").is_err());
    assert!(sanitize_device_id("../etc/passwd").is_err());
    assert!(sanitize_device_id("a/b").is_err());
    assert!(sanitize_device_id("a..b").is_err());
    assert_eq!(
        sanitize_device_id(" pixel-9_pro.1 ").unwrap(),
        "pixel-9_pro.1"
    );
}

#[test]
fn validate_display_name_rejects_empty_control_and_overlong() {
    assert!(matches!(
        validate_display_name(""),
        Err(KeynearError::Validation {
            field: "displayName",
            ..
        })
    ));
    assert!(matches!(
        validate_display_name("   \t\n  "),
        Err(KeynearError::Validation {
            field: "displayName",
            ..
        })
    ));
    assert!(matches!(
        validate_display_name("bad\x07name"),
        Err(KeynearError::Validation {
            field: "displayName",
            ..
        })
    ));
    let overlong = "a".repeat(MAX_DISPLAY_NAME_LEN + 1);
    assert!(matches!(
        validate_display_name(&overlong),
        Err(KeynearError::Validation {
            field: "displayName",
            ..
        })
    ));
    assert_eq!(
        validate_display_name("  Work Laptop  ").expect("valid trimmed name"),
        "Work Laptop"
    );
}

#[test]
fn paired_device_store_rename_updates_name_and_preserves_other_fields() {
    let path = unique_temp_path("rename");
    let store = PairedDeviceStore::new(path.clone());
    store
        .upsert(sample_device("pixel-9", "Pixel 9"))
        .expect("seed paired device");

    let renamed = store
        .rename("pixel-9", "  Studio Phone  ")
        .expect("rename succeeds");
    assert_eq!(renamed.display_name, "Studio Phone");
    assert_eq!(renamed.static_x25519_pubkey, "pubkey-hex");
    assert!(renamed.auto_accept_files);

    let loaded = store
        .get("pixel-9")
        .expect("load renamed")
        .expect("present");
    assert_eq!(loaded.display_name, "Studio Phone");

    let raw = fs::read_to_string(&path).expect("read devices toml");
    assert!(!raw.contains("reconnect_secret"));
    assert!(store.rename("missing-dev", "New Name").is_err());

    let _ = fs::remove_file(&path);
}

#[test]
fn daemon_config_save_to_path_roundtrips_with_restricted_mode() {
    let dir = unique_temp_path("cfg-dir").with_extension("");
    let path = dir.join("config.toml");
    let cfg = DaemonConfig {
        visibility_mode: VisibilityMode::Hidden,
        incoming_directory: "/tmp/keynear-in".to_string(),
        auto_accept_from_paired: true,
        ..DaemonConfig::default()
    };
    cfg.save_to_path(&path).expect("save config");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(&path).expect("meta").permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }

    let loaded = DaemonConfig::load_or_default(&path).expect("reload config");
    assert_eq!(loaded, cfg);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn config_rejects_invalid_proximity_distance_bounds() {
    for distance in [0.0, 0.49, 200.01, f32::NAN, f32::INFINITY] {
        let cfg = DaemonConfig {
            proximity_distance_meters: distance,
            ..DaemonConfig::default()
        };
        assert!(matches!(
            cfg.validate(),
            Err(KeynearError::Validation {
                field: "proximityDistanceMeters",
                ..
            })
        ));
    }
    let cfg = DaemonConfig {
        proximity_distance_meters: 0.5,
        ..DaemonConfig::default()
    };
    cfg.validate().expect("lower bound accepted");
    let cfg = DaemonConfig {
        proximity_distance_meters: 200.0,
        ..DaemonConfig::default()
    };
    cfg.validate().expect("upper bound accepted");
}

#[test]
fn config_load_or_default_rejects_malformed_toml() {
    let path = unique_temp_path("malformed-config");
    std::fs::write(&path, b"visibilityMode = [not valid toml").expect("write");
    assert!(matches!(
        DaemonConfig::load_or_default(&path),
        Err(KeynearError::Config(_))
    ));
    let invalid = DaemonConfig {
        proximity_distance_meters: f32::NAN,
        ..DaemonConfig::default()
    };
    invalid
        .save_to_path(&path)
        .expect_err("invalid config must not be saved");
    std::fs::remove_file(&path).expect("cleanup");
}

#[test]
fn config_save_preserves_all_phoenix_flags() {
    let path = unique_temp_path("phoenix-config");
    let cfg = DaemonConfig {
        audio_continuity_enabled: false,
        proximity_lock_enabled: false,
        presence_gating_enabled: false,
        proximity_distance_meters: 5.5,
        ..DaemonConfig::default()
    };
    cfg.save_to_path(&path).expect("save");
    let loaded = DaemonConfig::load_or_default(&path).expect("load");
    assert_eq!(loaded, cfg);
    std::fs::remove_file(&path).expect("cleanup");
}
