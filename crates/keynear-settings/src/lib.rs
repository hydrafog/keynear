#![deny(
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    clippy::too_many_arguments
)]

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

#[derive(Default)]
struct SettingsArgsState {
    config_path: Option<PathBuf>,
    validate_only: bool,
    send_picker: bool,
    share_mode: bool,
    device_id: Option<String>,
    share_paths: Vec<PathBuf>,
    literal_paths: bool,
}

fn push_share_path(state: &mut SettingsArgsState, path: &str) -> Result<(), KeynearError> {
    if path.trim().is_empty() {
        return Err(KeynearError::Validation {
            field: "paths",
            message: "--share paths must not be empty".to_string(),
        });
    }
    state.share_paths.push(PathBuf::from(path));
    Ok(())
}

fn parse_device_arg(
    iter: &mut impl Iterator<Item = String>,
    state: &mut SettingsArgsState,
) -> Result<(), KeynearError> {
    let raw = iter.next().ok_or_else(|| KeynearError::Validation {
        field: "deviceId",
        message: "--device requires a device_id".to_string(),
    })?;
    state.device_id = Some(sanitize_device_id(&raw)?);
    Ok(())
}

fn parse_config_arg(
    iter: &mut impl Iterator<Item = String>,
    state: &mut SettingsArgsState,
) -> Result<(), KeynearError> {
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
    state.config_path = Some(PathBuf::from(value));
    Ok(())
}

fn consume_settings_args<I, S>(
    args: I,
    state: &mut SettingsArgsState,
) -> Result<Option<SettingsCliAction>, KeynearError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut iter = args.into_iter().map(Into::into);
    while let Some(arg) = iter.next() {
        if state.literal_paths {
            push_share_path(state, &arg)?;
            continue;
        }
        match arg.as_str() {
            "--version" | "-V" => return Ok(Some(SettingsCliAction::Version)),
            "--help" | "-h" => return Ok(Some(SettingsCliAction::Help)),
            "--validate" => state.validate_only = true,
            "--send-picker" => state.send_picker = true,
            "--share" => state.share_mode = true,
            "--" if state.share_mode => state.literal_paths = true,
            "--device" => parse_device_arg(&mut iter, state)?,
            "--config" => parse_config_arg(&mut iter, state)?,
            other if state.share_mode && !other.starts_with('-') => {
                push_share_path(state, other)?;
            }
            other => {
                return Err(KeynearError::Validation {
                    field: "args",
                    message: format!("unknown argument '{other}'"),
                });
            }
        }
    }
    Ok(None)
}

fn validate_settings_args(state: &SettingsArgsState) -> Result<(), KeynearError> {
    if (state.validate_only && (state.send_picker || state.share_mode || state.device_id.is_some()))
        || (state.send_picker && state.share_mode)
    {
        return Err(KeynearError::Validation {
            field: "args",
            message: "conflicting CLI mode flags".to_string(),
        });
    }
    if state.device_id.is_some() && !state.send_picker && !state.share_mode {
        return Err(KeynearError::Validation {
            field: "deviceId",
            message: "--device requires --send-picker or --share".to_string(),
        });
    }
    Ok(())
}

fn build_settings_action(state: SettingsArgsState) -> Result<SettingsCliAction, KeynearError> {
    if state.validate_only {
        Ok(SettingsCliAction::Validate {
            config_path: state.config_path,
        })
    } else if state.send_picker {
        Ok(SettingsCliAction::SendPicker {
            config_path: state.config_path,
            device_id: state.device_id,
        })
    } else if state.share_mode {
        if state.share_paths.is_empty() {
            return Err(KeynearError::Validation {
                field: "paths",
                message: "--share requires at least one file or directory path".to_string(),
            });
        }
        Ok(SettingsCliAction::Share {
            config_path: state.config_path,
            device_id: state.device_id,
            paths: state.share_paths,
        })
    } else {
        Ok(SettingsCliAction::Run {
            config_path: state.config_path,
        })
    }
}

pub fn parse_settings_args<I, S>(args: I) -> Result<SettingsCliAction, KeynearError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut state = SettingsArgsState::default();
    if let Some(action) = consume_settings_args(args, &mut state)? {
        return Ok(action);
    }
    validate_settings_args(&state)?;
    build_settings_action(state)
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
        "min-height: var(--kn-size-touch-gtk)",
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
