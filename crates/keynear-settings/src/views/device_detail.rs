mod build;

use gtk4::prelude::*;

use crate::state::{role_label, SettingsModel, SubScreen};
use crate::views::components::sync_switch;

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

pub(crate) use build::build_device_detail;

fn update_detail_identity(
    parts: &DeviceDetailParts,
    model: &SettingsModel,
    dev: &keynear_protocol::PairedDevice,
    online: bool,
) {
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
}

fn update_detail_values(
    parts: &DeviceDetailParts,
    model: &SettingsModel,
    dev: &keynear_protocol::PairedDevice,
) {
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

pub(crate) fn update_device_detail(parts: &DeviceDetailParts, model: &SettingsModel) {
    let Some(SubScreen::DeviceDetail(_)) = model.sub_screen else {
        return;
    };
    let Some(dev) = model.selected_detail_device() else {
        return;
    };
    let online = !dev.last_lan_address.is_empty();
    update_detail_identity(parts, model, dev, online);
    update_detail_values(parts, model, dev);
}
