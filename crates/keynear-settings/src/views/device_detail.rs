use gtk4::pango::EllipsizeMode;
use gtk4::prelude::*;
use relm4::ComponentSender;

use crate::state::{role_label, SettingsModel, SettingsMsg, SubScreen};
use crate::views::components::{lucide_icon, sync_switch, LucideIcon};

pub(crate) struct DeviceDetailParts {
    pub(crate) root: gtk4::ScrolledWindow,
    pub(crate) icon_box: gtk4::Box,
    pub(crate) name_label: gtk4::Label,
    pub(crate) status_label: gtk4::Label,
    pub(crate) name_entry: gtk4::Entry,
    pub(crate) auto_accept_switch: gtk4::Switch,
    pub(crate) key_value: gtk4::Label,
    pub(crate) ble_value: gtk4::Label,
    pub(crate) lan_value: gtk4::Label,
    pub(crate) remove_btn: gtk4::Button,
    pub(crate) confirm_box: gtk4::Box,
    pub(crate) confirm_label: gtk4::Label,
}

fn meta_row(label: &str) -> (gtk4::Box, gtk4::Label) {
    let row = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(12)
        .build();
    let key = gtk4::Label::builder()
        .label(label)
        .xalign(0.0)
        .hexpand(true)
        .build();
    key.add_css_class("keynear-caption");
    let val = gtk4::Label::builder()
        .label("")
        .xalign(1.0)
        .ellipsize(EllipsizeMode::End)
        .build();
    val.add_css_class("keynear-caption");
    row.append(&key);
    row.append(&val);
    (row, val)
}

fn current_detail_id(entry: &gtk4::Entry) -> String {
    entry.widget_name().to_string()
}

pub(crate) fn build_device_detail(sender: &ComponentSender<SettingsModel>) -> DeviceDetailParts {
    let page = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(16)
        .margin_start(16)
        .margin_end(16)
        .margin_top(4)
        .margin_bottom(24)
        .build();

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
    page.append(&hero_card);

    let rename_heading = gtk4::Label::builder()
        .label("COMPUTER NAME")
        .xalign(0.0)
        .build();
    rename_heading.add_css_class("kn-section-heading");
    page.append(&rename_heading);

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
    page.append(&rename_card);

    let transfers_heading = gtk4::Label::builder()
        .label("TRANSFERS")
        .xalign(0.0)
        .build();
    transfers_heading.add_css_class("kn-section-heading");
    page.append(&transfers_heading);

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
    page.append(&transfers_card);

    let conn_heading = gtk4::Label::builder()
        .label("CONNECTION")
        .xalign(0.0)
        .build();
    conn_heading.add_css_class("kn-section-heading");
    page.append(&conn_heading);

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
    page.append(&conn_card);

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
    page.append(&remove_btn);

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

pub(crate) fn update_device_detail(parts: &DeviceDetailParts, model: &SettingsModel) {
    let Some(SubScreen::DeviceDetail(_)) = model.sub_screen else {
        return;
    };
    let Some(dev) = model.selected_detail_device() else {
        return;
    };
    let online = !dev.last_lan_address.is_empty();
    parts.icon_box.remove_css_class("kn-loading-tonal-badge");
    parts.icon_box.remove_css_class("keynear-tonal");
    parts.icon_box.add_css_class(if online {
        "kn-loading-tonal-badge"
    } else {
        "keynear-tonal"
    });
    parts.name_label.set_label(&dev.display_name);
    let status_txt = format!(
        "{} · {} ({})",
        role_label(dev.device_role),
        if online {
            "Connected via Wi-Fi"
        } else {
            "Bluetooth standby"
        },
        dev.paired_at
    );
    parts.status_label.set_label(&status_txt);

    if parts.name_entry.widget_name() != dev.device_id {
        parts.name_entry.set_widget_name(&dev.device_id);
        parts.name_entry.set_text(&model.rename_draft);
    } else if parts.name_entry.text().as_str() != model.rename_draft
        && !parts.name_entry.has_focus()
    {
        parts.name_entry.set_text(&model.rename_draft);
    }
    sync_switch(&parts.auto_accept_switch, dev.auto_accept_files);

    let key_prefix: String = dev.static_x25519_pubkey.chars().take(19).collect();
    parts.key_value.set_label(&format!("{key_prefix}…"));
    parts
        .ble_value
        .set_label(if dev.last_ble_address.is_empty() {
            "Unavailable"
        } else {
            &dev.last_ble_address
        });
    parts
        .lan_value
        .set_label(if dev.last_lan_address.is_empty() {
            "Standby"
        } else {
            &dev.last_lan_address
        });

    let confirming = model.pending_unpair_id.as_deref() == Some(dev.device_id.as_str());
    parts.remove_btn.set_visible(!confirming);
    parts.confirm_box.set_visible(confirming);
    if confirming {
        parts.confirm_label.set_label(&format!(
            "Remove {}? You will need to confirm a new 6-digit code on both devices to pair again.",
            dev.display_name
        ));
    }
}
