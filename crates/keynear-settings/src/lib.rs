pub mod app;
pub mod portal;
pub mod state;
pub mod views;

use std::path::PathBuf;

use keynear_protocol::{
    default_config_path, sanitize_device_id, DaemonConfig, KeynearError, PairedDeviceStore,
};

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

pub fn load_device_store_snapshot() -> (
    PairedDeviceStore,
    Vec<keynear_protocol::PairedDevice>,
    Option<String>,
) {
    let device_store = PairedDeviceStore::default_location();
    match device_store.load() {
        Ok(list) => (device_store, list, None),
        Err(err) => (
            device_store,
            Vec::new(),
            Some(format!("Failed to read paired devices: {err}")),
        ),
    }
}
