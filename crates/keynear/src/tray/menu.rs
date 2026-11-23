use keynear_core::{TransferSession, TransferState, VisibilityMode};
use ksni::menu::{CheckmarkItem, MenuItem, RadioGroup, RadioItem, StandardItem, SubMenu};
use ksni::Tray;

use super::{
    build_tray_snapshot, format_transfer_menu_label, media_menu::now_playing_menu,
    resolve_icon_theme_path, KeynearTray, TrayCommand, TrayMenuSnapshot, COPY_LATEST_OTP_LABEL,
    TRAY_APP_NAME, TRAY_ICON_NAME,
};

fn disabled_item(label: String) -> MenuItem<KeynearTray> {
    StandardItem {
        label,
        enabled: false,
        ..Default::default()
    }
    .into()
}

fn command_item(label: &str, cmd: TrayCommand) -> MenuItem<KeynearTray> {
    StandardItem {
        label: label.to_string(),
        activate: Box::new(move |this: &mut KeynearTray| {
            let _ = this.cmd_tx.send(cmd.clone());
        }),
        ..Default::default()
    }
    .into()
}

impl KeynearTray {
    fn visibility_group(&self, snapshot: &TrayMenuSnapshot) -> MenuItem<Self> {
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
            options: snapshot
                .visibility_labels
                .iter()
                .map(|label| RadioItem {
                    label: label.clone(),
                    ..Default::default()
                })
                .collect(),
        }
        .into()
    }

    fn transfer_submenu(session: &TransferSession) -> MenuItem<Self> {
        let tid = session.transfer_id.clone();
        let mut actions: Vec<MenuItem<Self>> = Vec::new();
        if session.state == TransferState::PendingConsent {
            actions.push(command_item(
                "Accept",
                TrayCommand::AcceptTransfer(tid.clone()),
            ));
            actions.push(command_item(
                "Decline",
                TrayCommand::DeclineTransfer(tid.clone()),
            ));
        }
        actions.push(command_item("Cancel", TrayCommand::CancelTransfer(tid)));
        SubMenu {
            label: format_transfer_menu_label(session),
            submenu: actions,
            ..Default::default()
        }
        .into()
    }

    fn transfer_menu(&self) -> Option<MenuItem<Self>> {
        let transfer_items: Vec<MenuItem<Self>> = self
            .state
            .transfers
            .iter()
            .filter(|t| {
                matches!(
                    t.state,
                    TransferState::PendingConsent | TransferState::Active
                )
            })
            .map(Self::transfer_submenu)
            .collect();
        if transfer_items.is_empty() {
            return None;
        }
        Some(
            SubMenu {
                label: format!("Transfers ({})", transfer_items.len()),
                submenu: transfer_items,
                ..Default::default()
            }
            .into(),
        )
    }

    fn paired_device_submenu(device: &keynear_core::PairedDevice) -> MenuItem<Self> {
        let device_id_toggle = device.device_id.clone();
        let next_enabled = !device.auto_accept_files;
        SubMenu {
            label: device.display_name.clone(),
            submenu: vec![
                command_item(
                    "Send files...",
                    TrayCommand::LaunchSendPicker {
                        device_id: Some(device.device_id.clone()),
                        directory: false,
                    },
                ),
                command_item(
                    "Send folder...",
                    TrayCommand::LaunchSendPicker {
                        device_id: Some(device.device_id.clone()),
                        directory: true,
                    },
                ),
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
    }

    fn paired_devices_menu(&self) -> MenuItem<Self> {
        let submenu: Vec<MenuItem<Self>> = if self.state.paired_devices.is_empty() {
            vec![disabled_item("No paired devices".to_string())]
        } else {
            self.state
                .paired_devices
                .iter()
                .map(Self::paired_device_submenu)
                .collect()
        };
        SubMenu {
            label: format!("Paired devices ({})", self.state.paired_devices.len()),
            submenu,
            ..Default::default()
        }
        .into()
    }

    fn send_picker_item(&self, label: &str, directory: bool) -> MenuItem<Self> {
        if self.state.paired_devices.len() > 1 {
            let targets = self
                .state
                .paired_devices
                .iter()
                .map(|device| {
                    command_item(
                        &device.display_name,
                        TrayCommand::LaunchSendPicker {
                            device_id: Some(device.device_id.clone()),
                            directory,
                        },
                    )
                })
                .collect();
            return SubMenu {
                label: label.to_string(),
                submenu: targets,
                ..Default::default()
            }
            .into();
        }
        let default_device = self
            .state
            .paired_devices
            .first()
            .map(|d| d.device_id.clone());
        StandardItem {
            label: label.to_string(),
            enabled: !self.state.paired_devices.is_empty(),
            activate: Box::new(move |this: &mut Self| {
                let _ = this.cmd_tx.send(TrayCommand::LaunchSendPicker {
                    device_id: default_device.clone(),
                    directory,
                });
            }),
            ..Default::default()
        }
        .into()
    }

    fn action_items(&self) -> Vec<MenuItem<Self>> {
        vec![
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
            StandardItem {
                label: COPY_LATEST_OTP_LABEL.to_string(),
                enabled: self.state.has_latest_otp,
                activate: Box::new(|this: &mut Self| {
                    let _ = this.cmd_tx.send(TrayCommand::CopyLatestOtp);
                }),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            self.send_picker_item("Send files...", false),
            self.send_picker_item("Send folder...", true),
        ]
    }
}

impl Tray for KeynearTray {
    const MENU_ON_ACTIVATE: bool = true;

    fn id(&self) -> String {
        "org.keynear.Daemon".to_string()
    }

    fn title(&self) -> String {
        TRAY_APP_NAME.to_string()
    }

    fn icon_name(&self) -> String {
        TRAY_ICON_NAME.to_string()
    }

    fn attention_icon_name(&self) -> String {
        TRAY_ICON_NAME.to_string()
    }

    fn icon_theme_path(&self) -> String {
        resolve_icon_theme_path()
    }

    fn tool_tip(&self) -> ksni::ToolTip {
        let snap = build_tray_snapshot(&self.state);
        ksni::ToolTip {
            icon_name: snap.icon_name,
            icon_pixmap: Vec::new(),
            title: snap.title,
            description: snap.status_summary,
        }
    }

    fn status(&self) -> ksni::Status {
        if build_tray_snapshot(&self.state).needs_attention {
            ksni::Status::NeedsAttention
        } else {
            ksni::Status::Active
        }
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let snapshot = build_tray_snapshot(&self.state);
        let mut items: Vec<MenuItem<Self>> = vec![
            disabled_item(snapshot.status_summary.clone()),
            MenuItem::Separator,
            self.visibility_group(&snapshot),
            MenuItem::Separator,
        ];
        if let Some(transfer_menu) = self.transfer_menu() {
            items.push(transfer_menu);
        }
        items.push(self.paired_devices_menu());
        if let Some(ref now_playing) = snapshot.now_playing {
            items.push(now_playing_menu(now_playing));
        }
        items.extend(self.action_items());
        items
    }
}
