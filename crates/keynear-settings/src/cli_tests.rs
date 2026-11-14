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
    let err = parse_settings_args(["--config"]).expect_err("missing --config operand must fail");
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
    assert!(app::STYLE_CSS.contains(parsed["light"]["switchTrackOff"].as_str().unwrap()));
    assert!(app::STYLE_CSS.contains(parsed["dark"]["switchTrackOff"].as_str().unwrap()));
    assert!(app::STYLE_CSS.contains(parsed["light"]["surfaceTonal"].as_str().unwrap()));
    assert!(app::STYLE_CSS.contains(parsed["dark"]["surfaceTonal"].as_str().unwrap()));
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
        config_path: None,
        everyone_remaining_secs: None,
        error_message: None,
        warning_message: None,
        transfer_error: None,
        paired_devices: Vec::new(),
        nearby_peers: Vec::new(),
        transfers: Vec::new(),
        send_paths: Vec::new(),
        selected_send_device_id: None,
        send_picker_active: false,
        device_store,
        secret_store: None,
        ipc_socket_path: None,
    });

    assert_eq!(model.visibility_mode(), VisibilityMode::PairedOnly);
    model.apply_msg(app::SettingsMsg::SetVisibility(
        VisibilityMode::EveryoneNearby,
    ));
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
        last_endpoint_id: String::new(),
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
        config_path: None,
        everyone_remaining_secs: None,
        error_message: None,
        warning_message: None,
        transfer_error: None,
        paired_devices: vec![dev],
        nearby_peers: Vec::new(),
        transfers: Vec::new(),
        send_paths: Vec::new(),
        selected_send_device_id: None,
        send_picker_active: false,
        device_store: device_store.clone(),
        secret_store: Some(Arc::clone(&secret_store) as Arc<dyn SecretStore>),
        ipc_socket_path: None,
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

