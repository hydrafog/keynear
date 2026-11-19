pub mod radar;
pub mod sheet;

use std::cell::Cell;
use std::rc::Rc;

use gtk4::prelude::*;
use keynear_protocol::VisibilityMode;
use relm4::ComponentSender;

use crate::state::{SettingsModel, SettingsMsg};
use crate::views::components::{lucide_icon, LucideIcon};

use radar::build_radar_stage;
use sheet::build_receive_sheets;

pub(crate) use radar::rebuild_radar_pins;
pub(crate) use sheet::{rebuild_sheet_rows, rebuild_transfers_rows};

pub(crate) struct ReceiveTabParts {
    pub(crate) root: gtk4::Box,
    pub(crate) vis_hidden_btn: gtk4::Button,
    pub(crate) vis_paired_btn: gtk4::Button,
    pub(crate) vis_everyone_btn: gtk4::Button,
    pub(crate) radar_draw_state: Rc<Cell<(bool, bool)>>,
    pub(crate) radar_canvas: gtk4::DrawingArea,
    pub(crate) self_node_circle: gtk4::Box,
    pub(crate) self_name_label: gtk4::Label,
    pub(crate) self_countdown_label: gtk4::Label,
    pub(crate) radar_pins_overlay: gtk4::Overlay,
    pub(crate) pairing_card: gtk4::Box,
    pub(crate) pairing_peer_label: gtk4::Label,
    pub(crate) pairing_code_label: gtk4::Label,
    pub(crate) pairing_timer_label: gtk4::Label,
    pub(crate) transfers_card: gtk4::Box,
    pub(crate) transfers_error_banner: gtk4::Box,
    pub(crate) transfers_error_label: gtk4::Label,
    pub(crate) transfers_list_box: gtk4::Box,
    pub(crate) sheet_count_label: gtk4::Label,
    pub(crate) sheet_toggle_label: gtk4::Label,
    pub(crate) sheet_scroll: gtk4::ScrolledWindow,
    pub(crate) sheet_list_box: gtk4::Box,
}

pub(crate) fn build_receive_tab(sender: &ComponentSender<SettingsModel>) -> ReceiveTabParts {
    let root = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(0)
        .vexpand(true)
        .build();
    root.add_css_class("kn-map-ground");

    let top_controls = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(8)
        .margin_start(16)
        .margin_end(16)
        .margin_top(10)
        .build();

    let seg_well = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .homogeneous(true)
        .spacing(4)
        .build();
    seg_well.add_css_class("kn-track-well");

    let mk_seg = |icon_kind: LucideIcon, label: &str, mode: VisibilityMode| -> gtk4::Button {
        let row = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(6)
            .halign(gtk4::Align::Center)
            .valign(gtk4::Align::Center)
            .build();
        row.append(&lucide_icon(icon_kind, 16));
        row.append(&gtk4::Label::new(Some(label)));
        let btn = gtk4::Button::builder().child(&row).build();
        btn.add_css_class("kn-seg-btn");
        let s = sender.clone();
        btn.connect_clicked(move |_| s.input(SettingsMsg::SetVisibility(mode)));
        btn
    };

    let vis_hidden_btn = mk_seg(LucideIcon::EyeOff, "Hidden", VisibilityMode::Hidden);
    let vis_paired_btn = mk_seg(
        LucideIcon::ShieldCheck,
        "Paired",
        VisibilityMode::PairedOnly,
    );
    let vis_everyone_btn = mk_seg(
        LucideIcon::Radio,
        "Everyone",
        VisibilityMode::EveryoneNearby,
    );

    seg_well.append(&vis_hidden_btn);
    seg_well.append(&vis_paired_btn);
    seg_well.append(&vis_everyone_btn);
    top_controls.append(&seg_well);
    root.append(&top_controls);

    let radar = build_radar_stage();
    root.append(&radar.stage);

    let pairing_card = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(10)
        .margin_start(16)
        .margin_end(16)
        .margin_bottom(8)
        .visible(false)
        .build();
    pairing_card.add_css_class("keynear-card");

    let pairing_title = gtk4::Label::builder()
        .label("PAIRING VERIFICATION")
        .xalign(0.0)
        .build();
    pairing_title.add_css_class("kn-section-heading");

    let pairing_peer_label = gtk4::Label::builder()
        .label("")
        .xalign(0.0)
        .wrap(true)
        .build();
    pairing_peer_label.add_css_class("keynear-description");

    let pairing_code_label = gtk4::Label::builder().label("000 000").xalign(0.5).build();
    pairing_code_label.add_css_class("keynear-code");

    let pairing_timer_label = gtk4::Label::builder().label("").xalign(0.5).build();
    pairing_timer_label.add_css_class("keynear-caption");

    let pairing_actions = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(10)
        .homogeneous(true)
        .build();

    let pair_reject_btn = gtk4::Button::with_label("Decline");
    pair_reject_btn.add_css_class("kn-error-action-btn");
    {
        let s = sender.clone();
        pair_reject_btn.connect_clicked(move |_| s.input(SettingsMsg::CancelPairing));
    }

    let pair_confirm_btn = gtk4::Button::with_label("Confirm");
    pair_confirm_btn.add_css_class("kn-loading-btn");
    {
        let s = sender.clone();
        pair_confirm_btn.connect_clicked(move |_| s.input(SettingsMsg::ConfirmPairing));
    }

    pairing_actions.append(&pair_reject_btn);
    pairing_actions.append(&pair_confirm_btn);
    pairing_card.append(&pairing_title);
    pairing_card.append(&pairing_peer_label);
    pairing_card.append(&pairing_code_label);
    pairing_card.append(&pairing_timer_label);
    pairing_card.append(&pairing_actions);
    root.append(&pairing_card);

    let sheets = build_receive_sheets(sender);
    root.append(&sheets.transfers_card);
    root.append(&sheets.sheet_card);

    ReceiveTabParts {
        root,
        vis_hidden_btn,
        vis_paired_btn,
        vis_everyone_btn,
        radar_draw_state: radar.radar_draw_state,
        radar_canvas: radar.radar_canvas,
        self_node_circle: radar.self_node_circle,
        self_name_label: radar.self_name_label,
        self_countdown_label: radar.self_countdown_label,
        radar_pins_overlay: radar.radar_pins_overlay,
        pairing_card,
        pairing_peer_label,
        pairing_code_label,
        pairing_timer_label,
        transfers_card: sheets.transfers_card,
        transfers_error_banner: sheets.transfers_error_banner,
        transfers_error_label: sheets.transfers_error_label,
        transfers_list_box: sheets.transfers_list_box,
        sheet_count_label: sheets.sheet_count_label,
        sheet_toggle_label: sheets.sheet_toggle_label,
        sheet_scroll: sheets.sheet_scroll,
        sheet_list_box: sheets.sheet_list_box,
    }
}
