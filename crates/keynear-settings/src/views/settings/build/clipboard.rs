use gtk4::prelude::*;
use relm4::ComponentSender;

use crate::state::{SettingsModel, SettingsMsg};
use crate::views::components::lucide_icon;

use super::common::switch_row;
use crate::views::components::LucideIcon;

pub(super) struct ClipboardCardParts {
    pub(super) card: gtk4::Box,
    pub(super) sync_switch: gtk4::Switch,
    pub(super) images_switch: gtk4::Switch,
    pub(super) otp_switch: gtk4::Switch,
    pub(super) count_label: gtk4::Label,
    pub(super) chevron_down: gtk4::DrawingArea,
    pub(super) chevron_up: gtk4::DrawingArea,
    pub(super) editor_box: gtk4::Box,
    pub(super) chips_box: gtk4::Box,
    pub(super) entry: gtk4::Entry,
}

pub(super) struct MimeEditorParts {
    count_label: gtk4::Label,
    chevron_down: gtk4::DrawingArea,
    chevron_up: gtk4::DrawingArea,
    editor_box: gtk4::Box,
    chips_box: gtk4::Box,
    entry: gtk4::Entry,
}

fn build_mime_header(
    card: &gtk4::Box,
    sender: &ComponentSender<SettingsModel>,
) -> (gtk4::Label, gtk4::DrawingArea, gtk4::DrawingArea) {
    let mime_header_btn = gtk4::Button::new();
    mime_header_btn.add_css_class("kn-setting-row-btn");
    let mime_row = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(10)
        .build();
    let mime_text = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(2)
        .hexpand(true)
        .build();
    let mime_title = gtk4::Label::builder()
        .label("Ignored clipboard formats")
        .xalign(0.0)
        .build();
    mime_title.add_css_class("keynear-description");
    let mime_sub = gtk4::Label::builder()
        .label("Formats that are never shared")
        .xalign(0.0)
        .build();
    mime_sub.add_css_class("keynear-caption");
    mime_text.append(&mime_title);
    mime_text.append(&mime_sub);

    let mime_count_label = gtk4::Label::builder().label("0").build();
    mime_count_label.add_css_class("kn-count-badge");
    let mime_chevron_down = lucide_icon(LucideIcon::ChevronDown, 20);
    let mime_chevron_up = lucide_icon(LucideIcon::ChevronUp, 20);
    mime_chevron_up.set_visible(false);

    mime_row.append(&mime_text);
    mime_row.append(&mime_count_label);
    mime_row.append(&mime_chevron_down);
    mime_row.append(&mime_chevron_up);
    mime_header_btn.set_child(Some(&mime_row));
    {
        let s = sender.clone();
        mime_header_btn.connect_clicked(move |_| s.input(SettingsMsg::ToggleMimeExpanded));
    }
    card.append(&mime_header_btn);

    (mime_count_label, mime_chevron_down, mime_chevron_up)
}

fn build_mime_input(
    sender: &ComponentSender<SettingsModel>,
) -> (gtk4::Box, gtk4::Box, gtk4::Entry) {
    let mime_editor_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(8)
        .visible(false)
        .build();
    let mime_chips_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(6)
        .build();
    let mime_input_row = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(8)
        .build();
    let mime_entry = gtk4::Entry::builder()
        .placeholder_text("application/x-secret")
        .hexpand(true)
        .build();
    mime_entry.add_css_class("kn-recessed-entry");
    {
        let s = sender.clone();
        mime_entry
            .connect_changed(move |e| s.input(SettingsMsg::UpdateMimeInput(e.text().to_string())));
    }
    {
        let s = sender.clone();
        mime_entry.connect_activate(move |_| s.input(SettingsMsg::AddIgnoredMime));
    }
    let add_mime_btn = gtk4::Button::with_label("Add");
    add_mime_btn.add_css_class("kn-loading-btn");
    {
        let s = sender.clone();
        add_mime_btn.connect_clicked(move |_| s.input(SettingsMsg::AddIgnoredMime));
    }
    mime_input_row.append(&mime_entry);
    mime_input_row.append(&add_mime_btn);
    mime_editor_box.append(&mime_chips_box);
    mime_editor_box.append(&mime_input_row);

    (mime_editor_box, mime_chips_box, mime_entry)
}

pub(super) fn build_mime_editor(
    card: &gtk4::Box,
    sender: &ComponentSender<SettingsModel>,
) -> MimeEditorParts {
    let (count_label, chevron_down, chevron_up) = build_mime_header(card, sender);
    let (editor_box, chips_box, entry) = build_mime_input(sender);
    MimeEditorParts {
        count_label,
        chevron_down,
        chevron_up,
        editor_box,
        chips_box,
        entry,
    }
}

pub(super) fn build_clipboard_card(sender: &ComponentSender<SettingsModel>) -> ClipboardCardParts {
    let clip_card = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(10)
        .build();
    clip_card.add_css_class("keynear-card");

    let (clip_row, clipboard_sync_switch) =
        switch_row("Clipboard sync", "Keep clipboard text in sync", {
            let s = sender.clone();
            move |enabled| s.input(SettingsMsg::ToggleClipboardSync(enabled))
        });
    let (img_row, clipboard_images_switch) = switch_row(
        "Sync clipboard images",
        "Include images in clipboard sync",
        {
            let s = sender.clone();
            move |enabled| s.input(SettingsMsg::ToggleClipboardImages(enabled))
        },
    );
    let (otp_row, otp_passthrough_switch) = switch_row(
        "2FA code pass-through",
        "Pass verification codes to the computer",
        {
            let s = sender.clone();
            move |enabled| s.input(SettingsMsg::ToggleOtpPassthrough(enabled))
        },
    );

    clip_card.append(&clip_row);
    clip_card.append(&img_row);
    clip_card.append(&otp_row);

    let mime = build_mime_editor(&clip_card, sender);

    clip_card.append(&mime.editor_box);
    ClipboardCardParts {
        card: clip_card,
        sync_switch: clipboard_sync_switch,
        images_switch: clipboard_images_switch,
        otp_switch: otp_passthrough_switch,
        count_label: mime.count_label,
        chevron_down: mime.chevron_down,
        chevron_up: mime.chevron_up,
        editor_box: mime.editor_box,
        chips_box: mime.chips_box,
        entry: mime.entry,
    }
}