#[test]
fn auto_accept_toggle_persists_to_device_store() {
    let (dir, device_store) = temp_store("auto-accept");
    let dev = PairedDevice {
        device_id: "pixel-9".to_string(),
        display_name: "Pixel 9".to_string(),
        device_role: DeviceRole::Mobile,
        static_x25519_pubkey: "ab".repeat(32),
        last_ble_address: String::new(),
        last_lan_address: String::new(),
        last_endpoint_id: String::new(),
        capability_flags: 0x03,
        auto_accept_files: false,
        paired_at: "1".to_string(),
    };
    device_store.upsert(dev.clone()).expect("seed device");

    let mut model = app::SettingsModel::from_init(app::SettingsInit {
        config: DaemonConfig::default(),
        config_path: None,
        everyone_remaining_secs: None,
        error_message: None,
        warning_message: None,
        transfer_error: None,
        paired_devices: vec![dev],
        nearby_peers: Vec::new(),
        transfers: Vec::new(),
        send_paths: Vec::new(),
        selected_send_device_id: None,
        send_picker_active: false,
        device_store: device_store.clone(),
        secret_store: None,
        ipc_socket_path: None,
    });

    model.apply_msg(app::SettingsMsg::SetAutoAcceptFiles {
        device_id: "pixel-9".to_string(),
        enabled: true,
    });
    assert!(model.paired_devices()[0].auto_accept_files);
    assert!(device_store.load().expect("load")[0].auto_accept_files);

    model.apply_msg(app::SettingsMsg::SetAutoAcceptFiles {
        device_id: "pixel-9".to_string(),
        enabled: false,
    });
    assert!(!model.paired_devices()[0].auto_accept_files);
    assert!(!device_store.load().expect("load")[0].auto_accept_files);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn active_transfers_accept_decline_cancel_communicate_over_ipc() {
    use keynear_core::{TransferDirection, TransferItem, TransferSession, TransferState};
    use std::io::{Read, Write};
    use std::os::unix::net::UnixListener;

    let (dir, device_store) = temp_store("transfers-ipc");
    let sock_path = dir.join("daemon.sock");
    ipc::prepare_daemon_socket_dir(&sock_path).expect("prepare sock dir");
    let listener = UnixListener::bind(&sock_path).expect("bind test sock");
    ipc::set_daemon_socket_permissions(&sock_path).expect("chmod test sock");

    let initial_sessions = vec![
        TransferSession {
            transfer_id: "tx-1".to_string(),
            peer_device_id: "pixel-9".to_string(),
            direction: TransferDirection::Receive,
            items: vec![TransferItem {
                name: "photo.jpg".to_string(),
                mime_type: "image/jpeg".to_string(),
                size_bytes: 2048,
                is_directory: false,
                target_path: None,
                checksum: Some("aa".repeat(32)),
            }],
            total_bytes: 2048,
            transferred_bytes: 0,
            state: TransferState::PendingConsent,
            started_at: "1".to_string(),
        },
        TransferSession {
            transfer_id: "tx-2".to_string(),
            peer_device_id: "pixel-9".to_string(),
            direction: TransferDirection::Receive,
            items: vec![TransferItem {
                name: "notes.txt".to_string(),
                mime_type: "text/plain".to_string(),
                size_bytes: 512,
                is_directory: false,
                target_path: None,
                checksum: Some("bb".repeat(32)),
            }],
            total_bytes: 512,
            transferred_bytes: 0,
            state: TransferState::PendingConsent,
            started_at: "2".to_string(),
        },
    ];

    let mut server_sessions = initial_sessions.clone();
    let handle = std::thread::spawn(move || {
        for _ in 0..3 {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut len_buf = [0u8; 4];
            stream.read_exact(&mut len_buf).expect("read len");
            let len = u32::from_be_bytes(len_buf) as usize;
            let mut payload = vec![0u8; len];
            stream.read_exact(&mut payload).expect("read payload");
            let req = ipc::decode_request(&payload).expect("decode req");

            let resp = match req {
                ipc::IpcRequest::AcceptTransfer { transfer_id } => {
                    let updated = server_sessions
                        .iter_mut()
                        .find(|s| s.transfer_id == transfer_id)
                        .map(|s| {
                            s.state = TransferState::Active;
                            s.clone()
                        });
                    match updated {
                        Some(s) => ipc::IpcResponse::Transfer(s),
                        None => ipc::IpcResponse::Error {
                            message: "not found".to_string(),
                        },
                    }
                }
                ipc::IpcRequest::CancelTransfer { transfer_id } => {
                    let updated = server_sessions
                        .iter_mut()
                        .find(|s| s.transfer_id == transfer_id)
                        .map(|s| {
                            s.state = TransferState::Cancelled;
                            s.clone()
                        });
                    match updated {
                        Some(s) => ipc::IpcResponse::Transfer(s),
                        None => ipc::IpcResponse::Error {
                            message: "not found".to_string(),
                        },
                    }
                }
                ipc::IpcRequest::DeclineTransfer { transfer_id } => {
                    let updated = server_sessions
                        .iter_mut()
                        .find(|s| s.transfer_id == transfer_id)
                        .map(|s| {
                            s.state = TransferState::Cancelled;
                            s.clone()
                        });
                    match updated {
                        Some(s) => ipc::IpcResponse::Transfer(s),
                        None => ipc::IpcResponse::Error {
                            message: "not found".to_string(),
                        },
                    }
                }
                ipc::IpcRequest::ListTransfers => {
                    ipc::IpcResponse::Transfers(server_sessions.clone())
                }
                other => ipc::IpcResponse::Error {
                    message: format!("unexpected request: {other:?}"),
                },
            };

            let encoded = ipc::encode_response(&resp).expect("encode resp");
            stream
                .write_all(&(encoded.len() as u32).to_be_bytes())
                .expect("write len");
            stream.write_all(&encoded).expect("write payload");
            stream.flush().expect("flush");
        }
    });

    let mut model = app::SettingsModel::from_init(app::SettingsInit {
        config: DaemonConfig::default(),
        config_path: None,
        everyone_remaining_secs: None,
        error_message: None,
        warning_message: None,
        transfer_error: None,
        paired_devices: Vec::new(),
        nearby_peers: Vec::new(),
        transfers: initial_sessions,
        send_paths: Vec::new(),
        selected_send_device_id: None,
        send_picker_active: false,
        device_store,
        secret_store: None,
        ipc_socket_path: Some(sock_path),
    });

    model.apply_msg(app::SettingsMsg::AcceptTransfer("tx-1".to_string()));
    assert!(model.transfer_error().is_none());
    assert_eq!(model.transfers()[0].state, TransferState::Active);

    model.apply_msg(app::SettingsMsg::CancelTransfer("tx-1".to_string()));
    assert!(model.transfer_error().is_none());
    assert_eq!(model.transfers()[0].state, TransferState::Cancelled);

    model.apply_msg(app::SettingsMsg::DeclineTransfer("tx-2".to_string()));
    assert!(model.transfer_error().is_none());
    assert_eq!(model.transfers()[1].state, TransferState::Cancelled);

    handle.join().expect("ipc server thread");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn cli_parses_send_picker_and_share_with_dash_filenames() {
    assert_eq!(
        parse_settings_args(["--send-picker", "--device", "pixel-9"]).unwrap(),
        SettingsCliAction::SendPicker {
            config_path: None,
            device_id: Some("pixel-9".to_string()),
        }
    );
    assert_eq!(
        parse_settings_args(["--share", "--", "-report.pdf", "/tmp/my dir"]).unwrap(),
        SettingsCliAction::Share {
            config_path: None,
            device_id: None,
            paths: vec![PathBuf::from("-report.pdf"), PathBuf::from("/tmp/my dir")],
        }
    );
    assert!(matches!(
        parse_settings_args(["--share"]),
        Err(KeynearError::Validation { field: "paths", .. })
    ));
    assert!(matches!(
        parse_settings_args(["--share", "-unknown-flag"]),
        Err(KeynearError::Validation { field: "args", .. })
    ));
}

#[test]
fn file_uri_to_path_decodes_escapes_and_rejects_non_file() {
    assert_eq!(
        app::file_uri_to_path("file:///tmp/my%20file%25.txt").unwrap(),
        PathBuf::from("/tmp/my file%.txt")
    );
    assert_eq!(
        app::file_uri_to_path("file://localhost/tmp/-leading.pdf").unwrap(),
        PathBuf::from("/tmp/-leading.pdf")
    );
    assert!(app::file_uri_to_path("https://example.com/a").is_err());
    assert!(app::file_uri_to_path("file:///tmp/bad%2").is_err());
}

#[test]
fn inline_rename_and_global_config_persist_and_sync() {
    let (dir, device_store) = temp_store("rename-config");
    let cfg_path = dir.join("config.toml");
    let incoming = dir.join("incoming");
    fs::create_dir_all(&incoming).expect("incoming");
    let initial_cfg = DaemonConfig {
        incoming_directory: incoming.display().to_string(),
        ..Default::default()
    };
    initial_cfg.save_to_path(&cfg_path).expect("save cfg");

    let dev = PairedDevice {
        device_id: "pixel-9".to_string(),
        display_name: "Pixel 9".to_string(),
        device_role: DeviceRole::Mobile,
        static_x25519_pubkey: "ab".repeat(32),
        last_ble_address: String::new(),
        last_lan_address: String::new(),
        last_endpoint_id: String::new(),
        capability_flags: 0x03,
        auto_accept_files: false,
        paired_at: "1".to_string(),
    };
    device_store.upsert(dev.clone()).expect("seed device");

    let mut model = app::SettingsModel::from_init(app::SettingsInit {
        config: initial_cfg,
        config_path: Some(cfg_path.clone()),
        everyone_remaining_secs: None,
        error_message: None,
        warning_message: None,
        transfer_error: None,
        paired_devices: vec![dev],
        nearby_peers: Vec::new(),
        transfers: Vec::new(),
        send_paths: Vec::new(),
        selected_send_device_id: None,
        send_picker_active: false,
        device_store: device_store.clone(),
        secret_store: None,
        ipc_socket_path: Some(dir.join("missing-daemon.sock")),
    });

    model.apply_msg(app::SettingsMsg::StartRename("pixel-9".to_string()));
    model.apply_msg(app::SettingsMsg::SubmitRename {
        device_id: "pixel-9".to_string(),
        display_name: "   ".to_string(),
    });
    assert!(model.error_message().is_some());

    model.apply_msg(app::SettingsMsg::SubmitRename {
        device_id: "pixel-9".to_string(),
        display_name: "Pixel Desk".to_string(),
    });
    assert!(model.error_message().is_none());
    assert_eq!(model.paired_devices()[0].display_name, "Pixel Desk");
    assert_eq!(
        device_store.load().expect("reload")[0].display_name,
        "Pixel Desk"
    );

    let updated_incoming = dir.join("received");
    model.apply_msg(app::SettingsMsg::SetGlobalAutoAccept(true));
    model.apply_msg(app::SettingsMsg::SetIncomingDirectory(
        updated_incoming.display().to_string(),
    ));
    assert!(model.auto_accept_from_paired());
    assert_eq!(
        model.incoming_directory(),
        updated_incoming.to_string_lossy()
    );
    let saved = DaemonConfig::load_or_default(&cfg_path).expect("reload saved cfg");
    assert!(saved.auto_accept_from_paired);
    assert_eq!(saved.incoming_directory, updated_incoming.to_string_lossy());

    model.apply_msg(app::SettingsMsg::StageSendPaths(vec![PathBuf::from(
        "/tmp/a.txt",
    )]));
    model.apply_msg(app::SettingsMsg::SelectSendDevice("pixel-9".to_string()));
    assert_eq!(model.send_paths(), &[PathBuf::from("/tmp/a.txt")]);
    assert_eq!(model.selected_send_device_id(), Some("pixel-9"));
    model.apply_msg(app::SettingsMsg::SendCompleted(Err(
        "daemon offline".to_string()
    )));
    assert_eq!(model.send_error(), Some("daemon offline"));

    model.apply_msg(app::SettingsMsg::RetryDaemon);
    assert!(model
        .transfer_error()
        .unwrap_or_default()
        .contains("unavailable"));

    let _ = fs::remove_dir_all(&dir);
}
