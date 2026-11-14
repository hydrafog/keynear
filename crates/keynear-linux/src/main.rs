mod ble;
mod ipc;
mod notifications;
mod service;
mod tray;

use std::path::{Path, PathBuf};
use std::process;

use keynear_core::{
    default_config_path, sanitize_device_id, validate_display_name, DaemonConfig, KeynearError,
    PairedDevice, TransferDirection, TransferSession, TransferState, VisibilityMode,
};
use tracing_subscriber::EnvFilter;

use crate::ipc::{send_request_to, validate_transfer_id, DaemonStatus, IpcRequest, IpcResponse};

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

pub fn parse_cli_args<I, S>(args: I) -> Result<CliAction, KeynearError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut iter = args.into_iter().map(Into::into);
    let mut config_path: Option<PathBuf> = None;
    let mut pending_action: Option<CliAction> = None;

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--version" | "-V" => return Ok(CliAction::Version),
            "--help" | "-h" => return Ok(CliAction::Help),
            "--config" => {
                if pending_action.is_some() {
                    return Err(KeynearError::Validation {
                        field: "args",
                        message: "--config cannot be combined with transfer commands".to_string(),
                    });
                }
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
            "--transfers" => {
                ensure_no_prior_action(&pending_action, config_path.is_some())?;
                pending_action = Some(CliAction::Transfers);
            }
            "--status" => {
                ensure_no_prior_action(&pending_action, config_path.is_some())?;
                pending_action = Some(CliAction::Status);
            }
            "--devices" => {
                ensure_no_prior_action(&pending_action, config_path.is_some())?;
                pending_action = Some(CliAction::Devices);
            }
            "--set-visibility" => {
                ensure_no_prior_action(&pending_action, config_path.is_some())?;
                let value = iter.next().ok_or_else(|| KeynearError::Validation {
                    field: "visibilityMode",
                    message: "--set-visibility requires hidden, paired, or everyone".to_string(),
                })?;
                let mode = parse_visibility_arg(&value)?;
                pending_action = Some(CliAction::SetVisibility { mode });
            }
            "--rename-device" => {
                ensure_no_prior_action(&pending_action, config_path.is_some())?;
                let raw_device = iter.next().ok_or_else(|| KeynearError::Validation {
                    field: "deviceId",
                    message: "--rename-device requires <device_id> <display_name>".to_string(),
                })?;
                let device_id = sanitize_device_id(&raw_device)?;
                let raw_name = iter.next().ok_or_else(|| KeynearError::Validation {
                    field: "displayName",
                    message: "--rename-device requires <device_id> <display_name>".to_string(),
                })?;
                let display_name = validate_display_name(&raw_name)?;
                pending_action = Some(CliAction::RenameDevice {
                    device_id,
                    display_name,
                });
            }
            "--accept" => {
                ensure_no_prior_action(&pending_action, config_path.is_some())?;
                let value = iter.next().ok_or_else(|| KeynearError::Validation {
                    field: "transferId",
                    message: "--accept requires a transfer_id".to_string(),
                })?;
                let clean = validate_transfer_id(&value)?.to_string();
                pending_action = Some(CliAction::Accept { transfer_id: clean });
            }
            "--decline" => {
                ensure_no_prior_action(&pending_action, config_path.is_some())?;
                let value = iter.next().ok_or_else(|| KeynearError::Validation {
                    field: "transferId",
                    message: "--decline requires a transfer_id".to_string(),
                })?;
                let clean = validate_transfer_id(&value)?.to_string();
                pending_action = Some(CliAction::Decline { transfer_id: clean });
            }
            "--cancel" => {
                ensure_no_prior_action(&pending_action, config_path.is_some())?;
                let value = iter.next().ok_or_else(|| KeynearError::Validation {
                    field: "transferId",
                    message: "--cancel requires a transfer_id".to_string(),
                })?;
                let clean = validate_transfer_id(&value)?.to_string();
                pending_action = Some(CliAction::Cancel { transfer_id: clean });
            }
            "--send" => {
                ensure_no_prior_action(&pending_action, config_path.is_some())?;
                let raw_device = iter.next().ok_or_else(|| KeynearError::Validation {
                    field: "deviceId",
                    message: "--send requires <device_id> <path>...".to_string(),
                })?;
                let device_id = sanitize_device_id(&raw_device)?;
                let mut paths = Vec::new();
                let mut literal_paths = false;
                for operand in iter.by_ref() {
                    if !literal_paths && operand == "--" {
                        literal_paths = true;
                        continue;
                    }
                    if !literal_paths && operand.starts_with('-') {
                        return Err(KeynearError::Validation {
                            field: "args",
                            message: format!(
                                "unexpected flag '{operand}' after --send (use '--' before paths starting with '-')"
                            ),
                        });
                    }
                    if operand.trim().is_empty() {
                        return Err(KeynearError::Validation {
                            field: "paths",
                            message: "--send paths must not be empty".to_string(),
                        });
                    }
                    paths.push(PathBuf::from(operand));
                }
                if paths.is_empty() {
                    return Err(KeynearError::Validation {
                        field: "paths",
                        message: "--send requires at least one path".to_string(),
                    });
                }
                pending_action = Some(CliAction::Send { device_id, paths });
            }
            other => {
                return Err(KeynearError::Validation {
                    field: "args",
                    message: format!("unknown argument '{other}'"),
                });
            }
        }
    }

    Ok(pending_action.unwrap_or(CliAction::Run { config_path }))
}

