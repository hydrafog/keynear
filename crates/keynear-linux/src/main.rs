mod ble;
mod ipc;
mod service;

use std::path::{Path, PathBuf};
use std::process;

use keynear_core::{
    default_config_path, sanitize_device_id, DaemonConfig, KeynearError, TransferDirection,
    TransferSession, TransferState,
};
use tracing_subscriber::EnvFilter;

use crate::ipc::{send_request_to, validate_transfer_id, IpcRequest, IpcResponse};

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
        CliAction::Version | CliAction::Help | CliAction::Run { .. } => None,
    }
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
        IpcResponse::Error { message } => Err(KeynearError::Transfer(message)),
        IpcResponse::Status(_) => Err(KeynearError::Protocol(
            "unexpected status response for transfer command".to_string(),
        )),
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
                "Usage: keynear-linux [--config <path>] [--version] [--help]\n       keynear-linux --send <device_id> <path>...\n       keynear-linux --accept <transfer_id>\n       keynear-linux --decline <transfer_id>\n       keynear-linux --cancel <transfer_id>\n       keynear-linux --transfers\n\nOptions:\n  --config <path>              Path to TOML daemon configuration\n  --send <device_id> <path>... Send files or directories via running daemon\n  --accept <transfer_id>       Accept an incoming transfer awaiting consent\n  --decline <transfer_id>      Decline an incoming transfer awaiting consent\n  --cancel <transfer_id>       Cancel an active or pending transfer\n  --transfers                  List known transfers from running daemon\n  --version, -V                Print version information\n  --help, -h                   Print this help message"
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
            if let Err(err) = service::DaemonRuntime::run_until_shutdown(&config).await {
                eprintln!("error: {err}");
                process::exit(1);
            }
        }
        CliAction::Send { .. }
        | CliAction::Accept { .. }
        | CliAction::Decline { .. }
        | CliAction::Cancel { .. }
        | CliAction::Transfers => unreachable!(),
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
}
