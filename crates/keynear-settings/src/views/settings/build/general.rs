use gtk4::prelude::*;
use relm4::ComponentSender;

use crate::state::{SettingsModel, SettingsMsg};

use super::common::switch_row;

pub(super) fn build_general_card(
    sender: &ComponentSender<SettingsModel>,
) -> (gtk4::Box, gtk4::Button, gtk4::Switch) {
    let recv_card = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(10)
        .build();
    recv_card.add_css_class("keynear-card");

    let folder_row = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(12)
        .build();
    folder_row.add_css_class("kn-setting-row");
    let folder_text = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(2)
        .hexpand(true)
        .build();
    let folder_title = gtk4::Label::builder()
        .label("Save folder")
        .xalign(0.0)
        .build();
    folder_title.add_css_class("keynear-description");
    let folder_desc = gtk4::Label::builder()
        .label("Where received files are saved")
        .xalign(0.0)
        .build();
    folder_desc.add_css_class("keynear-caption");
    folder_text.append(&folder_title);
    folder_text.append(&folder_desc);

    let save_folder_btn = gtk4::Button::with_label("~/Downloads/Keynear");
    save_folder_btn.add_css_class("kn-tonal-action-btn");
    {
        let s = sender.clone();
        save_folder_btn.connect_clicked(move |_| s.input(SettingsMsg::PickIncomingDirectory));
    }
    folder_row.append(&folder_text);
    folder_row.append(&save_folder_btn);
    recv_card.append(&folder_row);

    let (global_aa_row, global_auto_accept_switch) = switch_row(
        "Auto-accept from paired devices",
        "Save files from paired devices without asking",
        {
            let s = sender.clone();
            move |enabled| s.input(SettingsMsg::SetGlobalAutoAccept(enabled))
        },
    );
    recv_card.append(&global_aa_row);
    (recv_card, save_folder_btn, global_auto_accept_switch)
}
