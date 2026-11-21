use gtk4::prelude::*;
use relm4::ComponentSender;

use crate::state::{SettingsModel, SettingsMsg};
use crate::views::components::lucide_icon;

use super::{LucideIcon, ReceiveSheetsParts};

fn build_transfers_card() -> (gtk4::Box, gtk4::Box, gtk4::Label, gtk4::Box) {
    let transfers_card = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(8)
        .margin_start(16)
        .margin_end(16)
        .margin_bottom(8)
        .visible(false)
        .build();
    transfers_card.add_css_class("keynear-card");

    let transfers_heading = gtk4::Label::builder()
        .label("ACTIVE TRANSFERS")
        .xalign(0.0)
        .build();
    transfers_heading.add_css_class("kn-section-heading");

    let transfers_error_banner = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(8)
        .visible(false)
        .build();
    transfers_error_banner.add_css_class("keynear-error");
    let transfers_error_label = gtk4::Label::builder()
        .label("")
        .xalign(0.0)
        .wrap(true)
        .build();
    transfers_error_label.add_css_class("keynear-description");
    transfers_error_banner.append(&transfers_error_label);

    let transfers_list_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(6)
        .build();

    transfers_card.append(&transfers_heading);
    transfers_card.append(&transfers_error_banner);
    transfers_card.append(&transfers_list_box);
    (
        transfers_card,
        transfers_error_banner,
        transfers_error_label,
        transfers_list_box,
    )
}

fn build_sheet_header(
    sender: &ComponentSender<SettingsModel>,
) -> (gtk4::Box, gtk4::Label, gtk4::Label) {
    let sheet_header = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(8)
        .build();

    let sheet_titles = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(2)
        .hexpand(true)
        .build();
    let sheet_title = gtk4::Label::builder()
        .label("Devices in range")
        .xalign(0.0)
        .build();
    sheet_title.add_css_class("keynear-section-title");
    let sheet_count_label = gtk4::Label::builder()
        .label("0 devices")
        .xalign(0.0)
        .build();
    sheet_count_label.add_css_class("keynear-caption");
    sheet_titles.append(&sheet_title);
    sheet_titles.append(&sheet_count_label);

    let sheet_toggle_label = gtk4::Label::builder().label("All devices").build();
    sheet_toggle_label.add_css_class("kn-link-label");
    let sheet_toggle_btn = gtk4::Button::builder().child(&sheet_toggle_label).build();
    sheet_toggle_btn.add_css_class("kn-link-btn");
    {
        let s = sender.clone();
        sheet_toggle_btn.connect_clicked(move |_| s.input(SettingsMsg::ToggleSheetExpanded));
    }

    let refresh_icon = lucide_icon(LucideIcon::RefreshCw, 18);
    let refresh_btn = gtk4::Button::builder()
        .child(&refresh_icon)
        .tooltip_text("Refresh devices and daemon state")
        .build();
    refresh_btn.add_css_class("kn-tonal-action-btn");
    {
        let s = sender.clone();
        refresh_btn.connect_clicked(move |_| s.input(SettingsMsg::RefreshPeers));
    }

    sheet_header.append(&sheet_titles);
    sheet_header.append(&sheet_toggle_btn);
    sheet_header.append(&refresh_btn);
    (sheet_header, sheet_count_label, sheet_toggle_label)
}

pub(crate) fn build_receive_sheets(sender: &ComponentSender<SettingsModel>) -> ReceiveSheetsParts {
    let (transfers_card, transfers_error_banner, transfers_error_label, transfers_list_box) =
        build_transfers_card();

    let sheet_card = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(8)
        .margin_start(16)
        .margin_end(16)
        .build();
    sheet_card.add_css_class("kn-bottom-sheet");

    let handle = gtk4::Box::builder()
        .halign(gtk4::Align::Center)
        .width_request(36)
        .height_request(4)
        .build();
    handle.add_css_class("kn-sheet-handle");
    sheet_card.append(&handle);

    let (sheet_header, sheet_count_label, sheet_toggle_label) = build_sheet_header(sender);
    sheet_card.append(&sheet_header);

    let sheet_list_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(6)
        .build();

    let sheet_scroll = gtk4::ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .min_content_height(112)
        .max_content_height(112)
        .child(&sheet_list_box)
        .build();

    sheet_card.append(&sheet_scroll);

    ReceiveSheetsParts {
        transfers_card,
        transfers_error_banner,
        transfers_error_label,
        transfers_list_box,
        sheet_card,
        sheet_count_label,
        sheet_toggle_label,
        sheet_scroll,
        sheet_list_box,
    }
}
