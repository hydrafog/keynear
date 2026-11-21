mod header;
mod receive;
mod send;
mod settings;

use relm4::ComponentSender;

use crate::state::SettingsModel;

use super::SettingsWidgets;

pub fn update_view(
    model: &SettingsModel,
    widgets: &mut SettingsWidgets,
    sender: ComponentSender<SettingsModel>,
) {
    header::update_top_bar(model, widgets);
    header::update_banner(model, widgets);
    receive::update_visibility(model, widgets);
    let pins = model.map_pins();
    crate::views::receive::rebuild_radar_pins(&widgets.receive.radar_pins_overlay, &pins, &sender);
    receive::update_pairing(model, widgets);
    receive::update_transfers(model, widgets, &sender);
    receive::update_sheet(model, widgets, &pins, &sender);
    send::update_send(model, widgets, &sender);
    settings::update_settings(model, widgets, &sender);
    settings::update_snackbar(model, widgets);
    settings::update_bottom_nav(model, widgets);
}
