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
    DaemonConfig, DeviceRole, KeynearError, PairedDevice, TransferDirection, TransferItem,
    TransferSession, TransferState, VisibilityMode,
};

use super::format::{format_device_summary, format_status_summary};
use super::*;

fn expect_validation_field(args: &[&str], expected_field: &'static str) {
    match parse_cli_args(args.iter().copied()) {
        Err(KeynearError::Validation { field, .. }) => assert_eq!(field, expected_field),
        other => {
            panic!("expected validation error on '{expected_field}' for {args:?}, got {other:?}")
        }
    }
}

#[test]
fn cli_subcommands_daemon_settings_and_help_parsed() {
    assert_eq!(
        parse_cli_args(["daemon"]).unwrap(),
        CliAction::Run { config_path: None }
    );
    assert_eq!(
        parse_cli_args(["daemon", "--config", "/tmp/k.toml"]).unwrap(),
        CliAction::Run {
            config_path: Some(PathBuf::from("/tmp/k.toml"))
        }
    );
    assert_eq!(
        parse_cli_args(["settings"]).unwrap(),
        CliAction::Settings {
            config_path: None,
            validate_only: false,
        }
    );
    assert_eq!(
        parse_cli_args(["settings", "validate", "--config", "/tmp/k.toml"]).unwrap(),
        CliAction::Settings {
            config_path: Some(PathBuf::from("/tmp/k.toml")),
            validate_only: true,
        }
    );
    assert_eq!(
        parse_cli_args(["--validate", "--config", "/tmp/a.toml"]).unwrap(),
        CliAction::Settings {
            config_path: Some(PathBuf::from("/tmp/a.toml")),
            validate_only: true,
        }
    );
    assert_eq!(parse_cli_args(["version"]).unwrap(), CliAction::Version);
    assert_eq!(parse_cli_args(["help"]).unwrap(), CliAction::Help);
    assert_eq!(parse_cli_args(["send", "--help"]).unwrap(), CliAction::Help);
}

#[test]
fn cli_subcommands_status_devices_transfers_visibility_and_rename_parsed() {
    assert_eq!(parse_cli_args(["status"]).unwrap(), CliAction::Status);
    assert_eq!(parse_cli_args(["devices"]).unwrap(), CliAction::Devices);
    assert_eq!(parse_cli_args(["transfers"]).unwrap(), CliAction::Transfers);
    assert_eq!(
        parse_cli_args(["accept", "tx-1"]).unwrap(),
        CliAction::Accept {
            transfer_id: "tx-1".into()
        }
    );
    assert_eq!(
        parse_cli_args(["decline", "tx-2"]).unwrap(),
        CliAction::Decline {
            transfer_id: "tx-2".into()
        }
    );
    assert_eq!(
        parse_cli_args(["cancel", "tx-3"]).unwrap(),
        CliAction::Cancel {
            transfer_id: "tx-3".into()
        }
    );
    assert_eq!(
        parse_cli_args(["visibility", "everyone"]).unwrap(),
        CliAction::SetVisibility {
            mode: VisibilityMode::EveryoneNearby
        }
    );
    assert_eq!(
        parse_cli_args(["rename", "pixel-9", "My Pixel"]).unwrap(),
        CliAction::RenameDevice {
            device_id: "pixel-9".into(),
            display_name: "My Pixel".into()
        }
    );
}

#[test]
fn cli_pair_subcommands_parsed() {
    assert_eq!(
        parse_cli_args(["pair", "approve"]).unwrap(),
        CliAction::RespondPairing { approve: true }
    );
    assert_eq!(
        parse_cli_args(["pair", "reject"]).unwrap(),
        CliAction::RespondPairing { approve: false }
    );
    assert_eq!(parse_cli_args(["pair", "--help"]).unwrap(), CliAction::Help);
    assert_eq!(
        cli_action_to_ipc_request(&CliAction::RespondPairing { approve: true }),
        Some(IpcRequest::RespondPairing { approve: true })
    );
}

#[test]
fn cli_send_subcommands_and_legacy_flags_parsed() {
    assert_eq!(
        parse_cli_args(["send", "--picker", "--device", "pixel-9"]).unwrap(),
        CliAction::SendPicker {
            device_id: Some("pixel-9".into())
        }
    );
    assert_eq!(
        parse_cli_args(["send", "--", "-report.pdf", "/tmp/my dir"]).unwrap(),
        CliAction::Send {
            device_id: None,
            paths: vec![PathBuf::from("-report.pdf"), PathBuf::from("/tmp/my dir")],
        }
    );
    assert_eq!(
        parse_cli_args(["send", "pixel-9", "/tmp/a.txt"]).unwrap(),
        CliAction::Send {
            device_id: Some("pixel-9".into()),
            paths: vec![PathBuf::from("/tmp/a.txt")],
        }
    );
    assert_eq!(
        parse_cli_args(["--share", "--", "-report.pdf"]).unwrap(),
        CliAction::Send {
            device_id: None,
            paths: vec![PathBuf::from("-report.pdf")],
        }
    );
    assert_eq!(
        parse_cli_args(["--send", "pixel-9", "/tmp/a.txt"]).unwrap(),
        CliAction::Send {
            device_id: Some("pixel-9".into()),
            paths: vec![PathBuf::from("/tmp/a.txt")],
        }
    );
    assert_eq!(
        parse_cli_args(["--send-picker", "--device", "pixel-9"]).unwrap(),
        CliAction::SendPicker {
            device_id: Some("pixel-9".into())
        }
    );
}

