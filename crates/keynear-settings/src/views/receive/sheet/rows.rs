use gtk4::pango::EllipsizeMode;
use gtk4::prelude::*;
use keynear_protocol::{TransferSession, TransferState};
use relm4::ComponentSender;

use crate::state::{role_label, MapPin, SettingsModel, SettingsMsg};
use crate::views::components::{clear_box_children, lucide_icon};

use super::{transfer_fraction, transfer_summary, LucideIcon};

fn transfer_action_buttons(
    session: &TransferSession,
    sender: &ComponentSender<SettingsModel>,
) -> Vec<gtk4::Button> {
    let mut buttons = Vec::new();
    if session.state == TransferState::PendingConsent {
        let tid = session.transfer_id.clone();
        let accept_btn = gtk4::Button::with_label("Accept");
        accept_btn.add_css_class("kn-loading-btn");
        {
            let s = sender.clone();
            let id = tid.clone();
            accept_btn.connect_clicked(move |_| s.input(SettingsMsg::AcceptTransfer(id.clone())));
        }
        let decline_btn = gtk4::Button::with_label("Decline");
        decline_btn.add_css_class("kn-error-action-btn");
        {
            let s = sender.clone();
            decline_btn
                .connect_clicked(move |_| s.input(SettingsMsg::DeclineTransfer(tid.clone())));
        }
        buttons.push(accept_btn);
        buttons.push(decline_btn);
    } else if session.state == TransferState::Active {
        let tid = session.transfer_id.clone();
        let cancel_btn = gtk4::Button::with_label("Cancel");
        cancel_btn.add_css_class("kn-error-action-btn");
        {
            let s = sender.clone();
            cancel_btn.connect_clicked(move |_| s.input(SettingsMsg::CancelTransfer(tid.clone())));
        }
        buttons.push(cancel_btn);
    }
    buttons
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

        for btn in transfer_action_buttons(session, sender) {
            top.append(&btn);
        }

        let bar = gtk4::ProgressBar::new();
        bar.set_fraction(transfer_fraction(session));
        bar.add_css_class("keynear-progress");

        row.append(&top);
        row.append(&bar);
        container.append(&row);
    }
}

fn pin_sub_text(pin: &MapPin, model: &SettingsModel) -> String {
    if let Some(dev) = model.paired_devices.iter().find(|d| d.device_id == pin.id) {
        format!(
            "{} · Paired {} · {}",
            role_label(dev.device_role),
            dev.paired_at,
            if pin.is_online { "Wi-Fi" } else { "Standby" }
        )
    } else {
        format!("Nearby peer · {} dBm", pin.rssi)
    }
}

fn pin_action_button(pin: &MapPin, sender: &ComponentSender<SettingsModel>) -> gtk4::Button {
    let btn = if pin.is_paired {
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
        btn.connect_clicked(move |_| {
            if is_paired {
                s.input(SettingsMsg::OpenDeviceDetail(id.clone()));
            } else {
                s.input(SettingsMsg::BeginPair(id.clone()));
            }
        });
    }
    btn
}

fn pin_badge(pin: &MapPin) -> gtk4::Box {
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

    badge
}

fn pin_text_col(pin: &MapPin, model: &SettingsModel) -> gtk4::Box {
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

    let sub = gtk4::Label::builder()
        .label(pin_sub_text(pin, model))
        .xalign(0.0)
        .ellipsize(EllipsizeMode::End)
        .build();
    sub.add_css_class("keynear-caption");

    text_col.append(&name);
    text_col.append(&sub);

    text_col
}

fn sheet_row(
    pin: &MapPin,
    model: &SettingsModel,
    sender: &ComponentSender<SettingsModel>,
) -> gtk4::Box {
    let row = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(12)
        .build();
    row.add_css_class("kn-sheet-row");
    row.append(&pin_badge(pin));
    row.append(&pin_text_col(pin, model));
    row.append(&pin_action_button(pin, sender));
    row
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
        container.append(&sheet_row(pin, model, sender));
    }
}
