use super::*;

#[test]
fn core_status_reports_ready() {
    let handle = CoreHandle::new();
    let status = handle.status();
    assert!(status.ready);
    assert_eq!(
        status.max_control_frame_bytes,
        MAX_CONTROL_FRAME_BYTES as u32
    );
    assert_eq!(handle.visibility_mode(), FfiVisibilityMode::PairedOnly);
    assert_eq!(ble_service_uuid16(), 0xFE6D);
    assert_eq!(everyone_nearby_timeout_secs(), 600);
    assert_eq!(pairing_timeout_secs(), 120);
}

#[test]
fn core_handle_cycles_visibility_thread_safely() {
    let handle = Arc::new(CoreHandle::new());
    let mut workers = Vec::new();
    for _ in 0..6 {
        let shared = Arc::clone(&handle);
        workers.push(thread::spawn(move || {
            shared.cycle_visibility_mode();
        }));
    }
    for worker in workers {
        worker.join().expect("thread completes");
    }
    assert_eq!(handle.visibility_mode(), FfiVisibilityMode::PairedOnly);
}

#[test]
fn everyone_nearby_deadline_auto_reverts_to_paired_only() {
    let now = Instant::now();
    let mut inner = VisibilityStateInner::default();
    inner.set_mode(VisibilityMode::EveryoneNearby, now);
    assert_eq!(inner.remaining_secs(now + Duration::from_secs(1)), 599);

    let expired_remaining = inner.remaining_secs(now + Duration::from_secs(600));
    assert_eq!(expired_remaining, 0);
    assert_eq!(inner.mode, VisibilityMode::PairedOnly);
}