fn ensure_no_prior_action(
    pending: &Option<CliAction>,
    has_config: bool,
) -> Result<(), KeynearError> {
    if pending.is_some() || has_config {
        return Err(KeynearError::Validation {
            field: "args",
            message: "conflicting CLI commands specified".to_string(),
        });
    }
    Ok(())
}

fn parse_visibility_arg(raw: &str) -> Result<VisibilityMode, KeynearError> {
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

pub fn cli_action_to_ipc_request(action: &CliAction) -> Option<IpcRequest> {
    match action {
        CliAction::Send { device_id, paths } => Some(IpcRequest::SendFiles {
            device_id: device_id.clone(),
            paths: paths.clone(),
        }),
        CliAction::Accept { transfer_id } => Some(IpcRequest::AcceptTransfer {
            transfer_id: transfer_id.clone(),
        }),
        CliAction::Decline { transfer_id } => Some(IpcRequest::DeclineTransfer {
            transfer_id: transfer_id.clone(),
        }),
        CliAction::Cancel { transfer_id } => Some(IpcRequest::CancelTransfer {
            transfer_id: transfer_id.clone(),
        }),
        CliAction::Transfers => Some(IpcRequest::ListTransfers),
        CliAction::Status => Some(IpcRequest::Status),
        CliAction::Devices => Some(IpcRequest::ListDevices),
        CliAction::SetVisibility { mode } => Some(IpcRequest::SetVisibility { mode: *mode }),
        CliAction::RenameDevice {
            device_id,
            display_name,
        } => Some(IpcRequest::RenameDevice {
            device_id: device_id.clone(),
            display_name: display_name.clone(),
        }),
        CliAction::Version | CliAction::Help | CliAction::Run { .. } => None,
    }
}

fn format_visibility_mode(mode: VisibilityMode) -> &'static str {
    match mode {
        VisibilityMode::Hidden => "hidden",
        VisibilityMode::PairedOnly => "paired",
        VisibilityMode::EveryoneNearby => "everyone",
    }
}

pub fn format_device_summary(device: &PairedDevice) -> String {
    format!(
        "{}\t{}\t{:?}\tauto_accept={}\t{}",
        device.device_id,
        device.display_name,
        device.device_role,
        device.auto_accept_files,
        device.last_lan_address
    )
}

pub fn format_status_summary(status: &DaemonStatus) -> String {
    let remaining = status
        .everyone_remaining_secs
        .map(|secs| format!("{secs}s"))
        .unwrap_or_else(|| "none".to_string());
    let addrs = if status.listen_addrs.is_empty() {
        "none".to_string()
    } else {
        status.listen_addrs.join(",")
    };
    format!(
        "device_id={}\nendpoint_id={}\nvisibility={}\neveryone_remaining={}\nauto_accept_from_paired={}\nincoming_directory={}\nactive_transfers={}\npaired_devices={}\nmdns_degraded={}\nlisten_addrs={}",
        status.device_id,
        status.endpoint_id,
        format_visibility_mode(status.visibility_mode),
        remaining,
        status.auto_accept_from_paired,
        status.incoming_directory,
        status.active_transfer_count,
        status.paired_device_count,
        status.mdns_degraded,
        addrs
    )
}

pub fn format_config_summary(config: &DaemonConfig) -> String {
    format!(
        "visibility={}\nauto_accept_from_paired={}\nincoming_directory={}\nnotify_auto_accept_complete={}",
        format_visibility_mode(config.visibility_mode),
        config.auto_accept_from_paired,
        config.incoming_directory,
        config.notify_auto_accept_complete
    )
}

