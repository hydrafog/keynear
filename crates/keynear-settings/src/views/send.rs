use gtk4::pango::EllipsizeMode;
use gtk4::prelude::*;
use keynear_protocol::{DeviceRole, PairedDevice};
use relm4::ComponentSender;

use crate::state::{
    format_bytes, role_label, SendItemKind, SendQueueItem, SettingsModel, SettingsMsg,
};
use crate::views::components::{clear_box_children, lucide_icon, LucideIcon};

pub(crate) struct SendTabParts {
    pub(crate) root: gtk4::ScrolledWindow,
    pub(crate) text_tile: gtk4::Button,
    pub(crate) text_composer_box: gtk4::Box,
    pub(crate) text_entry: gtk4::Entry,
    pub(crate) queue_card: gtk4::Box,
    pub(crate) queue_heading: gtk4::Label,
    pub(crate) clear_btn: gtk4::Button,
    pub(crate) queue_list_box: gtk4::Box,
    pub(crate) error_banner: gtk4::Box,
    pub(crate) error_label: gtk4::Label,
    pub(crate) targets_box: gtk4::Box,
}

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

pub(crate) fn build_send_tab(sender: &ComponentSender<SettingsModel>) -> SendTabParts {
    let page = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(16)
        .margin_start(16)
        .margin_end(16)
        .margin_top(4)
        .margin_bottom(24)
        .build();

    let sel_heading = gtk4::Label::builder()
        .label("SELECTION")
        .xalign(0.0)
        .build();
    sel_heading.add_css_class("kn-section-heading");
    page.append(&sel_heading);

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
    page.append(&grid);

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
    page.append(&text_composer_box);

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
    page.append(&queue_card);

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
    page.append(&error_banner);

    let targets_heading = gtk4::Label::builder().label("SEND TO").xalign(0.0).build();
    targets_heading.add_css_class("kn-section-heading");
    page.append(&targets_heading);

    let targets_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(8)
        .build();
    targets_box.add_css_class("keynear-card");
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

pub(crate) fn rebuild_send_queue_rows(
    container: &gtk4::Box,
    queue: &[SendQueueItem],
    sender: &ComponentSender<SettingsModel>,
) {
    clear_box_children(container);
    for item in queue {
        let row = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(10)
            .build();
        row.add_css_class("kn-queue-row");

        let icon_kind = match item.kind {
            SendItemKind::Folder => LucideIcon::Folder,
            SendItemKind::Clipboard => LucideIcon::Clipboard,
            SendItemKind::Text => LucideIcon::Plus,
            SendItemKind::File => LucideIcon::FileText,
        };
        let badge = gtk4::Box::builder()
            .width_request(36)
            .height_request(36)
            .halign(gtk4::Align::Center)
            .valign(gtk4::Align::Center)
            .build();
        badge.add_css_class("kn-loading-tonal-badge");
        let icon = lucide_icon(icon_kind, 16);
        icon.set_hexpand(true);
        icon.set_vexpand(true);
        icon.set_halign(gtk4::Align::Center);
        icon.set_valign(gtk4::Align::Center);
        badge.append(&icon);

        let col = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(1)
            .hexpand(true)
            .build();
        let name = gtk4::Label::builder()
            .label(&item.name)
            .xalign(0.0)
            .ellipsize(EllipsizeMode::End)
            .build();
        name.add_css_class("keynear-description");

        let size_lbl = gtk4::Label::builder()
            .label(format_bytes(item.size_bytes))
            .xalign(0.0)
            .build();
        size_lbl.add_css_class("keynear-caption");

        col.append(&name);
        col.append(&size_lbl);

        let remove_icon = lucide_icon(LucideIcon::X, 16);
        let remove_btn = gtk4::Button::builder()
            .child(&remove_icon)
            .tooltip_text("Remove")
            .build();
        remove_btn.add_css_class("kn-error-action-btn");
        {
            let s = sender.clone();
            let id = item.id.clone();
            remove_btn
                .connect_clicked(move |_| s.input(SettingsMsg::RemoveSendQueueItem(id.clone())));
        }

        row.append(&badge);
        row.append(&col);
        row.append(&remove_btn);
        container.append(&row);
    }
}

pub(crate) fn rebuild_send_targets(
    container: &gtk4::Box,
    devices: &[PairedDevice],
    sending_in_flight: bool,
    has_queue: bool,
    sender: &ComponentSender<SettingsModel>,
) {
    clear_box_children(container);
    if devices.is_empty() {
        let empty = gtk4::Label::builder()
            .label("No paired devices")
            .xalign(0.0)
            .build();
        empty.add_css_class("keynear-caption");
        container.append(&empty);
        return;
    }

    for (idx, dev) in devices.iter().enumerate() {
        let row = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(12)
            .build();
        row.add_css_class("kn-sheet-row");

        let online = !dev.last_lan_address.is_empty();
        let badge = gtk4::Box::builder()
            .width_request(48)
            .height_request(48)
            .halign(gtk4::Align::Center)
            .valign(gtk4::Align::Center)
            .build();
        badge.add_css_class(if online {
            "kn-loading-tonal-badge"
        } else {
            "kn-surface-tonal-badge"
        });

        let icon = lucide_icon(
            match dev.device_role {
                DeviceRole::Desktop => LucideIcon::Laptop,
                DeviceRole::Mobile => LucideIcon::Smartphone,
            },
            20,
        );
        icon.set_hexpand(true);
        icon.set_vexpand(true);
        icon.set_halign(gtk4::Align::Center);
        icon.set_valign(gtk4::Align::Center);
        badge.append(&icon);

        let info = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(2)
            .hexpand(true)
            .build();
        let name = gtk4::Label::builder()
            .label(&dev.display_name)
            .xalign(0.0)
            .ellipsize(EllipsizeMode::End)
            .build();
        name.add_css_class("keynear-description");

        let sub = gtk4::Label::builder()
            .label(format!(
                "{} · {}",
                role_label(dev.device_role),
                if online {
                    "Wi-Fi direct"
                } else {
                    "Bluetooth wake"
                }
            ))
            .xalign(0.0)
            .build();
        sub.add_css_class("keynear-caption");

        info.append(&name);
        info.append(&sub);

        let send_btn = gtk4::Button::with_label(if sending_in_flight {
            "Sending..."
        } else {
            "Send"
        });
        send_btn.set_sensitive(!sending_in_flight && has_queue);
        send_btn.add_css_class(if idx == 0 {
            "kn-loading-btn"
        } else {
            "kn-tonal-action-btn"
        });
        {
            let s = sender.clone();
            let id = dev.device_id.clone();
            send_btn.connect_clicked(move |_| s.input(SettingsMsg::SendQueueToPeer(id.clone())));
        }

        row.append(&badge);
        row.append(&info);
        row.append(&send_btn);
        container.append(&row);
    }
}
