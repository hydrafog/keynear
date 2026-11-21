use gtk4::prelude::*;
use relm4::ComponentSender;

use crate::state::{SettingsModel, SettingsMsg};
use crate::views::components::lucide_icon;

use super::common::{current_detail_id, meta_row};
use crate::views::components::LucideIcon;

pub(super) fn build_conn_card(
    sender: &ComponentSender<SettingsModel>,
    name_entry: &gtk4::Entry,
) -> (gtk4::Box, gtk4::Label, gtk4::Label, gtk4::Label) {
    let conn_card = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(8)
        .build();
    conn_card.add_css_class("keynear-card");

    let (key_row, key_value) = meta_row("Key fingerprint");
    let (ble_row, ble_value) = meta_row("Bluetooth address");
    let (lan_row, lan_value) = meta_row("Local Wi-Fi endpoint");

    let reset_row = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(8)
        .halign(gtk4::Align::Center)
        .build();
    reset_row.append(&lucide_icon(LucideIcon::RefreshCw, 18));
    reset_row.append(&gtk4::Label::new(Some("Reset Bluetooth bond")));
    let reset_ble_btn = gtk4::Button::builder().child(&reset_row).build();
    reset_ble_btn.add_css_class("kn-tonal-action-btn");
    {
        let s = sender.clone();
        let e_ref = name_entry.clone();
        reset_ble_btn.connect_clicked(move |_| {
            let id = current_detail_id(&e_ref);
            if !id.is_empty() {
                s.input(SettingsMsg::ResetBleBond(id));
            }
        });
    }

    conn_card.append(&key_row);
    conn_card.append(&ble_row);
    conn_card.append(&lan_row);
    conn_card.append(&reset_ble_btn);
    (conn_card, key_value, ble_value, lan_value)
}

pub(super) fn build_remove_btn(
    sender: &ComponentSender<SettingsModel>,
    name_entry: &gtk4::Entry,
) -> gtk4::Button {
    let remove_row = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(8)
        .halign(gtk4::Align::Center)
        .build();
    remove_row.append(&lucide_icon(LucideIcon::Trash2, 18));
    remove_row.append(&gtk4::Label::new(Some("Remove device")));
    let remove_btn = gtk4::Button::builder().child(&remove_row).build();
    remove_btn.add_css_class("keynear-danger-btn");
    {
        let s = sender.clone();
        let e_ref = name_entry.clone();
        remove_btn.connect_clicked(move |_| {
            let id = current_detail_id(&e_ref);
            if !id.is_empty() {
                s.input(SettingsMsg::RequestUnpair(id));
            }
        });
    }
    remove_btn
}

pub(super) fn build_confirm_box(
    sender: &ComponentSender<SettingsModel>,
    name_entry: &gtk4::Entry,
) -> (gtk4::Box, gtk4::Label) {
    let confirm_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(10)
        .visible(false)
        .build();
    confirm_box.add_css_class("keynear-error");

    let confirm_label = gtk4::Label::builder()
        .label("")
        .xalign(0.0)
        .wrap(true)
        .build();
    confirm_label.add_css_class("keynear-description");

    let confirm_btns = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(10)
        .homogeneous(true)
        .build();

    let cancel_unpair_btn = gtk4::Button::with_label("Cancel");
    cancel_unpair_btn.add_css_class("kn-tonal-action-btn");
    {
        let s = sender.clone();
        cancel_unpair_btn.connect_clicked(move |_| s.input(SettingsMsg::CancelUnpair));
    }

    let confirm_unpair_btn = gtk4::Button::with_label("Remove");
    confirm_unpair_btn.add_css_class("keynear-danger-btn");
    {
        let s = sender.clone();
        let e_ref = name_entry.clone();
        confirm_unpair_btn.connect_clicked(move |_| {
            let id = current_detail_id(&e_ref);
            if !id.is_empty() {
                s.input(SettingsMsg::ConfirmUnpair(id));
            }
        });
    }

    confirm_btns.append(&cancel_unpair_btn);
    confirm_btns.append(&confirm_unpair_btn);
    confirm_box.append(&confirm_label);
    confirm_box.append(&confirm_btns);
    (confirm_box, confirm_label)
}
