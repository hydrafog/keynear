mod format;
mod parse;

#[cfg(test)]
mod tests;

pub use format::{cli_action_to_ipc_request, run_ipc_command};
pub use parse::{load_daemon_config, parse_cli_args};

use std::path::PathBuf;

use keynear_protocol::VisibilityMode;

pub const CLI_USAGE_TEXT: &str = "Usage: keynear-linux [--config <path>] [--version] [--help]
       keynear-linux --send <device_id> <path>...
       keynear-linux --accept <transfer_id>
       keynear-linux --decline <transfer_id>
       keynear-linux --cancel <transfer_id>
       keynear-linux --transfers
       keynear-linux --status
       keynear-linux --devices
       keynear-linux --set-visibility <hidden|paired|everyone>
       keynear-linux --rename-device <device_id> <display_name>

Options:
  --config <path>                          Path to TOML daemon configuration
  --send <device_id> <path>...             Send files or directories via running daemon
  --accept <transfer_id>                   Accept an incoming transfer awaiting consent
  --decline <transfer_id>                  Decline an incoming transfer awaiting consent
  --cancel <transfer_id>                   Cancel an active or pending transfer
  --transfers                              List known transfers from running daemon
  --status                                 Print daemon status and visibility timer
  --devices                                List paired devices from running daemon
  --set-visibility <mode>                  Set daemon visibility (hidden|paired|everyone)
  --rename-device <device_id> <name>       Rename a paired device
  --version, -V                            Print version information
  --help, -h                               Print this help message";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliAction {
    Version,
    Help,
    Run {
        config_path: Option<PathBuf>,
    },
    Send {
        device_id: String,
        paths: Vec<PathBuf>,
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
    RenameDevice {
        device_id: String,
        display_name: String,
    },
}