#[test]
fn load_daemon_config_does_not_reset_active_everyone_deadline() {
    let dir = std::env::temp_dir().join(format!("keynear-ffi-deadline-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("config.toml");
    let handle = CoreHandle::new();
    handle.set_visibility_mode(FfiVisibilityMode::EveryoneNearby);
    let cfg = handle
        .load_daemon_config(path.display().to_string())
        .expect("load default");
    let saved = FfiDaemonConfig {
        visibility_mode: FfiVisibilityMode::EveryoneNearby,
        ..cfg
    };
    handle
        .save_daemon_config(path.display().to_string(), saved)
        .expect("save everyone");
    std::thread::sleep(Duration::from_millis(1100));
    let before = handle.everyone_nearby_remaining_secs();
    handle
        .load_daemon_config(path.display().to_string())
        .expect("reload config");
    let after = handle.everyone_nearby_remaining_secs();
    assert!(
        after <= before,
        "reloading config must not reset timer: before={before} after={after}"
    );
    assert!(before < 600, "timer must count down: before={before}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn ffi_beacons_and_noise_pairing_roundtrip() {
    let handle = CoreHandle::new();
    let beacon_bytes = handle
        .encode_everyone_nearby_beacon(FfiDeviceRole::Mobile, 0x03, "Pixel 9".to_string())
        .expect("encode everyone nearby");
    let decoded_beacon = handle
        .decode_everyone_nearby_beacon(beacon_bytes)
        .expect("decode everyone nearby");
    assert_eq!(decoded_beacon.role, FfiDeviceRole::Mobile);
    assert_eq!(decoded_beacon.display_name, "Pixel 9");

    let init_kp = handle.generate_static_keypair();
    let resp_kp = handle.generate_static_keypair();

    let initiator = FfiNoisePairingSession::new_initiator(
        init_kp.private_key,
        "phone-1".to_string(),
        "Pixel 9".to_string(),
        FfiDeviceRole::Mobile,
        0x03,
    )
    .expect("init session");
    let responder = FfiNoisePairingSession::new_responder(
        resp_kp.private_key,
        "linux-1".to_string(),
        "Workstation".to_string(),
        FfiDeviceRole::Desktop,
        0x07,
    )
    .expect("resp session");

    let m1 = initiator.start_handshake().expect("m1");
    let m2 = responder
        .step(m1.outbound_frame.expect("m1 bytes"))
        .expect("m2");
    let m3 = initiator
        .step(m2.outbound_frame.expect("m2 bytes"))
        .expect("m3");
    let fin = responder
        .step(m3.outbound_frame.expect("m3 bytes"))
        .expect("m3 read");
    assert_eq!(m3.state, FfiPairingState::AwaitingConfirmation);
    assert_eq!(fin.state, FfiPairingState::AwaitingConfirmation);
    assert_eq!(
        initiator.formatted_verification_code().expect("init code"),
        responder.formatted_verification_code().expect("resp code")
    );

    let init_export = initiator.approve_and_build_export().expect("init export");
    let resp_export = responder.approve_and_build_export().expect("resp export");
    let init_done = initiator
        .complete_with_peer_export(resp_export)
        .expect("init done");
    let resp_done = responder
        .complete_with_peer_export(init_export)
        .expect("resp done");
    assert_eq!(init_done.reconnect_secret, resp_done.reconnect_secret);

    let key_debug = format!("{init_done:?}");
    assert!(key_debug.contains("[REDACTED]"));
    assert!(!key_debug.contains(&hex::encode(&init_done.reconnect_secret)));

    let paired_bytes = handle
        .encode_paired_beacon(
            "phone-1".to_string(),
            5,
            init_done.reconnect_secret.clone(),
            true,
            Some(88),
        )
        .expect("encode paired beacon");
    let decoded_paired = handle
        .decode_paired_beacon(
            paired_bytes,
            "phone-1".to_string(),
            5,
            None,
            init_done.reconnect_secret,
        )
        .expect("decode paired beacon");
    assert_eq!(decoded_paired.counter, 5);
    assert!(decoded_paired.available);
    assert_eq!(decoded_paired.battery_level, Some(88));
}

#[test]
fn ffi_paired_device_store_roundtrip() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("keynear-uniffi-test-{nonce}"));
    fs::create_dir_all(&dir).expect("tempdir");
    let path = dir.join("devices.toml").display().to_string();
    let handle = CoreHandle::new();

    handle
        .save_paired_device(
            path.clone(),
            FfiPairedDevice {
                device_id: "linux-1".to_string(),
                display_name: "Workstation".to_string(),
                device_role: FfiDeviceRole::Desktop,
                static_x25519_pubkey: "ab".repeat(32),
                last_ble_address: "AA:BB:CC:DD:EE:FF".to_string(),
                last_lan_address: String::new(),
                last_endpoint_id: String::new(),
                capability_flags: 0x03,
                auto_accept_files: false,
                paired_at: "2026-11-13T20:00:00Z".to_string(),
            },
        )
        .expect("save device");

    let loaded = handle
        .load_paired_devices(path.clone())
        .expect("load devices");
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].device_id, "linux-1");
    assert!(handle
        .remove_paired_device(path.clone(), "linux-1".to_string())
        .expect("remove"));
    assert!(handle
        .load_paired_devices(path)
        .expect("empty after remove")
        .is_empty());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn validate_config_toml_checks_schema_and_validation() {
    let handle = CoreHandle::new();
    let mode = handle
        .validate_config_toml("visibilityMode = \"everyoneNearby\"\n".to_string())
        .expect("valid toml succeeds");
    assert_eq!(mode, FfiVisibilityMode::EveryoneNearby);

    let err = handle
        .validate_config_toml("incomingDirectory = \"\"\n".to_string())
        .expect_err("empty incoming directory fails");
    assert!(matches!(err, FfiError::Validation { .. }));
}

#[test]
fn validate_control_frame_checks_envelope_and_size() {
    let handle = CoreHandle::new();
    let env = v1::Envelope {
        version: 1,
        sequence: 11,
        timestamp_ms: 1_763_000_000_000,
        payload: Some(v1::envelope::Payload::ControlAck(v1::ControlAck {
            acked_sequence: 10,
            accepted: true,
            reason: String::new(),
        })),
    };
    let frame = envelope_to_control_frame(&env).expect("envelope to frame");
    let bytes = encode_control_frame(&frame).expect("encode frame");
    let kind = handle
        .validate_control_frame(bytes)
        .expect("valid control frame accepted");
    assert_eq!(kind, "control_ack");

    let oversized = vec![0u8; MAX_CONTROL_FRAME_BYTES + 1];
    assert!(matches!(
        handle.validate_control_frame(oversized),
        Err(FfiError::FrameTooLarge { .. })
    ));

    let empty_payload = v1::ControlAck {
        acked_sequence: 1,
        accepted: true,
        reason: String::new(),
    }
    .encode_to_vec();
    let bad_kind = ControlFrame {
        version: 1,
        sequence: 1,
        timestamp_ms: 1,
        kind: "invalid".to_string(),
        payload: empty_payload,
    };
    let bad_bytes = encode_control_frame(&bad_kind).expect("encode bad kind frame");
    assert!(matches!(
        handle.validate_control_frame(bad_bytes),
        Err(FfiError::Protocol { .. })
    ));
}
