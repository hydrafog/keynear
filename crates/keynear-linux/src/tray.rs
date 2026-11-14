use std::process::{Command, Stdio};

use keynear_core::{
    PairedDevice, TransferDirection, TransferSession, TransferState, VisibilityMode,
    EVERYONE_NEARBY_TIMEOUT_SECS,
};
use ksni::TrayMethods;
use tokio::sync::mpsc::UnboundedSender;
use tracing::{info, warn};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrayCommand {
    SetVisibility(VisibilityMode),
    AcceptTransfer(String),
    DeclineTransfer(String),
    CancelTransfer(String),
    SetAutoAcceptFromPaired(bool),
    SetDeviceAutoAccept { device_id: String, enabled: bool },
    LaunchSettings,
    LaunchSendPicker { device_id: Option<String> },
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TrayState {
    pub visibility_mode: VisibilityMode,
    pub everyone_remaining_secs: Option<u64>,
    pub auto_accept_from_paired: bool,
    pub paired_devices: Vec<PairedDevice>,
    pub transfers: Vec<TransferSession>,
    pub mdns_degraded: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrayMenuSnapshot {
    pub title: String,
    pub icon_name: String,
    pub needs_attention: bool,
    pub visibility_selected: usize,
    pub visibility_labels: [String; 3],
    pub paired_device_labels: Vec<String>,
    pub transfer_labels: Vec<String>,
    pub auto_accept_from_paired: bool,
}

pub fn format_everyone_label(mode: VisibilityMode, remaining_secs: Option<u64>) -> String {
    let secs = match (mode, remaining_secs) {
        (VisibilityMode::EveryoneNearby, Some(rem)) => rem,
        (VisibilityMode::EveryoneNearby, None) => EVERYONE_NEARBY_TIMEOUT_SECS,
        (_, Some(rem)) if rem > 0 => rem,
        _ => EVERYONE_NEARBY_TIMEOUT_SECS,
    };
    let mins = secs / 60;
    let rem_secs = secs % 60;
    format!("Everyone ({mins:02}:{rem_secs:02})")
}

pub fn build_tray_snapshot(state: &TrayState) -> TrayMenuSnapshot {
    let visibility_selected = match state.visibility_mode {
        VisibilityMode::Hidden => 0,
        VisibilityMode::PairedOnly => 1,
        VisibilityMode::EveryoneNearby => 2,
    };
    let visibility_labels = [
        "Hidden".to_string(),
        "Paired devices only".to_string(),
        format_everyone_label(state.visibility_mode, state.everyone_remaining_secs),
    ];
    let needs_attention = state
        .transfers
        .iter()
        .any(|t| t.state == TransferState::PendingConsent);
    let icon_name = if needs_attention {
        "mail-unread-symbolic".to_string()
    } else if state.mdns_degraded && state.visibility_mode != VisibilityMode::Hidden {
        "dialog-warning-symbolic".to_string()
    } else {
        match state.visibility_mode {
            VisibilityMode::Hidden => "network-wireless-offline-symbolic".to_string(),
            VisibilityMode::PairedOnly => "network-wireless-encrypted-symbolic".to_string(),
            VisibilityMode::EveryoneNearby => {
                "network-wireless-signal-excellent-symbolic".to_string()
            }
        }
    };
    let title = format!("Keynear — {}", visibility_labels[visibility_selected]);
    let paired_device_labels = state
        .paired_devices
        .iter()
        .map(|d| {
            if d.auto_accept_files {
                format!("{} (auto-accept)", d.display_name)
            } else {
                d.display_name.clone()
            }
        })
        .collect();
    let transfer_labels = state
        .transfers
        .iter()
        .filter(|t| {
            matches!(
                t.state,
                TransferState::PendingConsent | TransferState::Active
            )
        })
        .map(format_transfer_menu_label)
        .collect();

    TrayMenuSnapshot {
        title,
        icon_name,
        needs_attention,
        visibility_selected,
        visibility_labels,
        paired_device_labels,
        transfer_labels,
        auto_accept_from_paired: state.auto_accept_from_paired,
    }
}

fn format_transfer_menu_label(session: &TransferSession) -> String {
    let direction = match session.direction {
        TransferDirection::Send => "Sending",
        TransferDirection::Receive => "Receiving",
    };
    let item_summary = if session.items.is_empty() {
        "files".to_string()
    } else if session.items.len() == 1 {
        session.items[0].name.clone()
    } else {
        format!(
            "{} (+{} more)",
            session.items[0].name,
            session.items.len() - 1
        )
    };
    match session.state {
        TransferState::PendingConsent => {
            format!("Incoming: {} from {}", item_summary, session.peer_device_id)
        }
        TransferState::Active => {
            let pct = session
                .transferred_bytes
                .saturating_mul(100)
                .checked_div(session.total_bytes)
                .unwrap_or(0)
                .min(100);
            format!("{direction}: {item_summary} ({pct}%)")
        }
        TransferState::Completed => format!("Completed: {item_summary}"),
        TransferState::Cancelled => format!("Cancelled: {item_summary}"),
        TransferState::Failed => format!("Failed: {item_summary}"),
    }
}

pub fn spawn_settings_process(extra_args: &[String]) -> std::io::Result<()> {
    let bin_path = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join("keynear-settings")))
        .filter(|candidate| candidate.is_file())
        .unwrap_or_else(|| std::path::PathBuf::from("keynear-settings"));

    Command::new(bin_path)
        .args(extra_args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    Ok(())
}

pub struct KeynearTray {
    pub state: TrayState,
    pub cmd_tx: UnboundedSender<TrayCommand>,
}

impl ksni::Tray for KeynearTray {
    fn id(&self) -> String {
        "org.keynear.Daemon".to_string()
    }

    fn title(&self) -> String {
        build_tray_snapshot(&self.state).title
    }

    fn icon_name(&self) -> String {
        build_tray_snapshot(&self.state).icon_name
    }

    fn status(&self) -> ksni::Status {
        if build_tray_snapshot(&self.state).needs_attention {
            ksni::Status::NeedsAttention
        } else {
            ksni::Status::Active
        }
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        let _ = self.cmd_tx.send(TrayCommand::LaunchSettings);
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::{CheckmarkItem, MenuItem, RadioGroup, RadioItem, StandardItem, SubMenu};

        let snapshot = build_tray_snapshot(&self.state);
        let mut items: Vec<MenuItem<Self>> = vec![
            RadioGroup {
                selected: snapshot.visibility_selected,
                select: Box::new(|this: &mut Self, index| {
                    let next = match index {
                        0 => VisibilityMode::Hidden,
                        1 => VisibilityMode::PairedOnly,
                        _ => VisibilityMode::EveryoneNearby,
                    };
                    let _ = this.cmd_tx.send(TrayCommand::SetVisibility(next));
                }),
                options: vec![
                    RadioItem {
                        label: snapshot.visibility_labels[0].clone(),
                        ..Default::default()
                    },
                    RadioItem {
                        label: snapshot.visibility_labels[1].clone(),
                        ..Default::default()
                    },
                    RadioItem {
                        label: snapshot.visibility_labels[2].clone(),
                        ..Default::default()
                    },
                ],
            }
            .into(),
            MenuItem::Separator,
        ];

        let active_transfers: Vec<&TransferSession> = self
            .state
            .transfers
            .iter()
            .filter(|t| {
                matches!(
                    t.state,
                    TransferState::PendingConsent | TransferState::Active
                )
            })
            .collect();

        if !active_transfers.is_empty() {
            let mut transfer_items: Vec<MenuItem<Self>> = Vec::new();
            for session in active_transfers {
                let tid = session.transfer_id.clone();
                let label = format_transfer_menu_label(session);
                let mut actions: Vec<MenuItem<Self>> = Vec::new();
                if session.state == TransferState::PendingConsent {
                    let accept_id = tid.clone();
                    let decline_id = tid.clone();
                    actions.push(
                        StandardItem {
                            label: "Accept".to_string(),
                            activate: Box::new(move |this: &mut Self| {
                                let _ = this
                                    .cmd_tx
                                    .send(TrayCommand::AcceptTransfer(accept_id.clone()));
                            }),
                            ..Default::default()
                        }
                        .into(),
                    );
                    actions.push(
                        StandardItem {
                            label: "Decline".to_string(),
                            activate: Box::new(move |this: &mut Self| {
                                let _ = this
                                    .cmd_tx
                                    .send(TrayCommand::DeclineTransfer(decline_id.clone()));
                            }),
                            ..Default::default()
                        }
                        .into(),
                    );
                }
                let cancel_id = tid;
                actions.push(
                    StandardItem {
                        label: "Cancel".to_string(),
                        activate: Box::new(move |this: &mut Self| {
                            let _ = this
                                .cmd_tx
                                .send(TrayCommand::CancelTransfer(cancel_id.clone()));
                        }),
                        ..Default::default()
                    }
                    .into(),
                );
                transfer_items.push(
                    SubMenu {
                        label,
                        submenu: actions,
                        ..Default::default()
                    }
                    .into(),
                );
            }
            items.push(
                SubMenu {
                    label: format!("Transfers ({})", transfer_items.len()),
                    submenu: transfer_items,
                    ..Default::default()
                }
                .into(),
            );
        }

        let paired_submenu: Vec<MenuItem<Self>> = if self.state.paired_devices.is_empty() {
            vec![StandardItem {
                label: "No paired devices".to_string(),
                enabled: false,
                ..Default::default()
            }
            .into()]
        } else {
            self.state
                .paired_devices
                .iter()
                .map(|device| {
                    let device_id_send = device.device_id.clone();
                    let device_id_toggle = device.device_id.clone();
                    let next_enabled = !device.auto_accept_files;
                    SubMenu {
                        label: device.display_name.clone(),
                        submenu: vec![
                            StandardItem {
                                label: "Send files...".to_string(),
                                activate: Box::new(move |this: &mut Self| {
                                    let _ = this.cmd_tx.send(TrayCommand::LaunchSendPicker {
                                        device_id: Some(device_id_send.clone()),
                                    });
                                }),
                                ..Default::default()
                            }
                            .into(),
                            CheckmarkItem {
                                label: "Auto-accept files".to_string(),
                                checked: device.auto_accept_files,
                                activate: Box::new(move |this: &mut Self| {
                                    let _ = this.cmd_tx.send(TrayCommand::SetDeviceAutoAccept {
                                        device_id: device_id_toggle.clone(),
                                        enabled: next_enabled,
                                    });
                                }),
                                ..Default::default()
                            }
                            .into(),
                        ],
                        ..Default::default()
                    }
                    .into()
                })
                .collect()
        };

        items.push(
            SubMenu {
                label: format!("Paired devices ({})", self.state.paired_devices.len()),
                submenu: paired_submenu,
                ..Default::default()
            }
            .into(),
        );
        items.push(
            CheckmarkItem {
                label: "Auto-accept from paired".to_string(),
                checked: self.state.auto_accept_from_paired,
                activate: Box::new(|this: &mut Self| {
                    let next = !this.state.auto_accept_from_paired;
                    let _ = this.cmd_tx.send(TrayCommand::SetAutoAcceptFromPaired(next));
                }),
                ..Default::default()
            }
            .into(),
        );
        items.push(MenuItem::Separator);
        items.push(
            StandardItem {
                label: "Send files...".to_string(),
                activate: Box::new(|this: &mut Self| {
                    let _ = this
                        .cmd_tx
                        .send(TrayCommand::LaunchSendPicker { device_id: None });
                }),
                ..Default::default()
            }
            .into(),
        );
        items.push(
            StandardItem {
                label: "Settings...".to_string(),
                activate: Box::new(|this: &mut Self| {
                    let _ = this.cmd_tx.send(TrayCommand::LaunchSettings);
                }),
                ..Default::default()
            }
            .into(),
        );
        items
    }
}

pub async fn spawn_tray(
    state: TrayState,
    cmd_tx: UnboundedSender<TrayCommand>,
) -> Option<ksni::Handle<KeynearTray>> {
    let tray = KeynearTray { state, cmd_tx };
    match tray.assume_sni_available(false).spawn().await {
        Ok(handle) => {
            info!(event = "tray_registered");
            Some(handle)
        }
        Err(err) => {
            warn!(event = "tray_unavailable", error = %err);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use keynear_core::{DeviceRole, TransferItem};

    use super::*;

    #[test]
    fn tray_snapshot_formats_countdown_and_devices() {
        let state = TrayState {
            visibility_mode: VisibilityMode::EveryoneNearby,
            everyone_remaining_secs: Some(125),
            auto_accept_from_paired: true,
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
        };

        let snapshot = build_tray_snapshot(&state);
        assert_eq!(snapshot.visibility_selected, 2);
        assert_eq!(snapshot.visibility_labels[2], "Everyone (02:05)");
        assert_eq!(snapshot.title, "Keynear — Everyone (02:05)");
        assert_eq!(snapshot.paired_device_labels, vec!["Pixel 9 (auto-accept)"]);
        assert!(!snapshot.needs_attention);
    }

    #[test]
    fn tray_snapshot_marks_needs_attention_for_pending_consent() {
        let state = TrayState {
            visibility_mode: VisibilityMode::PairedOnly,
            everyone_remaining_secs: None,
            auto_accept_from_paired: false,
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
        };

        let snapshot = build_tray_snapshot(&state);
        assert!(snapshot.needs_attention);
        assert_eq!(snapshot.icon_name, "mail-unread-symbolic");
        assert_eq!(
            snapshot.transfer_labels,
            vec!["Incoming: notes.txt from pixel-9"]
        );
    }
}
