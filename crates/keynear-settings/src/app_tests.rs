use crate::state::*;
use keynear_core::{DaemonConfig, DeviceRole, PairedDevice, PairedDeviceStore};
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn orbit_for_matches_prototype_formula() {
    let near = orbit_for(-40, 0, 2);
    assert_eq!(near.radius_px, 128);
    assert_eq!(near.duration_secs, 40);
    assert_eq!(near.phase_secs, 0);
    assert_eq!(near.offset_x, 0);
    assert_eq!(near.offset_y, -128);

    let far = orbit_for(-90, 1, 2);
    assert_eq!(far.radius_px, 176);
    assert_eq!(far.phase_secs, 20);
    assert_eq!(far.offset_x, 0);
    assert_eq!(far.offset_y, 176);
}

#[test]
fn prototype_tabs_send_queue_and_settings_persist() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("keynear-settings-proto-{nonce}"));
    fs::create_dir_all(&dir).expect("mkdir");
    let cfg_path = dir.join("config.toml");
    let cfg = DaemonConfig::default();
    cfg.save_to_path(&cfg_path).expect("save cfg");
    let store = PairedDeviceStore::new(dir.join("devices.toml"));

    let mut model = SettingsModel::from_init(SettingsInit {
        config: cfg,
        config_path: Some(cfg_path.clone()),
        everyone_remaining_secs: None,
        error_message: None,
        warning_message: None,
        transfer_error: None,
        paired_devices: vec![PairedDevice {
            device_id: "phone-1".to_string(),
            display_name: "Pixel".to_string(),
            device_role: DeviceRole::Mobile,
            static_x25519_pubkey: "abcd".repeat(16),
            last_ble_address: "AA:BB:CC:DD:EE:FF".to_string(),
            last_lan_address: "192.168.1.20:4433".to_string(),
            last_endpoint_id: String::new(),
            capability_flags: 1,
            auto_accept_files: false,
            paired_at: "Nov 14, 2026".to_string(),
        }],
        nearby_peers: Vec::new(),
        transfers: Vec::new(),
        send_paths: Vec::new(),
        selected_send_device_id: None,
        send_picker_active: false,
        device_store: store,
        secret_store: None,
        ipc_socket_path: None,
    });

    assert_eq!(model.active_tab(), ActiveTab::Receive);
    model.apply_msg(SettingsMsg::OpenDeviceDetail("phone-1".to_string()));
    assert_eq!(
        model.sub_screen(),
        Some(&SubScreen::DeviceDetail("phone-1".to_string()))
    );
    model.apply_msg(SettingsMsg::NavigateBack);
    assert!(model.sub_screen().is_none());

    model.apply_msg(SettingsMsg::SwitchTab(ActiveTab::Send));
    model.apply_msg(SettingsMsg::StageClipboardForSend);
    model.apply_msg(SettingsMsg::ToggleTextComposer);
    model.apply_msg(SettingsMsg::UpdateTextDraft(
        "https://keynear.example/spec".to_string(),
    ));
    model.apply_msg(SettingsMsg::StageTextSnippet);
    assert_eq!(model.send_queue().len(), 2);

    let first_id = model.send_queue()[0].id.clone();
    model.apply_msg(SettingsMsg::RemoveSendQueueItem(first_id));
    assert_eq!(model.send_queue().len(), 1);
    model.apply_msg(SettingsMsg::ClearSendPaths);
    assert!(model.send_queue().is_empty());

    model.apply_msg(SettingsMsg::SwitchTab(ActiveTab::Settings));
    model.apply_msg(SettingsMsg::ToggleClipboardSync(false));
    model.apply_msg(SettingsMsg::ToggleClipboardImages(false));
    model.apply_msg(SettingsMsg::ToggleOtpPassthrough(false));
    model.apply_msg(SettingsMsg::ToggleAudioHandoff(false));
    model.apply_msg(SettingsMsg::ToggleNotifyAutoAccept(false));
    model.apply_msg(SettingsMsg::UpdateMimeInput(
        "application/x-secret".to_string(),
    ));
    model.apply_msg(SettingsMsg::AddIgnoredMime);
    assert!(model
        .ignored_mime_types()
        .contains(&"application/x-secret".to_string()));
    model.apply_msg(SettingsMsg::RemoveIgnoredMime(
        "application/x-secret".to_string(),
    ));
    assert!(!model
        .ignored_mime_types()
        .contains(&"application/x-secret".to_string()));
    model.apply_msg(SettingsMsg::ToggleDarkTheme(true));
    assert!(model.dark_theme());

    let reloaded = DaemonConfig::load_or_default(&cfg_path).expect("reload");
    assert!(!reloaded.clipboard_sync_enabled);
    assert!(!reloaded.clipboard_images_enabled);
    assert!(!reloaded.otp_passthrough_enabled);
    assert!(!reloaded.audio_handoff_enabled);
    assert!(!reloaded.notify_auto_accept_complete);

    let _ = fs::remove_dir_all(&dir);
}
