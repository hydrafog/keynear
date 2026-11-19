use gtk4::pango::EllipsizeMode;
use gtk4::prelude::*;
use keynear_protocol::{TransferDirection, TransferSession, TransferState};
use relm4::ComponentSender;

use crate::state::{role_label, MapPin, SettingsModel, SettingsMsg};
use crate::views::components::{clear_box_children, lucide_icon, LucideIcon};

pub(crate) struct ReceiveSheetsParts {
    pub(crate) transfers_card: gtk4::Box,
    pub(crate) transfers_error_banner: gtk4::Box,
    pub(crate) transfers_error_label: gtk4::Label,
    pub(crate) transfers_list_box: gtk4::Box,
    pub(crate) sheet_card: gtk4::Box,
    pub(crate) sheet_count_label: gtk4::Label,
    pub(crate) sheet_toggle_label: gtk4::Label,
    pub(crate) sheet_scroll: gtk4::ScrolledWindow,
    pub(crate) sheet_list_box: gtk4::Box,
}

pub(crate) fn build_receive_sheets(sender: &ComponentSender<SettingsModel>) -> ReceiveSheetsParts {
    let transfers_card = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(8)
        .margin_start(16)
        .margin_end(16)
        .margin_bottom(8)
        .visible(false)
        .build();
    transfers_card.add_css_class("keynear-card");

    let transfers_heading = gtk4::Label::builder()
        .label("ACTIVE TRANSFERS")
        .xalign(0.0)
        .build();
    transfers_heading.add_css_class("kn-section-heading");

    let transfers_error_banner = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(8)
        .visible(false)
        .build();
    transfers_error_banner.add_css_class("keynear-error");
    let transfers_error_label = gtk4::Label::builder()
        .label("")
        .xalign(0.0)
        .wrap(true)
        .build();
    transfers_error_label.add_css_class("keynear-description");
    transfers_error_banner.append(&transfers_error_label);

    let transfers_list_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(6)
        .build();

    transfers_card.append(&transfers_heading);
    transfers_card.append(&transfers_error_banner);
    transfers_card.append(&transfers_list_box);

    let sheet_card = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(8)
        .margin_start(16)
        .margin_end(16)
        .build();
    sheet_card.add_css_class("kn-bottom-sheet");

    let handle = gtk4::Box::builder()
        .halign(gtk4::Align::Center)
        .width_request(36)
        .height_request(4)
        .build();
    handle.add_css_class("kn-sheet-handle");
    sheet_card.append(&handle);

    let sheet_header = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(8)
        .build();

    let sheet_titles = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(2)
        .hexpand(true)
        .build();
    let sheet_title = gtk4::Label::builder()
        .label("Devices in range")
        .xalign(0.0)
        .build();
    sheet_title.add_css_class("keynear-section-title");
    let sheet_count_label = gtk4::Label::builder()
        .label("0 devices")
        .xalign(0.0)
        .build();
    sheet_count_label.add_css_class("keynear-caption");
    sheet_titles.append(&sheet_title);
    sheet_titles.append(&sheet_count_label);

    let sheet_toggle_label = gtk4::Label::builder().label("All devices").build();
    sheet_toggle_label.add_css_class("kn-link-label");
    let sheet_toggle_btn = gtk4::Button::builder().child(&sheet_toggle_label).build();
    sheet_toggle_btn.add_css_class("kn-link-btn");
    {
        let s = sender.clone();
        sheet_toggle_btn.connect_clicked(move |_| s.input(SettingsMsg::ToggleSheetExpanded));
    }

    let refresh_icon = lucide_icon(LucideIcon::RefreshCw, 18);
    let refresh_btn = gtk4::Button::builder()
        .child(&refresh_icon)
        .tooltip_text("Refresh devices and daemon state")
        .build();
    refresh_btn.add_css_class("kn-tonal-action-btn");
    {
        let s = sender.clone();
        refresh_btn.connect_clicked(move |_| s.input(SettingsMsg::RefreshPeers));
    }

    sheet_header.append(&sheet_titles);
    sheet_header.append(&sheet_toggle_btn);
    sheet_header.append(&refresh_btn);
    sheet_card.append(&sheet_header);

    let sheet_list_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(6)
        .build();

    let sheet_scroll = gtk4::ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .min_content_height(112)
        .max_content_height(112)
        .child(&sheet_list_box)
        .build();

    sheet_card.append(&sheet_scroll);

    ReceiveSheetsParts {
        transfers_card,
        transfers_error_banner,
        transfers_error_label,
        transfers_list_box,
        sheet_card,
        sheet_count_label,
        sheet_toggle_label,
        sheet_scroll,
        sheet_list_box,
    }
}

fn transfer_fraction(session: &TransferSession) -> f64 {
    if session.total_bytes == 0 {
        return match session.state {
            TransferState::Completed => 1.0,
            _ => 0.0,
        };
    }
    (session.transferred_bytes as f64 / session.total_bytes as f64).clamp(0.0, 1.0)
}

fn transfer_summary(session: &TransferSession) -> String {
    let dir = match session.direction {
        TransferDirection::Send => "To",
        TransferDirection::Receive => "From",
    };
    let state = match session.state {
        TransferState::PendingConsent => "Awaiting decision",
        TransferState::Active => "Transferring",
        TransferState::Completed => "Completed",
        TransferState::Cancelled => "Cancelled",
        TransferState::Failed => "Failed",
    };
    let title = session
        .items
        .first()
        .map(|i| i.name.as_str())
        .unwrap_or("Transfer");
    format!(
        "{title} ({} items, {state} · {dir} {})",
        session.items.len(),
        session.peer_device_id
    )
}

