mod app;

use std::path::PathBuf;
use std::process;
use std::sync::Arc;

use keynear_core::{
    default_config_path, DaemonConfig, KeynearError, Oo7SecretStore, PairedDeviceStore, SecretStore,
};
use tracing::info;
use tracing_subscriber::EnvFilter;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsCliAction {
    Version,
    Help,
    Validate { config_path: Option<PathBuf> },
    Run { config_path: Option<PathBuf> },
}

pub fn parse_settings_args<I, S>(args: I) -> Result<SettingsCliAction, KeynearError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut iter = args.into_iter().map(Into::into);
    let mut config_path: Option<PathBuf> = None;
    let mut validate_only = false;

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--version" | "-V" => return Ok(SettingsCliAction::Version),
            "--help" | "-h" => return Ok(SettingsCliAction::Help),
            "--validate" => validate_only = true,
            "--config" => {
                let value = iter.next().ok_or_else(|| KeynearError::Validation {
                    field: "config",
                    message: "--config requires a file path".to_string(),
                })?;
                if value.trim().is_empty() {
                    return Err(KeynearError::Validation {
                        field: "config",
                        message: "--config requires a non-empty file path".to_string(),
                    });
                }
                config_path = Some(PathBuf::from(value));
            }
            other => {
                return Err(KeynearError::Validation {
                    field: "args",
                    message: format!("unknown argument '{other}'"),
                });
            }
        }
    }

    if validate_only {
        Ok(SettingsCliAction::Validate { config_path })
    } else {
        Ok(SettingsCliAction::Run { config_path })
    }
}

pub fn load_settings_config(explicit_path: Option<&PathBuf>) -> Result<DaemonConfig, KeynearError> {
    match explicit_path {
        Some(path) => {
            if !path.exists() {
                return Err(KeynearError::Config(format!(
                    "configuration file not found: {}",
                    path.display()
                )));
            }
            DaemonConfig::load_or_default(path)
        }
        None => DaemonConfig::load_or_default(&default_config_path()),
    }
}

pub fn validate_css_tokens() -> Result<(), KeynearError> {
    let required = [
        "#F0F4FA",
        "#0B1016",
        "#004CED",
        "#0172F9",
        "#FFF6D3",
        "#381C00",
        "#FFE7E7",
        "#8B0016",
        "Plus Jakarta Sans",
        "min-height: 44px",
        ".keynear-window",
        ".keynear-card",
        ".keynear-code",
        ".keynear-warning",
        ".keynear-error",
        ".keynear-danger-btn",
    ];
    for needle in required {
        if !app::STYLE_CSS.contains(needle) {
            return Err(KeynearError::Validation {
                field: "style_css",
                message: format!("missing required design token or selector '{needle}'"),
            });
        }
    }
    Ok(())
}

struct LoadedStores {
    device_store: PairedDeviceStore,
    paired_devices: Vec<keynear_core::PairedDevice>,
    secret_store: Option<Arc<dyn SecretStore>>,
    warning_message: Option<String>,
}

