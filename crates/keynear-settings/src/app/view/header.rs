use gtk4::prelude::*;

use crate::state::{ActiveTab, SettingsModel, SubScreen};

use super::SettingsWidgets;

pub(super) fn update_top_bar(model: &SettingsModel, widgets: &mut SettingsWidgets) {
    let title_str = match model.sub_screen {
        Some(SubScreen::DeviceDetail(_)) => model
            .selected_detail_device()
            .map(|d| d.display_name.as_str())
            .unwrap_or("Device details"),
        None => match model.active_tab {
            ActiveTab::Receive => "Keynear",
            ActiveTab::Send => "Send files",
            ActiveTab::Settings => "Settings",
        },
    };
    widgets.topbar.title_label.set_label(title_str);
    widgets
        .topbar
        .back_btn
        .set_visible(model.sub_screen.is_some());

    let stack_page = match model.sub_screen {
        Some(SubScreen::DeviceDetail(_)) => "deviceDetail",
        None => match model.active_tab {
            ActiveTab::Receive => "receive",
            ActiveTab::Send => "send",
            ActiveTab::Settings => "settings",
        },
    };
    if widgets.main_stack.visible_child_name().as_deref() != Some(stack_page) {
        widgets.main_stack.set_visible_child_name(stack_page);
    }
}

pub(super) fn update_banner(model: &SettingsModel, widgets: &mut SettingsWidgets) {
    if let Some(ref err) = model.error_message {
        widgets.banner.root.remove_css_class("keynear-warning");
        widgets.banner.root.add_css_class("keynear-error");
        widgets.banner.label.set_label(err);
        widgets.banner.root.set_visible(true);
    } else if let Some(ref warn) = model.warning_message {
        widgets.banner.root.remove_css_class("keynear-error");
        widgets.banner.root.add_css_class("keynear-warning");
        widgets.banner.label.set_label(warn);
        widgets.banner.root.set_visible(true);
    } else {
        widgets.banner.root.set_visible(false);
    }
}
