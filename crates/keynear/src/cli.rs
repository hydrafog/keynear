mod format;
mod parse;
mod send;

#[cfg(test)]
mod tests;

pub use format::{cli_action_to_ipc_request, run_ipc_command, run_settings_command};
pub use parse::parse_cli_args;

use std::path::PathBuf;

use keynear_protocol::{
    default_config_dir, default_config_path, DaemonConfig, KeynearError, VisibilityMode,
};

pub(super) fn data_scoped_config_path() -> Option<PathBuf> {
    let raw = std::env::var("KEYNEAR_DATA_DIR").ok()?;
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.contains('\0') {
        return None;
    }
    let dir = PathBuf::from(trimmed);
    (dir != default_config_dir()).then(|| dir.join("config.toml"))
}

pub(super) fn validation_err(field: &'static str, message: String) -> KeynearError {
    KeynearError::Validation { field, message }
}

pub(super) fn parse_config_path(
    iter: &mut impl Iterator<Item = String>,
) -> Result<PathBuf, KeynearError> {
    let value = iter.next().ok_or_else(|| KeynearError::Validation {
        field: "config",
        message: "--config requires a file path".into(),
    })?;
    if value.trim().is_empty() {
        return Err(KeynearError::Validation {
            field: "config",
            message: "--config requires a non-empty file path".into(),
        });
    }
    Ok(PathBuf::from(value))
}

pub(super) fn parse_visibility_arg(raw: &str) -> Result<VisibilityMode, KeynearError> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "hidden" => Ok(VisibilityMode::Hidden),
        "paired" | "paired_only" | "paired-only" => Ok(VisibilityMode::PairedOnly),
        "everyone" | "everyone_nearby" | "everyone-nearby" => Ok(VisibilityMode::EveryoneNearby),
        _ => Err(KeynearError::Validation {
            field: "visibilityMode",
            message: format!(
                "unknown visibility mode '{raw}' (expected hidden, paired, or everyone)"
            ),
        }),
    }
}

pub(super) fn parse_pair_arg(raw: &str) -> Result<bool, KeynearError> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "approve" => Ok(true),
        "reject" => Ok(false),
        _ => Err(KeynearError::Validation {
            field: "pairingDecision",
            message: format!("unknown pairing decision '{raw}' (expected approve or reject)"),
        }),
    }
}

pub fn load_daemon_config(explicit_path: Option<&PathBuf>) -> Result<DaemonConfig, KeynearError> {
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

pub const CLI_USAGE_TEXT: &str = "Usage: keynear [daemon] [--config <path>]
       keynear settings [show|validate] [--config <path>]
       keynear status
       keynear devices
       keynear transfers
       keynear send [--device <device_id>] [--] <path>...
       keynear send <device_id> [--] <path>...
       keynear send --picker [--folder] [--device <device_id>]
       keynear accept <transfer_id>
       keynear decline <transfer_id>
       keynear cancel <transfer_id>
       keynear visibility <hidden|paired|everyone>
       keynear pair <approve|reject>
       keynear rename <device_id> <display_name>
       keynear version | help

Subcommands:
  daemon [--config <path>]                 Run the Keynear daemon and tray (default)
  settings [show|validate] [--config <path>]
                                           Print or validate daemon configuration
  status                                   Print daemon status and visibility timer
  devices                                  List paired devices from running daemon
  transfers                                List known transfers from running daemon
  send [--device <id>] [--] <path>...      Send files or folders via running daemon
  send --picker [--folder] [--device <id>] Pick files or folders via desktop portal and send
  accept <transfer_id>                     Accept an incoming transfer awaiting consent
  decline <transfer_id>                    Decline an incoming transfer awaiting consent
  cancel <transfer_id>                     Cancel an active or pending transfer
  visibility <hidden|paired|everyone>      Set daemon visibility mode
  pair <approve|reject>                    Approve or reject a pending BLE pairing request
  rename <device_id> <name>                Rename a paired device
  version, --version, -V                   Print version information
  help, --help, -h                         Print this help message";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliAction {
    Version,
    Help,
    Run {
        config_path: Option<PathBuf>,
    },
    Settings {
        config_path: Option<PathBuf>,
        validate_only: bool,
    },
    Send {
        device_id: Option<String>,
        paths: Vec<PathBuf>,
    },
    SendPicker {
        device_id: Option<String>,
        directory: bool,
    },
    Accept {
        transfer_id: String,
    },
    Decline {
        transfer_id: String,
    },
    Cancel {
        transfer_id: String,
    },
    Transfers,
    Status,
    Devices,
    SetVisibility {
        mode: VisibilityMode,
    },
    RespondPairing {
        approve: bool,
    },
    RenameDevice {
        device_id: String,
        display_name: String,
    },
}
