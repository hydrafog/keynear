use keynear_core::{DeviceRole, TransferItem};

use super::*;

#[test]
fn tray_snapshot_formats_countdown_and_devices() {
    let state = TrayState {
        visibility_mode: VisibilityMode::EveryoneNearby,
        everyone_remaining_secs: Some(125),
        auto_accept_from_paired: true,
        has_latest_otp: true,
        paired_devices: vec![PairedDevice {
            device_id: "pixel-9".to_string(),
            display_name: "Pixel 9".to_string(),
            device_role: DeviceRole::Mobile,
            static_x25519_pubkey: "abcd".to_string(),
            last_ble_address: String::new(),
            last_lan_address: String::new(),
            last_endpoint_id: String::new(),
            capability_flags: 1,
            auto_accept_files: true,
            paired_at: "2026-11-14T00:00:00Z".to_string(),
        }],
        transfers: Vec::new(),
        mdns_degraded: false,
        now_playing: None,
    };

    let snapshot = build_tray_snapshot(&state);
    assert_eq!(snapshot.visibility_selected, 2);
    assert_eq!(snapshot.visibility_labels[2], "Everyone (02:05)");
    assert_eq!(snapshot.title, "Keynear — Everyone (02:05)");
    assert_eq!(snapshot.paired_device_labels, vec!["Pixel 9 (auto-accept)"]);
    assert!(!snapshot.needs_attention);
    assert!(snapshot.has_latest_otp);
}

#[test]
fn tray_snapshot_marks_needs_attention_for_pending_consent() {
    let state = TrayState {
        visibility_mode: VisibilityMode::PairedOnly,
        everyone_remaining_secs: None,
        auto_accept_from_paired: false,
        has_latest_otp: false,
        paired_devices: Vec::new(),
        transfers: vec![TransferSession {
            transfer_id: "tx-7".to_string(),
            peer_device_id: "pixel-9".to_string(),
            direction: TransferDirection::Receive,
            items: vec![TransferItem {
                name: "notes.txt".to_string(),
                mime_type: "text/plain".to_string(),
                size_bytes: 512,
                is_directory: false,
                target_path: None,
                checksum: None,
            }],
            total_bytes: 512,
            transferred_bytes: 0,
            state: TransferState::PendingConsent,
            started_at: "2026-11-14T00:00:00Z".to_string(),
        }],
        mdns_degraded: false,
        now_playing: None,
    };

    let snapshot = build_tray_snapshot(&state);
    assert!(snapshot.needs_attention);
    assert_eq!(snapshot.icon_name, "mail-unread-symbolic");
    assert_eq!(
        snapshot.transfer_labels,
        vec!["Incoming: notes.txt from pixel-9"]
    );
}
