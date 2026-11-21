mod cards;
mod common;
mod detail;

use gtk4::prelude::*;
use relm4::ComponentSender;

use crate::state::SettingsModel;

use super::DeviceDetailParts;

pub(crate) fn build_device_detail(sender: &ComponentSender<SettingsModel>) -> DeviceDetailParts {
    let page = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(16)
        .margin_start(16)
        .margin_end(16)
        .margin_top(4)
        .margin_bottom(24)
        .build();

    let (hero_card, icon_box, name_label, status_label) = cards::build_hero_card();
    page.append(&hero_card);

    page.append(&common::section_heading("COMPUTER NAME"));
    let (rename_card, name_entry) = cards::build_rename_card(sender);
    page.append(&rename_card);

    page.append(&common::section_heading("TRANSFERS"));
    let (transfers_card, auto_accept_switch) = cards::build_transfers_card(sender, &name_entry);
    page.append(&transfers_card);

    page.append(&common::section_heading("CONNECTION"));
    let (conn_card, key_value, ble_value, lan_value) = detail::build_conn_card(sender, &name_entry);
    page.append(&conn_card);

    let remove_btn = detail::build_remove_btn(sender, &name_entry);
    page.append(&remove_btn);

    let (confirm_box, confirm_label) = detail::build_confirm_box(sender, &name_entry);
    page.append(&confirm_box);

    let root = gtk4::ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vexpand(true)
        .child(&page)
        .build();

    DeviceDetailParts {
        root,
        icon_box,
        name_label,
        status_label,
        name_entry,
        auto_accept_switch,
        key_value,
        ble_value,
        lan_value,
        remove_btn,
        confirm_box,
        confirm_label,
    }
}
