use gtk4::gdk::Display;
use gtk4::pango::EllipsizeMode;
use gtk4::prelude::*;
use relm4::ComponentSender;

use crate::state::{ActiveTab, SettingsModel, SettingsMsg};

pub(crate) const STYLE_CSS: &str = include_str!("../../assets/style.css");

pub(crate) fn install_css(window: &gtk4::Window, dark_theme: bool) {
    let provider = gtk4::CssProvider::new();
    provider.load_from_data(STYLE_CSS);
    if let Some(display) = Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
    if dark_theme {
        window.add_css_class("dark");
    }
}

pub(crate) struct TopBarParts {
    pub(crate) root: gtk4::Box,
    pub(crate) back_btn: gtk4::Button,
    pub(crate) title_label: gtk4::Label,
}

pub(crate) fn build_top_bar(sender: &ComponentSender<SettingsModel>) -> TopBarParts {
    let root = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(12)
        .build();
    root.add_css_class("kn-topbar");

    let back_btn = gtk4::Button::builder()
        .icon_name("go-previous-symbolic")
        .visible(false)
        .width_request(48)
        .height_request(48)
        .tooltip_text("Back")
        .build();
    back_btn.add_css_class("kn-back-btn");
    {
        let s = sender.clone();
        back_btn.connect_clicked(move |_| s.input(SettingsMsg::NavigateBack));
    }

    let title_label = gtk4::Label::builder()
        .label("Keynear")
        .xalign(0.0)
        .hexpand(true)
        .ellipsize(EllipsizeMode::End)
        .build();
    title_label.add_css_class("keynear-title");

    root.append(&back_btn);
    root.append(&title_label);

    TopBarParts {
        root,
        back_btn,
        title_label,
    }
}

pub(crate) struct StatusBannerParts {
    pub(crate) root: gtk4::Box,
    pub(crate) label: gtk4::Label,
}

pub(crate) fn build_status_banner(sender: &ComponentSender<SettingsModel>) -> StatusBannerParts {
    let root = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(8)
        .margin_start(16)
        .margin_end(16)
        .margin_bottom(8)
        .visible(false)
        .build();
    root.add_css_class("keynear-warning");

    let label = gtk4::Label::builder()
        .label("")
        .xalign(0.0)
        .wrap(true)
        .hexpand(true)
        .build();
    label.add_css_class("keynear-description");

    let retry_btn = gtk4::Button::with_label("Retry");
    retry_btn.add_css_class("kn-tonal-action-btn");
    {
        let s = sender.clone();
        retry_btn.connect_clicked(move |_| s.input(SettingsMsg::RetryDaemon));
    }

    root.append(&label);
    root.append(&retry_btn);

    StatusBannerParts { root, label }
}

pub(crate) struct SnackbarParts {
    pub(crate) root: gtk4::Box,
    pub(crate) label: gtk4::Label,
}

pub(crate) fn build_snackbar() -> SnackbarParts {
    let root = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .halign(gtk4::Align::Center)
        .valign(gtk4::Align::End)
        .margin_bottom(12)
        .margin_start(16)
        .margin_end(16)
        .visible(false)
        .build();
    root.add_css_class("kn-snackbar");

    let label = gtk4::Label::builder().label("").wrap(true).build();
    label.add_css_class("kn-snackbar-text");
    root.append(&label);

    SnackbarParts { root, label }
}

pub(crate) struct BottomNavParts {
    pub(crate) root: gtk4::Box,
    pub(crate) receive_pill: gtk4::Box,
    pub(crate) receive_text: gtk4::Label,
    pub(crate) send_pill: gtk4::Box,
    pub(crate) send_text: gtk4::Label,
    pub(crate) settings_pill: gtk4::Box,
    pub(crate) settings_text: gtk4::Label,
}

fn nav_tab_button(
    icon_name: &str,
    label_text: &str,
    tab: ActiveTab,
    sender: &ComponentSender<SettingsModel>,
) -> (gtk4::Button, gtk4::Box, gtk4::Label) {
    let btn = gtk4::Button::builder().hexpand(true).build();
    btn.add_css_class("kn-nav-tab-btn");

    let col = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(4)
        .halign(gtk4::Align::Center)
        .build();

    let pill = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .halign(gtk4::Align::Center)
        .valign(gtk4::Align::Center)
        .width_request(64)
        .height_request(36)
        .build();
    pill.add_css_class("kn-nav-pill-inactive");

    let icon = gtk4::Image::from_icon_name(icon_name);
    icon.set_pixel_size(18);
    icon.set_hexpand(true);
    icon.set_halign(gtk4::Align::Center);
    icon.set_valign(gtk4::Align::Center);
    pill.append(&icon);

    let txt = gtk4::Label::builder().label(label_text).build();
    txt.add_css_class("keynear-caption");

    col.append(&pill);
    col.append(&txt);
    btn.set_child(Some(&col));

    let s = sender.clone();
    btn.connect_clicked(move |_| s.input(SettingsMsg::SwitchTab(tab)));

    (btn, pill, txt)
}

pub(crate) fn build_bottom_nav(sender: &ComponentSender<SettingsModel>) -> BottomNavParts {
    let root = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(8)
        .build();
    root.add_css_class("kn-bottom-nav");

    let tabs_row = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .homogeneous(true)
        .spacing(8)
        .build();

    let (rec_btn, receive_pill, receive_text) = nav_tab_button(
        "network-wireless-symbolic",
        "Receive",
        ActiveTab::Receive,
        sender,
    );
    let (send_btn, send_pill, send_text) =
        nav_tab_button("document-send-symbolic", "Send", ActiveTab::Send, sender);
    let (set_btn, settings_pill, settings_text) = nav_tab_button(
        "preferences-system-symbolic",
        "Settings",
        ActiveTab::Settings,
        sender,
    );

    tabs_row.append(&rec_btn);
    tabs_row.append(&send_btn);
    tabs_row.append(&set_btn);

    let gesture_pill = gtk4::Box::builder()
        .halign(gtk4::Align::Center)
        .width_request(128)
        .height_request(5)
        .build();
    gesture_pill.add_css_class("kn-gesture-pill");

    root.append(&tabs_row);
    root.append(&gesture_pill);

    BottomNavParts {
        root,
        receive_pill,
        receive_text,
        send_pill,
        send_text,
        settings_pill,
        settings_text,
    }
}
