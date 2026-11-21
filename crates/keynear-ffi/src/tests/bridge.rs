use super::*;

#[test]
fn rename_paired_device_updates_store_and_validates_input() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("keynear-uniffi-rename-{nonce}"));
    fs::create_dir_all(&dir).expect("mkdir");
    let devices_path = dir.join("devices.toml");
    let devices_str = devices_path.to_string_lossy().to_string();

    let handle = CoreHandle::new();
    assert_eq!(get_core_status().version, "0.5.0");

    let seeded = FfiPairedDevice {
        device_id: "phone-1".to_string(),
        display_name: "Pixel".to_string(),
        device_role: FfiDeviceRole::Mobile,
        static_x25519_pubkey: "abcd".to_string(),
        last_ble_address: "11:22:33:44:55:66".to_string(),
        last_lan_address: String::new(),
        last_endpoint_id: String::new(),
        capability_flags: 1,
        auto_accept_files: false,
        paired_at: "2026-11-14T00:00:00Z".to_string(),
    };
    handle
        .save_paired_device(devices_str.clone(), seeded)
        .expect("save device");

    let updated = handle
        .rename_paired_device(
            devices_str.clone(),
            "phone-1".to_string(),
            "Pixel Daily".to_string(),
        )
        .expect("rename via uniffi");
    assert_eq!(updated.display_name, "Pixel Daily");

    assert!(handle
        .rename_paired_device(devices_str, "phone-1".to_string(), "   ".to_string())
        .is_err());

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn uniffi_bridge_daemon_config_roundtrips_all_prototype_fields() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("keynear-uniffi-cfg-{nonce}"));
    fs::create_dir_all(&dir).expect("mkdir");
    let cfg_path = dir.join("config.toml");
    let cfg_str = cfg_path.to_string_lossy().to_string();

    let handle = CoreHandle::new();
    let initial = handle
        .load_daemon_config(cfg_str.clone())
        .expect("load default daemon config");
    assert_eq!(initial.visibility_mode, FfiVisibilityMode::PairedOnly);
    assert!(initial.clipboard_sync_enabled);

    let updated = FfiDaemonConfig {
        visibility_mode: FfiVisibilityMode::EveryoneNearby,
        incoming_directory: "Downloads/".to_string(),
        auto_accept_from_paired: true,
        clipboard_sync_enabled: false,
        clipboard_images_enabled: false,
        otp_passthrough_enabled: false,
        audio_handoff_enabled: false,
        ignored_mime_types: vec![
            "application/x-secret".to_string(),
            "  x-kde-passwordManagerHint  ".to_string(),
        ],
        notify_auto_accept_complete: false,
    };
    let saved = handle
        .save_daemon_config(cfg_str.clone(), updated)
        .expect("save daemon config");
    assert_eq!(
        saved.ignored_mime_types,
        vec![
            "application/x-secret".to_string(),
            "x-kde-passwordManagerHint".to_string(),
        ]
    );
    assert_eq!(handle.visibility_mode(), FfiVisibilityMode::EveryoneNearby);

    let reloaded = handle
        .load_daemon_config(cfg_str)
        .expect("reload daemon config");
    assert_eq!(reloaded, saved);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn orion_node_handle_demuxes_clipboard_and_enforces_config() {
    use std::net::{IpAddr, Ipv4Addr};

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("keynear-orion-clip-{nonce}"));
    fs::create_dir_all(&dir).expect("mkdir");

    let loopback = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
    let linux_node = OrionNodeHandle::build_with_init(OrionNodeInit {
        device_id: "linux-host".to_string(),
        display_name: "Linux".to_string(),
        capability_flags: 1,
        quic_secret_key: vec![0x31; 32],
        incoming_directory: dir.to_string_lossy().to_string(),
        auto_accept_from_paired: true,
        require_explicit_fd_for_single_file: false,
        bind_addr: Some(loopback),
    })
    .expect("bind linux");

    let phone_node = OrionNodeHandle::build_with_init(OrionNodeInit {
        device_id: "pixel-phone".to_string(),
        display_name: "Pixel".to_string(),
        capability_flags: 1,
        quic_secret_key: vec![0x32; 32],
        incoming_directory: dir.to_string_lossy().to_string(),
        auto_accept_from_paired: true,
        require_explicit_fd_for_single_file: false,
        bind_addr: Some(loopback),
    })
    .expect("bind phone");

    let secret = vec![0x88; 32];
    let linux_ep = linux_node.local_lan_endpoint();
    let phone_ep = phone_node.local_lan_endpoint();

    linux_node
        .sync_peer(
            "pixel-phone".to_string(),
            phone_ep.endpoint_id,
            phone_ep.socket_addrs,
            secret.clone(),
            true,
        )
        .expect("sync phone");
    phone_node
        .sync_peer(
            "linux-host".to_string(),
            linux_ep.endpoint_id,
            linux_ep.socket_addrs,
            secret,
            true,
        )
        .expect("sync linux");

    linux_node
        .send_clipboard_text("hello from desktop".to_string(), false)
        .expect("send clipboard text");

    let mut received_phone = Vec::new();
    for _ in 0..50 {
        received_phone.extend(phone_node.poll_incoming_clipboard());
        if !received_phone.is_empty() {
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(received_phone.len(), 1);
    assert_eq!(received_phone[0].data, b"hello from desktop");
    assert!(!received_phone[0].is_otp);

    phone_node
        .send_clipboard_text("847291".to_string(), true)
        .expect("send OTP");

    let mut received_linux = Vec::new();
    for _ in 0..50 {
        received_linux.extend(linux_node.poll_incoming_clipboard());
        if !received_linux.is_empty() {
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(received_linux.len(), 1);
    assert_eq!(received_linux[0].data, b"847291");
    assert!(received_linux[0].is_otp);

    let large_img = vec![0x5Au8; 64 * 1024];
    let offer = linux_node
        .runtime
        .block_on(linux_node.clipboard.offer_clipboard_image(
            &linux_node.node,
            "pixel-phone",
            vec!["image/png".to_string()],
            Arc::new({
                let data = large_img.clone();
                move |_| Ok(data.clone())
            }),
        ))
        .expect("offer delayed image");

    let mut phone_offers = Vec::new();
    for _ in 0..50 {
        phone_offers.extend(phone_node.poll_incoming_clipboard());
        if !phone_offers.is_empty() {
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(phone_offers.len(), 1);
    assert!(phone_offers[0].is_image_offer);
    assert!(phone_offers[0].data.is_empty());

    let fetched = phone_node
        .fetch_clipboard_offer(
            "linux-host".to_string(),
            offer.offer_id,
            "image/png".to_string(),
        )
        .expect("fetch delayed offer");
    assert_eq!(fetched.data, large_img);

    phone_node.set_clipboard_config(
        true,
        true,
        false,
        vec!["application/x-password".to_string()],
    );
    let otp_err = phone_node
        .send_clipboard_text("123456".to_string(), true)
        .expect_err("disabled otp passthrough must reject");
    assert!(matches!(otp_err, FfiError::Validation { .. }));

    linux_node.shutdown();
    phone_node.shutdown();
    let _ = fs::remove_dir_all(&dir);
}