#[test]
fn cli_rejects_invalid_and_conflicting_arguments() {
    expect_validation_field(&["send", "pixel-9"], "paths");
    expect_validation_field(&["--send", "../evil", "/tmp/a.txt"], "deviceId");
    expect_validation_field(&["send", "--picker", "--", "/tmp/a.txt"], "args");
    expect_validation_field(&["--transfers", "--accept", "tx-1"], "args");
    expect_validation_field(&["--config", "/tmp/a.toml", "--transfers"], "args");
    expect_validation_field(&["--config"], "config");
    expect_validation_field(&["--bogus"], "args");
    expect_validation_field(&["visibility", "public"], "visibilityMode");
    expect_validation_field(&["pair", "maybe"], "pairingDecision");
}

#[test]
fn settings_validate_and_show_format_output() {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("keynear-cli-settings-{nanos}"));
    fs::create_dir_all(&dir).expect("mkdir");
    let cfg_path = dir.join("config.toml");
    let incoming = dir.join("incoming");
    fs::create_dir_all(&incoming).expect("incoming");
    let cfg = DaemonConfig {
        incoming_directory: incoming.display().to_string(),
        visibility_mode: VisibilityMode::Hidden,
        ..Default::default()
    };
    cfg.save_to_path(&cfg_path).expect("save cfg");

    assert!(matches!(
        load_daemon_config(Some(&dir.join("missing.toml"))),
        Err(KeynearError::Config(_))
    ));
    let validated =
        run_settings_command(&dir.join("absent.sock"), Some(&cfg_path), true).expect("validate");
    assert!(validated.contains("config_valid=true") && validated.contains("visibility=hidden"));

    let shown =
        run_settings_command(&dir.join("absent.sock"), Some(&cfg_path), false).expect("show");
    assert!(shown.contains(&format!("config_path={}", cfg_path.display())));
    assert!(shown.contains("visibility=hidden"));
    let _ = fs::remove_dir_all(&dir);
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
        let mut buf = vec![0u8; u32::from_be_bytes(len_buf) as usize];
        conn.read_exact(&mut buf).expect("body");
        assert_eq!(
            decode_request(&buf).expect("decode"),
            IpcRequest::ListTransfers
        );
        let session = TransferSession {
            transfer_id: "tx-77".into(),
            peer_device_id: "pixel-9".into(),
            direction: TransferDirection::Receive,
            items: vec![TransferItem {
                name: "photo.jpg".into(),
                mime_type: "image/jpeg".into(),
                size_bytes: 2048,
                is_directory: false,
                target_path: None,
                checksum: None,
            }],
            total_bytes: 2048,
            transferred_bytes: 1024,
            state: TransferState::PendingConsent,
            started_at: "1".into(),
        };
        let encoded = encode_response(&IpcResponse::Transfers(vec![session])).expect("encode");
        conn.write_all(&(encoded.len() as u32).to_be_bytes())
            .expect("wlen");
        conn.write_all(&encoded).expect("wbody");
    });

    let req = cli_action_to_ipc_request(&CliAction::Transfers).expect("ipc req");
    let output = run_ipc_command(&sock_path, &req).expect("run ipc command");
    assert!(output.contains("tx-77") && output.contains("pending_consent"));
    server.join().expect("server");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn cli_formats_status_and_devices() {
    let status = DaemonStatus {
        device_id: "host-linux".into(),
        endpoint_id: "ab".repeat(32),
        listen_addrs: vec!["127.0.0.1:4242".into()],
        visibility_mode: VisibilityMode::EveryoneNearby,
        mdns_degraded: false,
        active_transfer_count: 1,
        paired_device_count: 1,
        everyone_remaining_secs: Some(325),
        auto_accept_from_paired: true,
        incoming_directory: "/tmp/incoming".into(),
    };
    assert!(format_status_summary(&status).contains("everyone_remaining=325s"));
    let device = PairedDevice {
        device_id: "pixel-9".into(),
        display_name: "Pixel Desk".into(),
        device_role: DeviceRole::Mobile,
        static_x25519_pubkey: "cd".repeat(32),
        last_ble_address: String::new(),
        last_lan_address: "192.168.1.5:4242".into(),
        last_endpoint_id: "ef".repeat(32),
        capability_flags: 1,
        auto_accept_files: true,
        paired_at: "1".into(),
    };
    assert!(format_device_summary(&device).contains("Pixel Desk"));
}
