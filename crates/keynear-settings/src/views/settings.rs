mod build;

use gtk4::pango::EllipsizeMode;
use gtk4::prelude::*;
use relm4::ComponentSender;

use crate::state::{SettingsModel, SettingsMsg};
use crate::views::components::{clear_box_children, lucide_icon, LucideIcon};

pub(crate) use build::build_settings_tab;

pub(crate) struct SettingsTabParts {
    pub(crate) root: gtk4::ScrolledWindow,
    pub(crate) save_folder_btn: gtk4::Button,
    pub(crate) global_auto_accept_switch: gtk4::Switch,
    pub(crate) clipboard_sync_switch: gtk4::Switch,
    pub(crate) clipboard_images_switch: gtk4::Switch,
    pub(crate) otp_passthrough_switch: gtk4::Switch,
    pub(crate) mime_count_label: gtk4::Label,
    pub(crate) mime_chevron_down: gtk4::DrawingArea,
    pub(crate) mime_chevron_up: gtk4::DrawingArea,
    pub(crate) mime_editor_box: gtk4::Box,
    pub(crate) mime_chips_box: gtk4::Box,
    pub(crate) mime_entry: gtk4::Entry,
    pub(crate) audio_handoff_switch: gtk4::Switch,
    pub(crate) notify_auto_accept_switch: gtk4::Switch,
    pub(crate) dark_theme_switch: gtk4::Switch,
}

pub(crate) fn rebuild_mime_chips(
    container: &gtk4::Box,
    mimes: &[String],
    sender: &ComponentSender<SettingsModel>,
) {
    clear_box_children(container);
    for mime in mimes {
        let row = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(8)
            .build();
        row.add_css_class("kn-mime-row");

        let lbl = gtk4::Label::builder()
            .label(mime)
            .xalign(0.0)
            .hexpand(true)
            .ellipsize(EllipsizeMode::End)
            .build();
        lbl.add_css_class("keynear-caption");

        let remove_icon = lucide_icon(LucideIcon::X, 16);
        let remove_btn = gtk4::Button::builder()
            .child(&remove_icon)
            .tooltip_text("Remove MIME rule")
            .build();
        remove_btn.add_css_class("kn-error-action-btn");
        {
            let s = sender.clone();
            let val = mime.clone();
            remove_btn
                .connect_clicked(move |_| s.input(SettingsMsg::RemoveIgnoredMime(val.clone())));
        }

        row.append(&lbl);
        row.append(&remove_btn);
        container.append(&row);
    }
}