pub fn format_transfer_summary(session: &TransferSession) -> String {
    let dir = match session.direction {
        TransferDirection::Send => "send",
        TransferDirection::Receive => "receive",
    };
    let state = match session.state {
        TransferState::PendingConsent => "pending_consent",
        TransferState::Active => "active",
        TransferState::Completed => "completed",
        TransferState::Cancelled => "cancelled",
        TransferState::Failed => "failed",
    };
    let items = if session.items.is_empty() {
        "transfer".to_string()
    } else if session.items.len() == 1 {
        session.items[0].name.clone()
    } else {
        format!(
            "{} (+{} more)",
            session.items[0].name,
            session.items.len() - 1
        )
    };
    format!(
        "{}\t{}\t{}\t{}\t{}/{}\t{}",
        session.transfer_id,
        dir,
        session.peer_device_id,
        state,
        session.transferred_bytes,
        session.total_bytes,
        items
    )
}

pub fn run_ipc_command(socket_path: &Path, request: &IpcRequest) -> Result<String, KeynearError> {
    match send_request_to(socket_path, request)? {
        IpcResponse::Transfer(session) => Ok(format_transfer_summary(&session)),
        IpcResponse::Transfers(sessions) => {
            if sessions.is_empty() {
                Ok("No active transfers".to_string())
            } else {
                let lines: Vec<String> = sessions.iter().map(format_transfer_summary).collect();
                Ok(lines.join("\n"))
            }
        }
        IpcResponse::Status(status) => Ok(format_status_summary(&status)),
        IpcResponse::Devices(devices) => {
            if devices.is_empty() {
                Ok("No paired devices".to_string())
            } else {
                let lines: Vec<String> = devices.iter().map(format_device_summary).collect();
                Ok(lines.join("\n"))
            }
        }
        IpcResponse::Device(device) => Ok(format_device_summary(&device)),
        IpcResponse::Config(config) => Ok(format_config_summary(&config)),
        IpcResponse::Error { message } => Err(KeynearError::Transfer(message)),
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let action = match parse_cli_args(std::env::args().skip(1)) {
        Ok(action) => action,
        Err(err) => {
            eprintln!("error: {err}");
            process::exit(2);
        }
    };

    if let Some(request) = cli_action_to_ipc_request(&action) {
        let sock = ipc::daemon_socket_path();
        let output =
            match tokio::task::spawn_blocking(move || run_ipc_command(&sock, &request)).await {
                Ok(Ok(text)) => text,
                Ok(Err(err)) => {
                    eprintln!("error: {err}");
                    process::exit(1);
                }
                Err(err) => {
                    eprintln!("error: command task failed: {err}");
                    process::exit(1);
                }
            };
        println!("{output}");
        return;
    }

    match action {
        CliAction::Version => {
            println!("keynear-linux {}", env!("CARGO_PKG_VERSION"));
        }
        CliAction::Help => {
            println!(
                "Usage: keynear-linux [--config <path>] [--version] [--help]\n       keynear-linux --send <device_id> <path>...\n       keynear-linux --accept <transfer_id>\n       keynear-linux --decline <transfer_id>\n       keynear-linux --cancel <transfer_id>\n       keynear-linux --transfers\n       keynear-linux --status\n       keynear-linux --devices\n       keynear-linux --set-visibility <hidden|paired|everyone>\n       keynear-linux --rename-device <device_id> <display_name>\n\nOptions:\n  --config <path>                          Path to TOML daemon configuration\n  --send <device_id> <path>...             Send files or directories via running daemon\n  --accept <transfer_id>                   Accept an incoming transfer awaiting consent\n  --decline <transfer_id>                  Decline an incoming transfer awaiting consent\n  --cancel <transfer_id>                   Cancel an active or pending transfer\n  --transfers                              List known transfers from running daemon\n  --status                                 Print daemon status and visibility timer\n  --devices                                List paired devices from running daemon\n  --set-visibility <mode>                  Set daemon visibility (hidden|paired|everyone)\n  --rename-device <device_id> <name>       Rename a paired device\n  --version, -V                            Print version information\n  --help, -h                               Print this help message"
            );
        }
        CliAction::Run { config_path } => {
            let config = match load_daemon_config(config_path.as_ref()) {
                Ok(config) => config,
                Err(err) => {
                    eprintln!("error: {err}");
                    process::exit(1);
                }
            };
            if let Err(err) =
                service::DaemonRuntime::run_until_shutdown_with_config_path(&config, config_path)
                    .await
            {
                eprintln!("error: {err}");
                process::exit(1);
            }
        }
        CliAction::Send { .. }
        | CliAction::Accept { .. }
        | CliAction::Decline { .. }
        | CliAction::Cancel { .. }
        | CliAction::Transfers
        | CliAction::Status
        | CliAction::Devices
        | CliAction::SetVisibility { .. }
        | CliAction::RenameDevice { .. } => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::{Read, Write};
    use std::os::unix::net::UnixListener;
    use std::thread;
    use std::time::{SystemTime, UNIX_EPOCH};

    use keynear_core::TransferItem;

    use super::*;
    use crate::ipc::{
        decode_request, encode_response, prepare_daemon_socket_dir, set_daemon_socket_permissions,
    };

    #[test]
    fn cli_version_and_help_parsed() {
        assert_eq!(parse_cli_args(["--version"]).unwrap(), CliAction::Version);
        assert_eq!(parse_cli_args(["-h"]).unwrap(), CliAction::Help);
    }

    #[test]
    fn cli_config_path_parsed() {
        assert_eq!(
            parse_cli_args(["--config", "/tmp/keynear.toml"]).unwrap(),
            CliAction::Run {
                config_path: Some(PathBuf::from("/tmp/keynear.toml"))
            }
        );
    }

    #[test]
    fn cli_transfer_commands_parsed() {
        assert_eq!(
            parse_cli_args(["--send", "pixel-9", "/tmp/a.txt", "/tmp/dir"]).unwrap(),
            CliAction::Send {
                device_id: "pixel-9".to_string(),
                paths: vec![PathBuf::from("/tmp/a.txt"), PathBuf::from("/tmp/dir")],
            }
        );
        assert_eq!(
            parse_cli_args(["--accept", "tx-1"]).unwrap(),
            CliAction::Accept {
                transfer_id: "tx-1".to_string(),
            }
        );
        assert_eq!(
            parse_cli_args(["--decline", "tx-2"]).unwrap(),
            CliAction::Decline {
                transfer_id: "tx-2".to_string(),
            }
        );
        assert_eq!(
            parse_cli_args(["--cancel", "tx-3"]).unwrap(),
            CliAction::Cancel {
                transfer_id: "tx-3".to_string(),
            }
        );
        assert_eq!(
            parse_cli_args(["--transfers"]).unwrap(),
            CliAction::Transfers
        );
    }

    #[test]
    fn cli_rejects_invalid_transfer_flags_and_missing_values() {
        assert!(matches!(
            parse_cli_args(["--send", "pixel-9"]),
            Err(KeynearError::Validation { field: "paths", .. })
        ));
        assert!(matches!(
            parse_cli_args(["--send", "../evil", "/tmp/a.txt"]),
            Err(KeynearError::Validation {
                field: "deviceId",
                ..
            })
        ));
        assert!(matches!(
            parse_cli_args(["--accept"]),
            Err(KeynearError::Validation {
                field: "transferId",
                ..
            })
        ));
        assert!(matches!(
            parse_cli_args(["--transfers", "--accept", "tx-1"]),
            Err(KeynearError::Validation { field: "args", .. })
        ));
        assert!(matches!(
            parse_cli_args(["--config", "/tmp/a.toml", "--transfers"]),
            Err(KeynearError::Validation { field: "args", .. })
        ));
    }

    #[test]
    fn cli_missing_config_value_fails() {
        let err = parse_cli_args(["--config"]).expect_err("missing --config operand must fail");
        assert!(matches!(
            err,
            KeynearError::Validation {
                field: "config",
                ..
            }
        ));
    }

    #[test]
    fn cli_unknown_flag_fails() {
        let err = parse_cli_args(["--bogus"]).expect_err("unknown flag must fail");
        assert!(matches!(
            err,
            KeynearError::Validation { field: "args", .. }
        ));
    }

    #[test]
    fn explicit_missing_config_file_rejected() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let missing = std::env::temp_dir().join(format!("keynear-linux-missing-{nanos}.toml"));
        let err =
            load_daemon_config(Some(&missing)).expect_err("missing explicit config must fail");
        assert!(matches!(err, KeynearError::Config(_)));
    }

    #[test]
    fn cli_ipc_command_formats_transfer_output() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("keynear-cli-ipc-{nanos}"));
        let sock_path = dir.join("daemon.sock");
        prepare_daemon_socket_dir(&sock_path).expect("prepare");
        let listener = UnixListener::bind(&sock_path).expect("bind");
        set_daemon_socket_permissions(&sock_path).expect("perms");

        let server = thread::spawn(move || {
            let (mut conn, _) = listener.accept().expect("accept");
            let mut len_buf = [0u8; 4];
            conn.read_exact(&mut len_buf).expect("len");
            let len = u32::from_be_bytes(len_buf) as usize;
            let mut buf = vec![0u8; len];
            conn.read_exact(&mut buf).expect("body");
            let req = decode_request(&buf).expect("decode");
            assert_eq!(req, IpcRequest::ListTransfers);
            let session = TransferSession {
                transfer_id: "tx-77".to_string(),
                peer_device_id: "pixel-9".to_string(),
                direction: TransferDirection::Receive,
                items: vec![TransferItem {
                    name: "photo.jpg".to_string(),
                    mime_type: "image/jpeg".to_string(),
                    size_bytes: 2048,
                    is_directory: false,
                    target_path: None,
                    checksum: None,
                }],
                total_bytes: 2048,
                transferred_bytes: 1024,
                state: TransferState::PendingConsent,
                started_at: "1".to_string(),
            };
            let encoded = encode_response(&IpcResponse::Transfers(vec![session])).expect("encode");
            let rlen = (encoded.len() as u32).to_be_bytes();
            conn.write_all(&rlen).expect("wlen");
            conn.write_all(&encoded).expect("wbody");
        });

        let req = cli_action_to_ipc_request(&CliAction::Transfers).expect("ipc req");
        let output = run_ipc_command(&sock_path, &req).expect("run ipc command");
        assert!(output.contains("tx-77"));
        assert!(output.contains("pending_consent"));
        assert!(output.contains("1024/2048"));
        assert!(output.contains("photo.jpg"));
        server.join().expect("server");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn cli_aquila_status_visibility_and_device_flags_parsed() {
        assert_eq!(parse_cli_args(["--status"]).unwrap(), CliAction::Status);
        assert_eq!(parse_cli_args(["--devices"]).unwrap(), CliAction::Devices);
        assert_eq!(
            parse_cli_args(["--set-visibility", "everyone"]).unwrap(),
            CliAction::SetVisibility {
                mode: VisibilityMode::EveryoneNearby,
            }
        );
        assert_eq!(
            parse_cli_args(["--set-visibility", "paired"]).unwrap(),
            CliAction::SetVisibility {
                mode: VisibilityMode::PairedOnly,
            }
        );
        assert_eq!(
            parse_cli_args(["--rename-device", "pixel-9", "My Pixel"]).unwrap(),
            CliAction::RenameDevice {
                device_id: "pixel-9".to_string(),
                display_name: "My Pixel".to_string(),
            }
        );
        assert!(matches!(
            parse_cli_args(["--set-visibility", "public"]),
            Err(KeynearError::Validation {
                field: "visibilityMode",
                ..
            })
        ));
        assert!(matches!(
            parse_cli_args(["--rename-device", "../bad", "Phone"]),
            Err(KeynearError::Validation {
                field: "deviceId",
                ..
            })
        ));
        assert!(matches!(
            parse_cli_args(["--rename-device", "pixel-9", "   "]),
            Err(KeynearError::Validation {
                field: "displayName",
                ..
            })
        ));
    }

    #[test]
    fn cli_ipc_formats_status_and_devices() {
        let status = DaemonStatus {
            device_id: "host-linux".to_string(),
            endpoint_id: "ab".repeat(32),
            listen_addrs: vec!["127.0.0.1:4242".to_string()],
            visibility_mode: VisibilityMode::EveryoneNearby,
            mdns_degraded: false,
            active_transfer_count: 1,
            paired_device_count: 1,
            everyone_remaining_secs: Some(325),
            auto_accept_from_paired: true,
            incoming_directory: "/tmp/incoming".to_string(),
        };
        let rendered_status = format_status_summary(&status);
        assert!(rendered_status.contains("visibility=everyone"));
        assert!(rendered_status.contains("everyone_remaining=325s"));
        assert!(rendered_status.contains("auto_accept_from_paired=true"));

        let device = keynear_core::PairedDevice {
            device_id: "pixel-9".to_string(),
            display_name: "Pixel Desk".to_string(),
            device_role: keynear_core::DeviceRole::Mobile,
            static_x25519_pubkey: "cd".repeat(32),
            last_ble_address: String::new(),
            last_lan_address: "192.168.1.5:4242".to_string(),
            last_endpoint_id: "ef".repeat(32),
            capability_flags: 1,
            auto_accept_files: true,
            paired_at: "1".to_string(),
        };
        let rendered_device = format_device_summary(&device);
        assert!(rendered_device.contains("pixel-9"));
        assert!(rendered_device.contains("Pixel Desk"));
        assert!(rendered_device.contains("auto_accept=true"));
    }
}
