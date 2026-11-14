mod app;
#[allow(dead_code)]
#[path = "../../keynear-linux/src/ipc.rs"]
mod ipc;
mod portal;
mod state;
mod views;

#[cfg(test)]
mod app_tests;
#[cfg(test)]
mod cli_tests;

use std::path::PathBuf;
use std::process;
use std::sync::Arc;

use keynear_core::{
    default_config_path, sanitize_device_id, DaemonConfig, KeynearError, Oo7SecretStore,
    PairedDeviceStore, SecretStore,
};
use tracing::info;
use tracing_subscriber::EnvFilter;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsCliAction {
    Version,
    Help,
    Validate {
        config_path: Option<PathBuf>,
    },
    Run {
        config_path: Option<PathBuf>,
    },
    SendPicker {
        config_path: Option<PathBuf>,
        device_id: Option<String>,
    },
    Share {
        config_path: Option<PathBuf>,
        device_id: Option<String>,
        paths: Vec<PathBuf>,
    },
}

pub fn parse_settings_args<I, S>(args: I) -> Result<SettingsCliAction, KeynearError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut iter = args.into_iter().map(Into::into);
    let mut config_path: Option<PathBuf> = None;
    let mut validate_only = false;
    let mut send_picker = false;
    let mut share_mode = false;
    let mut device_id: Option<String> = None;
    let mut share_paths: Vec<PathBuf> = Vec::new();
    let mut literal_paths = false;

    while let Some(arg) = iter.next() {
        if literal_paths {
            if arg.trim().is_empty() {
                return Err(KeynearError::Validation {
                    field: "paths",
                    message: "--share paths must not be empty".to_string(),
                });
            }
            share_paths.push(PathBuf::from(arg));
            continue;
        }
        match arg.as_str() {
            "--version" | "-V" => return Ok(SettingsCliAction::Version),
            "--help" | "-h" => return Ok(SettingsCliAction::Help),
            "--validate" => validate_only = true,
            "--send-picker" => send_picker = true,
            "--share" => share_mode = true,
            "--" if share_mode => {
                literal_paths = true;
            }
            "--device" => {
                let raw = iter.next().ok_or_else(|| KeynearError::Validation {
                    field: "deviceId",
                    message: "--device requires a device_id".to_string(),
                })?;
                device_id = Some(sanitize_device_id(&raw)?);
            }
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
            other if share_mode && !other.starts_with('-') => {
                if other.trim().is_empty() {
                    return Err(KeynearError::Validation {
                        field: "paths",
                        message: "--share paths must not be empty".to_string(),
                    });
                }
                share_paths.push(PathBuf::from(other));
            }
            other => {
                return Err(KeynearError::Validation {
                    field: "args",
                    message: format!("unknown argument '{other}'"),
                });
            }
        }
    }

    if (validate_only && (send_picker || share_mode || device_id.is_some()))
        || (send_picker && share_mode)
    {
        return Err(KeynearError::Validation {
            field: "args",
            message: "conflicting CLI mode flags".to_string(),
        });
    }
    if device_id.is_some() && !send_picker && !share_mode {
        return Err(KeynearError::Validation {
            field: "deviceId",
            message: "--device requires --send-picker or --share".to_string(),
        });
    }

    if validate_only {
        Ok(SettingsCliAction::Validate { config_path })
    } else if send_picker {
        Ok(SettingsCliAction::SendPicker {
            config_path,
            device_id,
        })
    } else if share_mode {
        if share_paths.is_empty() {
            return Err(KeynearError::Validation {
                field: "paths",
                message: "--share requires at least one file or directory path".to_string(),
            });
        }
        Ok(SettingsCliAction::Share {
            config_path,
            device_id,
            paths: share_paths,
        })
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
        "#DEE6F6",
        "#10141C",
        "#E7EDF8",
        "#1E202B",
        "Plus Jakarta Sans",
        "min-height: 44px",
        ".keynear-window",
        ".keynear-card",
        ".keynear-code",
        ".keynear-warning",
        ".keynear-error",
        ".keynear-danger-btn",
        ".keynear-switch",
        ".keynear-progress",
        ".keynear-transfer-row",
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

fn launch_settings_gui(
    config_path: Option<PathBuf>,
    send_picker_active: bool,
    selected_send_device_id: Option<String>,
    send_paths: Vec<PathBuf>,
) {
    if let Err(err) = validate_css_tokens() {
        eprintln!("error: {err}");
        process::exit(1);
    }
    let resolved_config_path = config_path.clone().unwrap_or_else(default_config_path);
    let (mut config, error_message) = match load_settings_config(config_path.as_ref()) {
        Ok(config) => (config, None),
        Err(err) => (DaemonConfig::default(), Some(err.to_string())),
    };
    let stores = load_stores();
    let mut paired_devices = stores.paired_devices;
    let mut everyone_remaining_secs = None;
    let mut transfer_error = None;

    match ipc::send_request(&ipc::IpcRequest::Status) {
        Ok(ipc::IpcResponse::Status(status)) => {
            config.visibility_mode = status.visibility_mode;
            config.auto_accept_from_paired = status.auto_accept_from_paired;
            config.incoming_directory = status.incoming_directory;
            everyone_remaining_secs = status.everyone_remaining_secs;
        }
        Ok(_) => {}
        Err(err) if send_picker_active => {
            transfer_error = Some(format!(
                "Keynear daemon unavailable ({err}); start keynear-linux and retry."
            ));
        }
        Err(_) => {}
    }

    if let Ok(ipc::IpcResponse::Config(daemon_cfg)) = ipc::send_request(&ipc::IpcRequest::GetConfig)
    {
        config = daemon_cfg;
    }
    if let Ok(ipc::IpcResponse::Devices(devices)) = ipc::send_request(&ipc::IpcRequest::ListDevices)
    {
        paired_devices = devices;
    }
    let initial_transfers = match ipc::send_request(&ipc::IpcRequest::ListTransfers) {
        Ok(ipc::IpcResponse::Transfers(transfers)) => transfers,
        _ => Vec::new(),
    };

    info!(
        event = "settings_launching",
        visibility = ?config.visibility_mode,
        paired_count = paired_devices.len(),
        send_picker_active,
        staged_send_count = send_paths.len(),
        has_error = error_message.is_some()
    );

    app::run_gui(app::SettingsInit {
        config,
        config_path: Some(resolved_config_path),
        everyone_remaining_secs,
        error_message,
        warning_message: stores.warning_message,
        transfer_error,
        paired_devices,
        nearby_peers: Vec::new(),
        transfers: initial_transfers,
        send_paths,
        selected_send_device_id,
        send_picker_active,
        device_store: stores.device_store,
        secret_store: stores.secret_store,
        ipc_socket_path: Some(ipc::daemon_socket_path()),
    });
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
            println!(
                "Usage: keynear-settings [--config <path>] [--validate] [--version] [--help]\n       keynear-settings --send-picker [--device <device_id>] [--config <path>]\n       keynear-settings --share [--device <device_id>] -- <path>..."
            );
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
            launch_settings_gui(config_path, false, None, Vec::new());
        }
        SettingsCliAction::SendPicker {
            config_path,
            device_id,
        } => {
            launch_settings_gui(config_path, true, device_id, Vec::new());
        }
        SettingsCliAction::Share {
            config_path,
            device_id,
            paths,
        } => {
            launch_settings_gui(config_path, true, device_id, paths);
        }
    }
}
