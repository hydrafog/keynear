use gtk4::pango::EllipsizeMode;
use gtk4::prelude::*;
use keynear_protocol::{DeviceRole, PairedDevice};
use relm4::ComponentSender;

use crate::state::{
    format_bytes, role_label, SendItemKind, SendQueueItem, SettingsModel, SettingsMsg,
};
use crate::views::components::{clear_box_children, lucide_icon};

use super::LucideIcon;

fn queue_badge(item: &SendQueueItem) -> gtk4::Box {
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

    badge
}

fn queue_info(
    item: &SendQueueItem,
    sender: &ComponentSender<SettingsModel>,
) -> (gtk4::Box, gtk4::Button) {
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
        remove_btn.connect_clicked(move |_| s.input(SettingsMsg::RemoveSendQueueItem(id.clone())));
    }

    (col, remove_btn)
}

fn queue_row(item: &SendQueueItem, sender: &ComponentSender<SettingsModel>) -> gtk4::Box {
    let row = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(10)
        .build();
    row.add_css_class("kn-queue-row");
    let (info, remove_btn) = queue_info(item, sender);
    row.append(&queue_badge(item));
    row.append(&info);
    row.append(&remove_btn);
    row
}

pub(crate) fn rebuild_send_queue_rows(
    container: &gtk4::Box,
    queue: &[SendQueueItem],
    sender: &ComponentSender<SettingsModel>,
) {
    clear_box_children(container);
    for item in queue {
        container.append(&queue_row(item, sender));
    }
}

pub struct SendTargetsParams<'a> {
    pub devices: &'a [PairedDevice],
    pub sending_in_flight: bool,
    pub has_queue: bool,
    pub sender: &'a ComponentSender<SettingsModel>,
}

fn target_badge(dev: &PairedDevice, online: bool) -> gtk4::Box {
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
    badge
}

fn target_info(dev: &PairedDevice, online: bool) -> gtk4::Box {
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
    info
}

fn target_send_btn(dev: &PairedDevice, idx: usize, params: &SendTargetsParams<'_>) -> gtk4::Button {
    let send_btn = gtk4::Button::with_label(if params.sending_in_flight {
        "Sending..."
    } else {
        "Send"
    });
    send_btn.set_sensitive(!params.sending_in_flight && params.has_queue);
    send_btn.add_css_class(if idx == 0 {
        "kn-loading-btn"
    } else {
        "kn-tonal-action-btn"
    });
    let s = params.sender.clone();
    let id = dev.device_id.clone();
    send_btn.connect_clicked(move |_| s.input(SettingsMsg::SendQueueToPeer(id.clone())));
    send_btn
}

fn target_row(dev: &PairedDevice, idx: usize, params: &SendTargetsParams<'_>) -> gtk4::Box {
    let row = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(12)
        .build();
    row.add_css_class("kn-sheet-row");
    let online = !dev.last_lan_address.is_empty();
    row.append(&target_badge(dev, online));
    row.append(&target_info(dev, online));
    row.append(&target_send_btn(dev, idx, params));
    row
}

pub(crate) fn rebuild_send_targets(container: &gtk4::Box, params: SendTargetsParams<'_>) {
    clear_box_children(container);
    if params.devices.is_empty() {
        let empty = gtk4::Label::builder()
            .label("No paired devices")
            .xalign(0.0)
            .build();
        empty.add_css_class("keynear-caption");
        container.append(&empty);
        return;
    }

    for (idx, dev) in params.devices.iter().enumerate() {
        container.append(&target_row(dev, idx, &params));
    }
}
