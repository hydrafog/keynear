use gtk4::prelude::*;
use relm4::ComponentSender;

use crate::state::{SettingsModel, SettingsMsg};
use crate::views::components::lucide_icon;

use super::common::current_detail_id;
use crate::views::components::LucideIcon;

pub(super) fn build_hero_card() -> (gtk4::Box, gtk4::Box, gtk4::Label, gtk4::Label) {
    let hero_card = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(14)
        .build();
    hero_card.add_css_class("keynear-card");

    let icon_box = gtk4::Box::builder()
        .width_request(56)
        .height_request(56)
        .halign(gtk4::Align::Center)
        .valign(gtk4::Align::Center)
        .build();
    icon_box.add_css_class("kn-loading-tonal-badge");
    let icon = lucide_icon(LucideIcon::Smartphone, 24);
    icon.set_hexpand(true);
    icon.set_vexpand(true);
    icon.set_halign(gtk4::Align::Center);
    icon.set_valign(gtk4::Align::Center);
    icon_box.append(&icon);

    let hero_text = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(3)
        .hexpand(true)
        .valign(gtk4::Align::Center)
        .build();
    let name_label = gtk4::Label::builder().label("").xalign(0.0).build();
    name_label.add_css_class("keynear-section-title");
    let status_label = gtk4::Label::builder()
        .label("")
        .xalign(0.0)
        .wrap(true)
        .build();
    status_label.add_css_class("keynear-caption");
    hero_text.append(&name_label);
    hero_text.append(&status_label);

    hero_card.append(&icon_box);
    hero_card.append(&hero_text);
    (hero_card, icon_box, name_label, status_label)
}

pub(super) fn build_rename_card(
    sender: &ComponentSender<SettingsModel>,
) -> (gtk4::Box, gtk4::Entry) {
    let rename_card = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(8)
        .build();
    rename_card.add_css_class("keynear-card");

    let name_entry = gtk4::Entry::builder().hexpand(true).build();
    name_entry.add_css_class("kn-recessed-entry");
    {
        let s = sender.clone();
        name_entry.connect_changed(move |e| {
            s.input(SettingsMsg::UpdateRenameDraft(e.text().to_string()))
        });
    }
    {
        let s = sender.clone();
        name_entry.connect_activate(move |e| {
            let id = current_detail_id(e);
            if !id.is_empty() {
                s.input(SettingsMsg::SubmitRename {
                    device_id: id,
                    display_name: e.text().to_string(),
                });
            }
        });
    }

    let save_btn = gtk4::Button::with_label("Save");
    save_btn.add_css_class("kn-loading-btn");
    {
        let s = sender.clone();
        let e_ref = name_entry.clone();
        save_btn.connect_clicked(move |_| {
            let id = current_detail_id(&e_ref);
            if !id.is_empty() {
                s.input(SettingsMsg::SubmitRename {
                    device_id: id,
                    display_name: e_ref.text().to_string(),
                });
            }
        });
    }

    rename_card.append(&name_entry);
    rename_card.append(&save_btn);
    (rename_card, name_entry)
}

pub(super) fn build_transfers_card(
    sender: &ComponentSender<SettingsModel>,
    name_entry: &gtk4::Entry,
) -> (gtk4::Box, gtk4::Switch) {
    let transfers_card = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(12)
        .build();
    transfers_card.add_css_class("keynear-card");

    let aa_text = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(2)
        .hexpand(true)
        .build();
    let aa_title = gtk4::Label::builder()
        .label("Auto-accept files")
        .xalign(0.0)
        .build();
    aa_title.add_css_class("keynear-description");
    let aa_desc = gtk4::Label::builder()
        .label("Save files from this device without asking")
        .xalign(0.0)
        .wrap(true)
        .build();
    aa_desc.add_css_class("keynear-caption");
    aa_text.append(&aa_title);
    aa_text.append(&aa_desc);

    let auto_accept_switch = gtk4::Switch::builder().valign(gtk4::Align::Center).build();
    auto_accept_switch.add_css_class("keynear-switch");
    {
        let s = sender.clone();
        let e_ref = name_entry.clone();
        auto_accept_switch.connect_state_set(move |_, enabled| {
            let id = current_detail_id(&e_ref);
            if !id.is_empty() {
                s.input(SettingsMsg::SetAutoAcceptFiles {
                    device_id: id,
                    enabled,
                });
            }
            gtk4::glib::Propagation::Proceed
        });
    }

    transfers_card.append(&aa_text);
    transfers_card.append(&auto_accept_switch);
    (transfers_card, auto_accept_switch)
}
