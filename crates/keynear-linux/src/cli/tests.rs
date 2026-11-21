use std::fs;
use std::io::{Read, Write};
use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use keynear_ipc::{
    decode_request, encode_response, prepare_daemon_socket_dir, set_daemon_socket_permissions,
    DaemonStatus, IpcRequest, IpcResponse,
};
use keynear_protocol::{
    DeviceRole, KeynearError, PairedDevice, TransferDirection, TransferItem, TransferSession,
    TransferState, VisibilityMode,
};

use super::format::{format_device_summary, format_status_summary};
use super::*;

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
}

#[test]
fn cli_rejects_conflicting_commands() {
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
    let err = load_daemon_config(Some(&missing)).expect_err("missing explicit config must fail");
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
}

#[test]
fn cli_aquila_invalid_flags_rejected() {
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

    let device = PairedDevice {
        device_id: "pixel-9".to_string(),
        display_name: "Pixel Desk".to_string(),
        device_role: DeviceRole::Mobile,
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
