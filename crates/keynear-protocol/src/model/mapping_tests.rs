use super::*;

#[test]
fn default_config_dir_returns_nonempty_path() {
    let dir = default_config_dir();
    assert!(!dir.as_os_str().is_empty());
}

#[test]
fn default_config_and_devices_paths_extend_config_dir() {
    let dir = default_config_dir();
    assert_eq!(default_config_path(), dir.join("config.toml"));
    assert_eq!(default_devices_path(), dir.join("devices.toml"));
    assert_eq!(
        default_config_path()
            .file_name()
            .and_then(|name| name.to_str()),
        Some("config.toml")
    );
    assert_eq!(
        default_devices_path()
            .file_name()
            .and_then(|name| name.to_str()),
        Some("devices.toml")
    );
}

#[test]
fn paired_device_store_default_location_points_at_default_devices_path() {
    let store = PairedDeviceStore::default_location();
    assert_eq!(store.path(), default_devices_path());
}

#[test]
fn daemon_config_validate_covers_negative_and_interior_bounds() {
    DaemonConfig::default()
        .validate()
        .expect("default config is valid");

    for distance in [f32::NEG_INFINITY, -1.0, 0.25, 201.0] {
        let config = DaemonConfig {
            proximity_distance_meters: distance,
            ..DaemonConfig::default()
        };
        assert!(matches!(
            config.validate(),
            Err(KeynearError::Validation {
                field: "proximityDistanceMeters",
                ..
            })
        ));
    }

    for distance in [0.5, 0.75, 100.0, 200.0] {
        let config = DaemonConfig {
            proximity_distance_meters: distance,
            ..DaemonConfig::default()
        };
        config.validate().expect("in-range distance accepted");
    }
}

#[test]
fn daemon_config_validate_checks_incoming_directory_before_distance() {
    let config = DaemonConfig {
        incoming_directory: "\t \n".to_string(),
        proximity_distance_meters: f32::NAN,
        ..DaemonConfig::default()
    };
    assert!(matches!(
        config.validate(),
        Err(KeynearError::Validation {
            field: "incomingDirectory",
            ..
        })
    ));
}
