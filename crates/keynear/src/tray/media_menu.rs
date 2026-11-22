use ksni::menu::{MenuItem, StandardItem, SubMenu};

use super::{KeynearTray, NowPlayingState, TrayCommand};

pub(super) fn now_playing_menu(state: &NowPlayingState) -> MenuItem<KeynearTray> {
    let playing = state.playing;
    SubMenu {
        label: "Now playing".to_string(),
        submenu: vec![
            StandardItem {
                label: now_playing_label(state),
                enabled: false,
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: state.artist.clone(),
                enabled: false,
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: if playing { "Pause" } else { "Play" }.to_string(),
                activate: Box::new(move |this: &mut KeynearTray| {
                    let command = if playing {
                        TrayCommand::MediaPause
                    } else {
                        TrayCommand::MediaPlay
                    };
                    let _ = this.cmd_tx.send(command);
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "Next".to_string(),
                activate: Box::new(|this: &mut KeynearTray| {
                    let _ = this.cmd_tx.send(TrayCommand::MediaNext);
                }),
                ..Default::default()
            }
            .into(),
        ],
        ..Default::default()
    }
    .into()
}

fn now_playing_label(state: &NowPlayingState) -> String {
    state
        .title
        .chars()
        .take(48)
        .collect::<String>()
        .split_whitespace()
        .take(6)
        .collect::<Vec<_>>()
        .join(" ")
}
