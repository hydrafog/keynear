mod audio;
mod clipboard;
mod common;
mod general;

use gtk4::prelude::*;
use relm4::ComponentSender;

use crate::state::SettingsModel;

use super::SettingsTabParts;

pub(crate) fn build_settings_tab(sender: &ComponentSender<SettingsModel>) -> SettingsTabParts {
    let page = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(12)
        .margin_start(16)
        .margin_end(16)
        .margin_top(4)
        .margin_bottom(24)
        .build();

    page.append(&common::section_heading("GENERAL & STORAGE"));
    let (recv_card, save_folder_btn, global_auto_accept_switch) =
        general::build_general_card(sender);
    page.append(&recv_card);

    page.append(&common::section_heading("CLIPBOARD"));
    let clipboard = clipboard::build_clipboard_card(sender);
    page.append(&clipboard.card);

    page.append(&common::section_heading("AUDIO & NOTIFICATIONS"));
    let (audio_card, audio_handoff_switch, notify_auto_accept_switch) =
        audio::build_audio_card(sender);
    page.append(&audio_card);

    page.append(&common::section_heading("APPEARANCE"));
    let (appear_card, dark_theme_switch) = audio::build_appearance_card(sender);
    page.append(&appear_card);

    let root = gtk4::ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vexpand(true)
        .child(&page)
        .build();

    SettingsTabParts {
        root,
        save_folder_btn,
        global_auto_accept_switch,
        clipboard_sync_switch: clipboard.sync_switch,
        clipboard_images_switch: clipboard.images_switch,
        otp_passthrough_switch: clipboard.otp_switch,
        mime_count_label: clipboard.count_label,
        mime_chevron_down: clipboard.chevron_down,
        mime_chevron_up: clipboard.chevron_up,
        mime_editor_box: clipboard.editor_box,
        mime_chips_box: clipboard.chips_box,
        mime_entry: clipboard.entry,
        audio_handoff_switch,
        notify_auto_accept_switch,
        dark_theme_switch,
    }
}
