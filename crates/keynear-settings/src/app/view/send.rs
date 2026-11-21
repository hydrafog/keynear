use gtk4::prelude::*;
use relm4::ComponentSender;

use crate::state::{format_bytes, SettingsModel};
use crate::views::send::{rebuild_send_queue_rows, rebuild_send_targets, SendTargetsParams};

use super::SettingsWidgets;

fn update_text_composer(model: &SettingsModel, widgets: &mut SettingsWidgets) {
    widgets.send.text_tile.remove_css_class("kn-selection-tile");
    widgets
        .send
        .text_tile
        .remove_css_class("kn-selection-tile-active");
    widgets
        .send
        .text_tile
        .add_css_class(if model.text_composer_open {
            "kn-selection-tile-active"
        } else {
            "kn-selection-tile"
        });
    widgets
        .send
        .text_composer_box
        .set_visible(model.text_composer_open);
    if widgets.send.text_entry.text().as_str() != model.text_draft
        && !widgets.send.text_entry.has_focus()
    {
        widgets.send.text_entry.set_text(&model.text_draft);
    }
}

fn update_queue_and_targets(
    model: &SettingsModel,
    widgets: &mut SettingsWidgets,
    sender: &ComponentSender<SettingsModel>,
) {
    let total_bytes: u64 = model.send_queue.iter().map(|i| i.size_bytes).sum();
    widgets.send.queue_heading.set_label(&format!(
        "{} item(s) ({})",
        model.send_queue.len(),
        format_bytes(total_bytes)
    ));
    widgets
        .send
        .queue_card
        .set_visible(!model.send_queue.is_empty());
    widgets
        .send
        .clear_btn
        .set_sensitive(!model.send_queue.is_empty() && !model.sending_in_flight);

    if let Some(ref err) = model.send_error {
        widgets.send.error_label.set_label(err);
        widgets.send.error_banner.set_visible(true);
    } else {
        widgets.send.error_banner.set_visible(false);
    }
    rebuild_send_queue_rows(&widgets.send.queue_list_box, &model.send_queue, sender);
    rebuild_send_targets(
        &widgets.send.targets_box,
        SendTargetsParams {
            devices: &model.paired_devices,
            sending_in_flight: model.sending_in_flight,
            has_queue: !model.send_queue.is_empty(),
            sender,
        },
    );
}

pub(super) fn update_send(
    model: &SettingsModel,
    widgets: &mut SettingsWidgets,
    sender: &ComponentSender<SettingsModel>,
) {
    update_text_composer(model, widgets);
    update_queue_and_targets(model, widgets, sender);
}
