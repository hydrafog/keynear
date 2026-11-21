use gtk4::prelude::*;
use keynear_protocol::VisibilityMode;
use relm4::ComponentSender;

use crate::state::{format_countdown_short, SettingsModel};
use crate::views::receive::{rebuild_sheet_rows, rebuild_transfers_rows};

use super::SettingsWidgets;

pub(super) fn update_visibility(model: &SettingsModel, widgets: &mut SettingsWidgets) {
    let vis_buttons = [
        (&widgets.receive.vis_hidden_btn, VisibilityMode::Hidden),
        (&widgets.receive.vis_paired_btn, VisibilityMode::PairedOnly),
        (
            &widgets.receive.vis_everyone_btn,
            VisibilityMode::EveryoneNearby,
        ),
    ];
    for (btn, mode) in vis_buttons {
        btn.remove_css_class("kn-seg-btn-active");
        btn.remove_css_class("kn-seg-btn");
        btn.add_css_class(if model.visibility_mode == mode {
            "kn-seg-btn-active"
        } else {
            "kn-seg-btn"
        });
    }

    let radar_active = model.visibility_mode != VisibilityMode::Hidden;
    widgets
        .receive
        .radar_draw_state
        .set((radar_active, model.dark_theme));
    widgets.receive.radar_canvas.queue_draw();

    widgets
        .receive
        .self_node_circle
        .remove_css_class("kn-center-node-active");
    widgets
        .receive
        .self_node_circle
        .remove_css_class("kn-center-node-hidden");
    widgets
        .receive
        .self_node_circle
        .add_css_class(if radar_active {
            "kn-center-node-active"
        } else {
            "kn-center-node-hidden"
        });
    let _ = &widgets.receive.self_name_label;
    if model.visibility_mode == VisibilityMode::EveryoneNearby {
        widgets
            .receive
            .self_countdown_label
            .set_label(&format_countdown_short(model.everyone_remaining_secs));
        widgets.receive.self_countdown_label.set_visible(true);
    } else {
        widgets.receive.self_countdown_label.set_visible(false);
    }
}

pub(super) fn update_pairing(model: &SettingsModel, widgets: &mut SettingsWidgets) {
    if let Some(ref prompt) = model.pending_pairing {
        widgets
            .receive
            .pairing_peer_label
            .set_label(&format!("Compare this code with {}:", prompt.peer_name));
        widgets
            .receive
            .pairing_code_label
            .set_label(&prompt.verification_code);
        widgets
            .receive
            .pairing_timer_label
            .set_label(&format!("Expires in {}s", prompt.remaining_secs));
        widgets.receive.pairing_card.set_visible(true);
    } else {
        widgets.receive.pairing_card.set_visible(false);
    }
}

pub(super) fn update_transfers(
    model: &SettingsModel,
    widgets: &mut SettingsWidgets,
    sender: &ComponentSender<SettingsModel>,
) {
    let show_transfers = !model.transfers.is_empty() || model.transfer_error.is_some();
    widgets.receive.transfers_card.set_visible(show_transfers);
    if let Some(err) = model.transfer_error.as_deref() {
        widgets.receive.transfers_error_label.set_label(err);
        widgets.receive.transfers_error_banner.set_visible(true);
    } else {
        widgets.receive.transfers_error_banner.set_visible(false);
    }
    rebuild_transfers_rows(
        &widgets.receive.transfers_list_box,
        &model.transfers,
        sender,
    );
}

pub(super) fn update_sheet(
    model: &SettingsModel,
    widgets: &mut SettingsWidgets,
    pins: &[crate::state::MapPin],
    sender: &ComponentSender<SettingsModel>,
) {
    widgets
        .receive
        .sheet_count_label
        .set_label(&format!("{} computer(s) in range", pins.len()));
    widgets
        .receive
        .sheet_toggle_label
        .set_label(if model.sheet_expanded {
            "Collapse"
        } else {
            "All devices"
        });
    let target_h = if model.sheet_expanded { 240 } else { 112 };
    widgets
        .receive
        .sheet_scroll
        .set_min_content_height(target_h);
    widgets
        .receive
        .sheet_scroll
        .set_max_content_height(target_h);
    rebuild_sheet_rows(&widgets.receive.sheet_list_box, model, pins, sender);
}