fn load_stores() -> LoadedStores {
    let device_store = PairedDeviceStore::default_location();
    let (paired_devices, device_warn) = match device_store.load() {
        Ok(list) => (list, None),
        Err(err) => (
            Vec::new(),
            Some(format!("Failed to read paired devices: {err}")),
        ),
    };
    let (secret_store, keyring_warn): (Option<Arc<dyn SecretStore>>, Option<String>) =
        match Oo7SecretStore::open_default() {
            Ok(store) => (Some(Arc::new(store)), None),
            Err(err) => (
                None,
                Some(format!(
                    "Keyring unavailable ({err}); running in read-only mode."
                )),
            ),
        };
    let warning_message = device_warn.or(keyring_warn);
    LoadedStores {
        device_store,
        paired_devices,
        secret_store,
        warning_message,
    }
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let action = match parse_settings_args(std::env::args().skip(1)) {
        Ok(action) => action,
        Err(err) => {
            eprintln!("error: {err}");
            process::exit(2);
        }
    };

    match action {
        SettingsCliAction::Version => {
            println!("keynear-settings {}", env!("CARGO_PKG_VERSION"));
        }
        SettingsCliAction::Help => {
            println!("Usage: keynear-settings [--config <path>] [--validate] [--version] [--help]");
        }
        SettingsCliAction::Validate { config_path } => {
            if let Err(err) = validate_css_tokens() {
                eprintln!("error: {err}");
                process::exit(1);
            }
            let config = match load_settings_config(config_path.as_ref()) {
                Ok(config) => config,
                Err(err) => {
                    eprintln!("error: {err}");
                    process::exit(1);
                }
            };
            info!(
                event = "settings_validated",
                visibility = ?config.visibility_mode,
                incoming_dir = %config.incoming_directory
            );
        }
        SettingsCliAction::Run { config_path } => {
            if let Err(err) = validate_css_tokens() {
                eprintln!("error: {err}");
                process::exit(1);
            }
            let (config, error_message) = match load_settings_config(config_path.as_ref()) {
                Ok(config) => (config, None),
                Err(err) => (DaemonConfig::default(), Some(err.to_string())),
            };
            let stores = load_stores();
            info!(
                event = "settings_launching",
                visibility = ?config.visibility_mode,
                paired_count = stores.paired_devices.len(),
                has_error = error_message.is_some()
            );
            app::run_gui(app::SettingsInit {
                config,
                error_message,
                warning_message: stores.warning_message,
                paired_devices: stores.paired_devices,
                nearby_peers: Vec::new(),
                device_store: stores.device_store,
                secret_store: stores.secret_store,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use keynear_core::{
        commit_paired_device, DeviceRole, InMemorySecretStore, PairedDevice, PeerSecrets,
        VisibilityMode, EVERYONE_NEARBY_TIMEOUT_SECS, PAIRING_TIMEOUT_SECS,
    };

    use super::*;

    fn temp_store(label: &str) -> (PathBuf, PairedDeviceStore) {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("keynear-settings-{label}-{nonce}"));
        fs::create_dir_all(&dir).expect("tempdir");
        let store = PairedDeviceStore::new(dir.join("devices.toml"));
        (dir, store)
    }

    #[test]
    fn cli_parses_validate_and_config() {
        assert_eq!(
            parse_settings_args(["--validate", "--config", "/tmp/a.toml"]).unwrap(),
            SettingsCliAction::Validate {
                config_path: Some(PathBuf::from("/tmp/a.toml"))
            }
        );
        assert_eq!(
            parse_settings_args(["--version"]).unwrap(),
            SettingsCliAction::Version
        );
    }

    #[test]
    fn cli_rejects_missing_config_value() {
        let err =
            parse_settings_args(["--config"]).expect_err("missing --config operand must fail");
        assert!(matches!(
            err,
            KeynearError::Validation {
                field: "config",
                ..
            }
        ));
    }

    #[test]
    fn css_tokens_match_design_tokens_json() {
        validate_css_tokens().expect("css tokens valid");
        let raw_json = include_str!("../../../design/tokens.json");
        let parsed: serde_json::Value =
            serde_json::from_str(raw_json).expect("parse design/tokens.json");
        let light_canvas = parsed["light"]["canvas"].as_str().unwrap();
        let dark_canvas = parsed["dark"]["canvas"].as_str().unwrap();
        let light_accent = parsed["light"]["accent"].as_str().unwrap();
        let focus_ring = parsed["light"]["focusRing"].as_str().unwrap();
        let light_warning = parsed["light"]["warningTonal"].as_str().unwrap();
        let light_danger = parsed["light"]["errorTonal"].as_str().unwrap();

        assert!(app::STYLE_CSS.contains(light_canvas));
        assert!(app::STYLE_CSS.contains(dark_canvas));
        assert!(app::STYLE_CSS.contains(light_accent));
        assert!(app::STYLE_CSS.contains(focus_ring));
        assert!(app::STYLE_CSS.contains(light_warning));
        assert!(app::STYLE_CSS.contains(light_danger));
    }

    #[test]
    fn headless_validate_mode_loads_default_config_and_css() {
        validate_css_tokens().expect("css tokens valid");
        let config = DaemonConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn visibility_countdown_ticks_and_auto_reverts() {
        let (dir, device_store) = temp_store("timer");
        let mut model = app::SettingsModel::from_init(app::SettingsInit {
            config: DaemonConfig::default(),
            error_message: None,
            warning_message: None,
            paired_devices: Vec::new(),
            nearby_peers: Vec::new(),
            device_store,
            secret_store: None,
        });

        assert_eq!(model.visibility_mode(), VisibilityMode::PairedOnly);
        model.apply_msg(app::SettingsMsg::CycleVisibility);
        assert_eq!(model.visibility_mode(), VisibilityMode::EveryoneNearby);
        assert_eq!(model.visibility_status_text(), "Everyone (10:00)");

        model.apply_msg(app::SettingsMsg::Tick);
        assert_eq!(model.visibility_status_text(), "Everyone (09:59)");

        for _ in 1..EVERYONE_NEARBY_TIMEOUT_SECS {
            model.apply_msg(app::SettingsMsg::Tick);
        }
        assert_eq!(model.visibility_mode(), VisibilityMode::PairedOnly);
        assert_eq!(model.visibility_status_text(), "Paired devices only");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn unpair_and_pairing_timeout_transitions_remove_secrets_and_expire() {
        let (dir, device_store) = temp_store("unpair");
        let secret_store = Arc::new(InMemorySecretStore::new());
        let dev = PairedDevice {
            device_id: "pixel-9".to_string(),
            display_name: "Pixel 9".to_string(),
            device_role: DeviceRole::Mobile,
            static_x25519_pubkey: "ab".repeat(32),
            last_ble_address: String::new(),
            last_lan_address: String::new(),
            capability_flags: 0x03,
            auto_accept_files: false,
            paired_at: "1".to_string(),
        };
        commit_paired_device(
            &device_store,
            secret_store.as_ref(),
            dev.clone(),
            &PeerSecrets {
                irk: [0x11; 32],
                reconnect_secret: [0x22; 32],
            },
        )
        .expect("commit device");

        let mut model = app::SettingsModel::from_init(app::SettingsInit {
            config: DaemonConfig::default(),
            error_message: None,
            warning_message: None,
            paired_devices: vec![dev],
            nearby_peers: Vec::new(),
            device_store: device_store.clone(),
            secret_store: Some(Arc::clone(&secret_store) as Arc<dyn SecretStore>),
        });

        model.apply_msg(app::SettingsMsg::RequestUnpair("pixel-9".to_string()));
        assert_eq!(model.pending_unpair_id(), Some("pixel-9"));
        model.apply_msg(app::SettingsMsg::ConfirmUnpair("pixel-9".to_string()));
        assert!(model.paired_devices().is_empty());
        assert!(device_store.load().expect("load").is_empty());
        assert!(secret_store
            .get_peer_secrets("pixel-9")
            .expect("get")
            .is_none());

        model.set_pending_pairing_for_test(app::PendingPairingView {
            peer_token: "tok-1".to_string(),
            peer_name: "Pixel 9".to_string(),
            verification_code: "482 193".to_string(),
            remaining_secs: PAIRING_TIMEOUT_SECS,
        });
        for _ in 0..PAIRING_TIMEOUT_SECS {
            model.apply_msg(app::SettingsMsg::Tick);
        }
        assert!(model.pending_pairing().is_none());
        assert!(model
            .error_message()
            .unwrap_or_default()
            .contains("120 seconds"));

        let _ = fs::remove_dir_all(&dir);
    }
}
