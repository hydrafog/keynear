use gtk4::prelude::*;
use relm4::ComponentSender;

use crate::state::{ActiveTab, SettingsModel};
use crate::views::components::sync_switch;
use crate::views::device_detail::update_device_detail;
use crate::views::settings::rebuild_mime_chips;

use super::SettingsWidgets;

fn update_settings_switches(model: &SettingsModel, widgets: &mut SettingsWidgets) {
    widgets
        .settings
        .save_folder_btn
        .set_label(&model.incoming_directory);
    sync_switch(
        &widgets.settings.global_auto_accept_switch,
        model.config.auto_accept_from_paired,
    );
    sync_switch(
        &widgets.settings.clipboard_sync_switch,
        model.config.clipboard_sync_enabled,
    );
    sync_switch(
        &widgets.settings.clipboard_images_switch,
        model.config.clipboard_images_enabled,
    );
    sync_switch(
        &widgets.settings.otp_passthrough_switch,
        model.config.otp_passthrough_enabled,
    );
}

fn update_mime_editor(
    model: &SettingsModel,
    widgets: &mut SettingsWidgets,
    sender: &ComponentSender<SettingsModel>,
) {
    widgets
        .settings
        .mime_count_label
        .set_label(&model.config.ignored_mime_types.len().to_string());
    widgets
        .settings
        .mime_chevron_down
        .set_visible(!model.mime_editor_open);
    widgets
        .settings
        .mime_chevron_up
        .set_visible(model.mime_editor_open);
    widgets
        .settings
        .mime_editor_box
        .set_visible(model.mime_editor_open);
    if widgets.settings.mime_entry.text().as_str() != model.new_mime_input
        && !widgets.settings.mime_entry.has_focus()
    {
        widgets.settings.mime_entry.set_text(&model.new_mime_input);
    }
    rebuild_mime_chips(
        &widgets.settings.mime_chips_box,
        &model.config.ignored_mime_types,
        sender,
    );
    sync_switch(
        &widgets.settings.audio_handoff_switch,
        model.config.audio_handoff_enabled,
    );
    sync_switch(
        &widgets.settings.notify_auto_accept_switch,
        model.config.notify_auto_accept_complete,
    );
    sync_switch(&widgets.settings.dark_theme_switch, model.dark_theme);
    update_device_detail(&widgets.detail, model);
}

pub(super) fn update_snackbar(model: &SettingsModel, widgets: &mut SettingsWidgets) {
    if let Some(ref snack) = model.snackbar_message {
        widgets.snackbar.label.set_label(snack);
        widgets.snackbar.root.set_visible(true);
    } else {
        widgets.snackbar.root.set_visible(false);
    }
}

pub(super) fn update_bottom_nav(model: &SettingsModel, widgets: &mut SettingsWidgets) {
    let navs = [
        (
            &widgets.bottom_nav.receive_pill,
            &widgets.bottom_nav.receive_text,
            model.sub_screen.is_none() && model.active_tab == ActiveTab::Receive,
        ),
        (
            &widgets.bottom_nav.send_pill,
            &widgets.bottom_nav.send_text,
            model.sub_screen.is_none() && model.active_tab == ActiveTab::Send,
        ),
        (
            &widgets.bottom_nav.settings_pill,
            &widgets.bottom_nav.settings_text,
            model.sub_screen.is_none() && model.active_tab == ActiveTab::Settings,
        ),
    ];
    for (pill, txt, active) in navs {
        pill.remove_css_class("kn-nav-pill-active");
        pill.remove_css_class("kn-nav-pill-inactive");
        pill.add_css_class(if active {
            "kn-nav-pill-active"
        } else {
            "kn-nav-pill-inactive"
        });
        txt.remove_css_class("kn-nav-text-active");
        txt.remove_css_class("keynear-caption");
        txt.add_css_class(if active {
            "kn-nav-text-active"
        } else {
            "keynear-caption"
        });
    }
}

pub(super) fn update_settings(
    model: &SettingsModel,
    widgets: &mut SettingsWidgets,
    sender: &ComponentSender<SettingsModel>,
) {
    update_settings_switches(model, widgets);
    update_mime_editor(model, widgets, sender);
    update_device_detail(&widgets.detail, model);
}
