use gtk4::prelude::*;
use relm4::ComponentSender;

use crate::state::{SettingsModel, SettingsMsg};
use crate::views::components::lucide_icon;

use super::{LucideIcon, SendTabParts};

fn selection_tile(
    icon_kind: LucideIcon,
    label: &str,
    on_click: impl Fn() + 'static,
) -> gtk4::Button {
    let btn = gtk4::Button::builder().hexpand(true).build();
    btn.add_css_class("kn-selection-tile");

    let inner = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(12)
        .halign(gtk4::Align::Center)
        .valign(gtk4::Align::Center)
        .build();

    let icon = lucide_icon(icon_kind, 32);
    icon.set_halign(gtk4::Align::Center);
    icon.add_css_class("kn-tile-icon");

    let title = gtk4::Label::builder()
        .label(label)
        .halign(gtk4::Align::Center)
        .build();
    title.add_css_class("kn-tile-label");

    inner.append(&icon);
    inner.append(&title);
    btn.set_child(Some(&inner));
    btn.connect_clicked(move |_| on_click());
    btn
}

fn build_selection_grid(
    sender: &ComponentSender<SettingsModel>,
) -> (gtk4::Label, gtk4::Grid, gtk4::Button) {
    let sel_heading = gtk4::Label::builder()
        .label("SELECTION")
        .xalign(0.0)
        .build();
    sel_heading.add_css_class("kn-section-heading");

    let grid = gtk4::Grid::builder()
        .row_spacing(8)
        .column_spacing(8)
        .column_homogeneous(true)
        .build();

    let file_tile = selection_tile(LucideIcon::FileText, "File", {
        let s = sender.clone();
        move || s.input(SettingsMsg::PickSendFiles)
    });
    let folder_tile = selection_tile(LucideIcon::Folder, "Folder", {
        let s = sender.clone();
        move || s.input(SettingsMsg::PickSendFolder)
    });
    let clipboard_tile = selection_tile(LucideIcon::Clipboard, "Clipboard", {
        let s = sender.clone();
        move || s.input(SettingsMsg::StageClipboardForSend)
    });
    let text_tile = selection_tile(LucideIcon::Plus, "Text", {
        let s = sender.clone();
        move || s.input(SettingsMsg::ToggleTextComposer)
    });

    grid.attach(&file_tile, 0, 0, 1, 1);
    grid.attach(&folder_tile, 1, 0, 1, 1);
    grid.attach(&clipboard_tile, 0, 1, 1, 1);
    grid.attach(&text_tile, 1, 1, 1, 1);
    (sel_heading, grid, text_tile)
}

fn build_text_composer(sender: &ComponentSender<SettingsModel>) -> (gtk4::Box, gtk4::Entry) {
    let text_composer_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(8)
        .visible(false)
        .build();
    text_composer_box.add_css_class("keynear-card");

    let composer_row = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(8)
        .build();

    let text_entry = gtk4::Entry::builder()
        .placeholder_text("Paste URL or note")
        .hexpand(true)
        .build();
    text_entry.add_css_class("kn-recessed-entry");
    {
        let s = sender.clone();
        text_entry
            .connect_changed(move |e| s.input(SettingsMsg::UpdateTextDraft(e.text().to_string())));
    }
    {
        let s = sender.clone();
        text_entry.connect_activate(move |_| s.input(SettingsMsg::StageTextSnippet));
    }

    let add_text_btn = gtk4::Button::with_label("Add");
    add_text_btn.add_css_class("kn-loading-btn");
    {
        let s = sender.clone();
        add_text_btn.connect_clicked(move |_| s.input(SettingsMsg::StageTextSnippet));
    }

    composer_row.append(&text_entry);
    composer_row.append(&add_text_btn);
    text_composer_box.append(&composer_row);
    (text_composer_box, text_entry)
}

fn build_queue_card(
    sender: &ComponentSender<SettingsModel>,
) -> (gtk4::Box, gtk4::Label, gtk4::Button, gtk4::Box) {
    let queue_card = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(10)
        .visible(false)
        .build();
    queue_card.add_css_class("keynear-card");

    let queue_header = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(8)
        .build();

    let queue_heading = gtk4::Label::builder()
        .label("0 items (0 B)")
        .xalign(0.0)
        .hexpand(true)
        .build();
    queue_heading.add_css_class("keynear-section-title");

    let clear_btn = gtk4::Button::with_label("Clear all");
    clear_btn.add_css_class("kn-error-action-btn");
    {
        let s = sender.clone();
        clear_btn.connect_clicked(move |_| s.input(SettingsMsg::ClearSendPaths));
    }

    queue_header.append(&queue_heading);
    queue_header.append(&clear_btn);
    queue_card.append(&queue_header);

    let queue_list_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(6)
        .build();
    queue_card.append(&queue_list_box);
    (queue_card, queue_heading, clear_btn, queue_list_box)
}

fn build_error_banner() -> (gtk4::Box, gtk4::Label) {
    let error_banner = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(8)
        .visible(false)
        .build();
    error_banner.add_css_class("keynear-error");
    let error_label = gtk4::Label::builder()
        .label("")
        .xalign(0.0)
        .wrap(true)
        .build();
    error_label.add_css_class("keynear-description");
    error_banner.append(&error_label);
    (error_banner, error_label)
}

fn build_targets() -> (gtk4::Label, gtk4::Box) {
    let targets_heading = gtk4::Label::builder().label("SEND TO").xalign(0.0).build();
    targets_heading.add_css_class("kn-section-heading");
    let targets_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(8)
        .build();
    targets_box.add_css_class("keynear-card");
    (targets_heading, targets_box)
}

pub(crate) fn build_send_tab(sender: &ComponentSender<SettingsModel>) -> SendTabParts {
    let page = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(16)
        .margin_start(16)
        .margin_end(16)
        .margin_top(4)
        .margin_bottom(24)
        .build();

    let (sel_heading, grid, text_tile) = build_selection_grid(sender);
    page.append(&sel_heading);
    page.append(&grid);

    let (text_composer_box, text_entry) = build_text_composer(sender);
    page.append(&text_composer_box);

    let (queue_card, queue_heading, clear_btn, queue_list_box) = build_queue_card(sender);
    page.append(&queue_card);

    let (error_banner, error_label) = build_error_banner();
    page.append(&error_banner);

    let (targets_heading, targets_box) = build_targets();
    page.append(&targets_heading);
    page.append(&targets_box);

    let root = gtk4::ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vexpand(true)
        .child(&page)
        .build();

    SendTabParts {
        root,
        text_tile,
        text_composer_box,
        text_entry,
        queue_card,
        queue_heading,
        clear_btn,
        queue_list_box,
        error_banner,
        error_label,
        targets_box,
    }
}
