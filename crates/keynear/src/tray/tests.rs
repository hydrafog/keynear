use std::path::Path;

use keynear_core::{DeviceRole, TransferItem};
use ksni::{MenuItem, Tray};
use tokio::sync::mpsc;

use super::*;

fn sample_paired_device(id: &str, name: &str, auto_accept: bool) -> PairedDevice {
    PairedDevice {
        device_id: id.to_string(),
        display_name: name.to_string(),
        device_role: DeviceRole::Mobile,
        static_x25519_pubkey: "abcd".to_string(),
        last_ble_address: String::new(),
        last_lan_address: String::new(),
        last_endpoint_id: String::new(),
        capability_flags: 1,
        auto_accept_files: auto_accept,
        paired_at: "2026-11-14T00:00:00Z".to_string(),
    }
}

fn item_label(item: &MenuItem<KeynearTray>) -> Option<&str> {
    match item {
        MenuItem::Standard(s) => Some(s.label.as_str()),
        MenuItem::SubMenu(s) => Some(s.label.as_str()),
        MenuItem::Checkmark(c) => Some(c.label.as_str()),
        _ => None,
    }
}

#[test]
fn tray_snapshot_formats_countdown_and_devices() {
    let state = TrayState {
        visibility_mode: VisibilityMode::EveryoneNearby,
        everyone_remaining_secs: Some(125),
        auto_accept_from_paired: true,
        has_latest_otp: true,
        paired_devices: vec![sample_paired_device("pixel-9", "Pixel 9", true)],
        transfers: Vec::new(),
        mdns_degraded: false,
        now_playing: None,
    };

    let snapshot = build_tray_snapshot(&state);
    assert_eq!(snapshot.visibility_selected, 2);
    assert_eq!(snapshot.visibility_labels[2], "Everyone (02:05)");
    assert_eq!(snapshot.title, "Keynear");
    assert_eq!(snapshot.status_summary, "Keynear — Everyone (02:05)");
    assert_eq!(snapshot.icon_name, "org.keynear.Keynear-symbolic");
    assert_eq!(snapshot.paired_device_labels, vec!["Pixel 9 (auto-accept)"]);
    assert!(!snapshot.needs_attention);
    assert!(snapshot.has_latest_otp);
}

#[test]
fn tray_snapshot_keeps_symbolic_icon_across_attention_and_degraded_states() {
    let pending = TransferSession {
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
    };

    let attention_state = TrayState {
        visibility_mode: VisibilityMode::PairedOnly,
        transfers: vec![pending],
        ..Default::default()
    };
    let snapshot = build_tray_snapshot(&attention_state);
    assert!(snapshot.needs_attention);
    assert_eq!(snapshot.icon_name, "org.keynear.Keynear-symbolic");
    assert_eq!(
        snapshot.transfer_labels,
        vec!["Incoming: notes.txt from pixel-9"]
    );

    for mode in [
        VisibilityMode::Hidden,
        VisibilityMode::PairedOnly,
        VisibilityMode::EveryoneNearby,
    ] {
        let degraded = TrayState {
            visibility_mode: mode,
            mdns_degraded: true,
            ..Default::default()
        };
        assert_eq!(
            build_tray_snapshot(&degraded).icon_name,
            "org.keynear.Keynear-symbolic"
        );
    }
}

#[test]
fn tray_trait_exposes_app_title_tooltip_icon_and_activation() {
    const {
        assert!(KeynearTray::MENU_ON_ACTIVATE);
    }
    let (tx, _rx) = mpsc::unbounded_channel();
    let tray = KeynearTray {
        state: TrayState::default(),
        cmd_tx: tx,
    };
    assert_eq!(tray.title(), "Keynear");
    assert_eq!(tray.icon_name(), "org.keynear.Keynear-symbolic");
    assert_eq!(tray.attention_icon_name(), "org.keynear.Keynear-symbolic");

    let tooltip = tray.tool_tip();
    assert_eq!(tooltip.title, "Keynear");
    assert_eq!(tooltip.description, "Paired devices only");
    assert_eq!(tooltip.icon_name, "org.keynear.Keynear-symbolic");

    let theme_path = tray.icon_theme_path();
    assert!(Path::new(&theme_path)
        .join("hicolor/symbolic/apps/org.keynear.Keynear-symbolic.svg")
        .is_file());
}

#[test]
fn tray_menu_renders_header_and_omits_settings() {
    let (tx, _rx) = mpsc::unbounded_channel();
    let tray = KeynearTray {
        state: TrayState::default(),
        cmd_tx: tx,
    };
    let menu = tray.menu();
    assert!(matches!(
        &menu[0],
        MenuItem::Standard(item) if !item.enabled && item.label == "Keynear — Paired devices only"
    ));
    let labels: Vec<&str> = menu.iter().filter_map(item_label).collect();
    assert!(labels.iter().all(|label| !label.starts_with("Settings")));
    assert_eq!(
        &labels[labels.len() - 2..],
        &["Send files...", "Send folder..."]
    );
}

#[test]
fn tray_menu_groups_send_files_by_device_when_multiple_paired() {
    let (tx, _rx) = mpsc::unbounded_channel();
    let multi = KeynearTray {
        state: TrayState {
            paired_devices: vec![
                sample_paired_device("pixel-9", "Pixel 9", false),
                sample_paired_device("tablet-1", "Tablet", false),
            ],
            ..Default::default()
        },
        cmd_tx: tx,
    };
    let multi_menu = multi.menu();
    assert!(matches!(
        &multi_menu[multi_menu.len() - 2],
        MenuItem::SubMenu(sub) if sub.label == "Send files..." && sub.submenu.len() == 2
    ));
    assert!(matches!(
        multi_menu.last(),
        Some(MenuItem::SubMenu(sub)) if sub.label == "Send folder..." && sub.submenu.len() == 2
    ));
}

#[test]
fn tray_menu_offers_immediate_device_removal() {
    let (tx, mut rx) = mpsc::unbounded_channel();
    let mut tray = KeynearTray {
        state: TrayState {
            paired_devices: vec![sample_paired_device("pixel-9", "Pixel 9", false)],
            ..Default::default()
        },
        cmd_tx: tx,
    };
    let menu = tray.menu();
    let paired = menu
        .iter()
        .find_map(|item| match item {
            MenuItem::SubMenu(sub) if sub.label == "Paired devices (1)" => Some(sub),
            _ => None,
        })
        .expect("paired devices submenu");
    let device = paired
        .submenu
        .iter()
        .find_map(|item| match item {
            MenuItem::SubMenu(sub) if sub.label == "Pixel 9" => Some(sub),
            _ => None,
        })
        .expect("device submenu");
    let labels: Vec<&str> = device.submenu.iter().filter_map(item_label).collect();
    assert!(labels.contains(&"Remove device"));
    assert!(!labels.iter().any(|label| label.contains("Confirm")));
    let remove = device
        .submenu
        .iter()
        .find_map(|item| match item {
            MenuItem::Standard(item) if item.label == "Remove device" => Some(item),
            _ => None,
        })
        .expect("remove item");
    (remove.activate)(&mut tray);
    assert_eq!(
        rx.try_recv().expect("command"),
        TrayCommand::UnpairDevice {
            device_id: "pixel-9".to_string()
        }
    );
}