pub(crate) fn rebuild_transfers_rows(
    container: &gtk4::Box,
    transfers: &[TransferSession],
    sender: &ComponentSender<SettingsModel>,
) {
    clear_box_children(container);
    for session in transfers {
        let row = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(6)
            .build();
        row.add_css_class("keynear-transfer-row");

        let top = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(8)
            .build();

        let lbl = gtk4::Label::builder()
            .label(transfer_summary(session))
            .xalign(0.0)
            .hexpand(true)
            .ellipsize(EllipsizeMode::End)
            .build();
        lbl.add_css_class("keynear-description");
        top.append(&lbl);

        match session.state {
            TransferState::PendingConsent => {
                let tid = session.transfer_id.clone();
                let accept_btn = gtk4::Button::with_label("Accept");
                accept_btn.add_css_class("kn-loading-btn");
                {
                    let s = sender.clone();
                    let id = tid.clone();
                    accept_btn
                        .connect_clicked(move |_| s.input(SettingsMsg::AcceptTransfer(id.clone())));
                }
                let decline_btn = gtk4::Button::with_label("Decline");
                decline_btn.add_css_class("kn-error-action-btn");
                {
                    let s = sender.clone();
                    decline_btn.connect_clicked(move |_| {
                        s.input(SettingsMsg::DeclineTransfer(tid.clone()))
                    });
                }
                top.append(&accept_btn);
                top.append(&decline_btn);
            }
            TransferState::Active => {
                let tid = session.transfer_id.clone();
                let cancel_btn = gtk4::Button::with_label("Cancel");
                cancel_btn.add_css_class("kn-error-action-btn");
                {
                    let s = sender.clone();
                    cancel_btn.connect_clicked(move |_| {
                        s.input(SettingsMsg::CancelTransfer(tid.clone()))
                    });
                }
                top.append(&cancel_btn);
            }
            _ => {}
        }

        let bar = gtk4::ProgressBar::new();
        bar.set_fraction(transfer_fraction(session));
        bar.add_css_class("keynear-progress");

        row.append(&top);
        row.append(&bar);
        container.append(&row);
    }
}

pub(crate) fn rebuild_sheet_rows(
    container: &gtk4::Box,
    model: &SettingsModel,
    pins: &[MapPin],
    sender: &ComponentSender<SettingsModel>,
) {
    clear_box_children(container);

    if pins.is_empty() {
        let empty = gtk4::Label::builder()
            .label("Discovery is hidden. Turn on Paired or Everyone nearby to browse devices on this network.")
            .xalign(0.0)
            .wrap(true)
            .build();
        empty.add_css_class("keynear-caption");
        container.append(&empty);
        return;
    }

    for pin in pins {
        let row = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(12)
            .build();
        row.add_css_class("kn-sheet-row");

        let badge = gtk4::Box::builder()
            .width_request(44)
            .height_request(44)
            .halign(gtk4::Align::Center)
            .valign(gtk4::Align::Center)
            .build();
        badge.add_css_class(if pin.is_paired && pin.is_online {
            "kn-loading-tonal-badge"
        } else {
            "kn-surface-tonal-badge"
        });

        let icon = lucide_icon(
            if pin.is_paired {
                LucideIcon::Smartphone
            } else {
                LucideIcon::Radio
            },
            20,
        );
        icon.set_hexpand(true);
        icon.set_vexpand(true);
        icon.set_halign(gtk4::Align::Center);
        icon.set_valign(gtk4::Align::Center);
        badge.append(&icon);

        let text_col = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(2)
            .hexpand(true)
            .build();
        let name = gtk4::Label::builder()
            .label(&pin.display_name)
            .xalign(0.0)
            .ellipsize(EllipsizeMode::End)
            .build();
        name.add_css_class("keynear-description");

        let sub_text =
            if let Some(dev) = model.paired_devices.iter().find(|d| d.device_id == pin.id) {
                format!(
                    "{} · Paired {} · {}",
                    role_label(dev.device_role),
                    dev.paired_at,
                    if pin.is_online { "Wi-Fi" } else { "Standby" }
                )
            } else {
                format!("Nearby peer · {} dBm", pin.rssi)
            };
        let sub = gtk4::Label::builder()
            .label(&sub_text)
            .xalign(0.0)
            .ellipsize(EllipsizeMode::End)
            .build();
        sub.add_css_class("keynear-caption");

        text_col.append(&name);
        text_col.append(&sub);

        let action_btn = if pin.is_paired {
            let icon = lucide_icon(LucideIcon::ChevronRight, 18);
            let btn = gtk4::Button::builder().child(&icon).build();
            btn.add_css_class("kn-tonal-action-btn");
            btn
        } else {
            let btn = gtk4::Button::with_label("Pair");
            btn.add_css_class("kn-loading-btn");
            btn
        };
        {
            let s = sender.clone();
            let id = pin.id.clone();
            let is_paired = pin.is_paired;
            action_btn.connect_clicked(move |_| {
                if is_paired {
                    s.input(SettingsMsg::OpenDeviceDetail(id.clone()));
                } else {
                    s.input(SettingsMsg::BeginPair(id.clone()));
                }
            });
        }

        row.append(&badge);
        row.append(&text_col);
        row.append(&action_btn);
        container.append(&row);
    }
}
