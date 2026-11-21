use keynear_core::{TransferSession, TransferState, VisibilityMode};
use ksni::menu::{CheckmarkItem, MenuItem, RadioGroup, RadioItem, StandardItem, SubMenu};
use ksni::Tray;

use super::{
    build_tray_snapshot, format_transfer_menu_label, KeynearTray, TrayCommand, TrayMenuSnapshot,
    COPY_LATEST_OTP_LABEL,
};

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
        .into()
    }

    fn transfer_submenu(session: &TransferSession) -> MenuItem<Self> {
        let tid = session.transfer_id.clone();
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
        SubMenu {
            label: format_transfer_menu_label(session),
            submenu: actions,
            ..Default::default()
        }
        .into()
    }

    fn transfer_menu(&self) -> Option<MenuItem<Self>> {
        let active: Vec<&TransferSession> = self
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
        if active.is_empty() {
            return None;
        }
        let transfer_items: Vec<MenuItem<Self>> = active
            .iter()
            .map(|session| Self::transfer_submenu(session))
            .collect();
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
    }

    fn paired_devices_menu(&self) -> MenuItem<Self> {
        let submenu: Vec<MenuItem<Self>> = if self.state.paired_devices.is_empty() {
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
            StandardItem {
                label: "Settings...".to_string(),
                activate: Box::new(|this: &mut Self| {
                    let _ = this.cmd_tx.send(TrayCommand::LaunchSettings);
                }),
                ..Default::default()
            }
            .into(),
        ]
    }
}

impl Tray for KeynearTray {
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

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let snapshot = build_tray_snapshot(&self.state);
        let mut items: Vec<MenuItem<Self>> =
            vec![self.visibility_group(&snapshot), MenuItem::Separator];
        if let Some(transfer_menu) = self.transfer_menu() {
            items.push(transfer_menu);
        }
        items.push(self.paired_devices_menu());
        items.extend(self.action_items());
        items
    }
}
