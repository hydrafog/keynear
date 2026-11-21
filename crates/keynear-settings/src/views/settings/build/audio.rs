use gtk4::prelude::*;
use relm4::ComponentSender;

use crate::state::{SettingsModel, SettingsMsg};

use super::common::switch_row;

pub(super) fn build_audio_card(
    sender: &ComponentSender<SettingsModel>,
) -> (gtk4::Box, gtk4::Switch, gtk4::Switch) {
    let audio_card = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(10)
        .build();
    audio_card.add_css_class("keynear-card");

    let (audio_row, audio_handoff_switch) =
        switch_row("Smart audio handoff", "Move audio between devices", {
            let s = sender.clone();
            move |enabled| s.input(SettingsMsg::ToggleAudioHandoff(enabled))
        });
    let (notify_row, notify_auto_accept_switch) = switch_row(
        "Completion notifications",
        "Notify when files finish saving",
        {
            let s = sender.clone();
            move |enabled| s.input(SettingsMsg::ToggleNotifyAutoAccept(enabled))
        },
    );
    audio_card.append(&audio_row);
    audio_card.append(&notify_row);
    (audio_card, audio_handoff_switch, notify_auto_accept_switch)
}

pub(super) fn build_appearance_card(
    sender: &ComponentSender<SettingsModel>,
) -> (gtk4::Box, gtk4::Switch) {
    let appear_card = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(10)
        .build();
    appear_card.add_css_class("keynear-card");

    let (theme_row, dark_theme_switch) = switch_row("Dark theme", "Switch the app color theme", {
        let s = sender.clone();
        move |enabled| s.input(SettingsMsg::ToggleDarkTheme(enabled))
    });
    appear_card.append(&theme_row);
    (appear_card, dark_theme_switch)
}
