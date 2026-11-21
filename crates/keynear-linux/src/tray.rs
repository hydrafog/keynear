mod menu;

#[cfg(test)]
mod tests;

use std::process::{Command, Stdio};

use keynear_core::{
    PairedDevice, TransferDirection, TransferSession, TransferState, VisibilityMode,
    EVERYONE_NEARBY_TIMEOUT_SECS,
};
use ksni::TrayMethods;
use tokio::sync::mpsc::UnboundedSender;
use tracing::{info, warn};

pub(crate) const COPY_LATEST_OTP_LABEL: &str = "Copy latest OTP";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrayCommand {
    SetVisibility(VisibilityMode),
    AcceptTransfer(String),
    DeclineTransfer(String),
    CancelTransfer(String),
    SetAutoAcceptFromPaired(bool),
    SetDeviceAutoAccept { device_id: String, enabled: bool },
    CopyLatestOtp,
    LaunchSettings,
    LaunchSendPicker { device_id: Option<String> },
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TrayState {
    pub visibility_mode: VisibilityMode,
    pub everyone_remaining_secs: Option<u64>,
    pub auto_accept_from_paired: bool,
    pub has_latest_otp: bool,
    pub paired_devices: Vec<PairedDevice>,
    pub transfers: Vec<TransferSession>,
    pub mdns_degraded: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrayMenuSnapshot {
    pub title: String,
    pub icon_name: String,
    pub needs_attention: bool,
    pub visibility_selected: usize,
    pub visibility_labels: [String; 3],
    pub paired_device_labels: Vec<String>,
    pub transfer_labels: Vec<String>,
    pub auto_accept_from_paired: bool,
    pub has_latest_otp: bool,
}

pub fn format_everyone_label(mode: VisibilityMode, remaining_secs: Option<u64>) -> String {
    let secs = match (mode, remaining_secs) {
        (VisibilityMode::EveryoneNearby, Some(rem)) => rem,
        (VisibilityMode::EveryoneNearby, None) => EVERYONE_NEARBY_TIMEOUT_SECS,
        (_, Some(rem)) if rem > 0 => rem,
        _ => EVERYONE_NEARBY_TIMEOUT_SECS,
    };
    let mins = secs / 60;
    let rem_secs = secs % 60;
    format!("Everyone ({mins:02}:{rem_secs:02})")
}

fn visibility_index(mode: VisibilityMode) -> usize {
    match mode {
        VisibilityMode::Hidden => 0,
        VisibilityMode::PairedOnly => 1,
        VisibilityMode::EveryoneNearby => 2,
    }
}

fn tray_icon_name(state: &TrayState, needs_attention: bool) -> String {
    if needs_attention {
        "mail-unread-symbolic".to_string()
    } else if state.mdns_degraded && state.visibility_mode != VisibilityMode::Hidden {
        "dialog-warning-symbolic".to_string()
    } else {
        match state.visibility_mode {
            VisibilityMode::Hidden => "network-wireless-offline-symbolic".to_string(),
            VisibilityMode::PairedOnly => "network-wireless-encrypted-symbolic".to_string(),
            VisibilityMode::EveryoneNearby => {
                "network-wireless-signal-excellent-symbolic".to_string()
            }
        }
    }
}

fn paired_device_label(device: &PairedDevice) -> String {
    if device.auto_accept_files {
        format!("{} (auto-accept)", device.display_name)
    } else {
        device.display_name.clone()
    }
}

fn active_transfers(state: &TrayState) -> Vec<&TransferSession> {
    state
        .transfers
        .iter()
        .filter(|t| {
            matches!(
                t.state,
                TransferState::PendingConsent | TransferState::Active
            )
        })
        .collect()
}

pub fn build_tray_snapshot(state: &TrayState) -> TrayMenuSnapshot {
    let visibility_selected = visibility_index(state.visibility_mode);
    let visibility_labels = [
        "Hidden".to_string(),
        "Paired devices only".to_string(),
        format_everyone_label(state.visibility_mode, state.everyone_remaining_secs),
    ];
    let needs_attention = state
        .transfers
        .iter()
        .any(|t| t.state == TransferState::PendingConsent);
    let icon_name = tray_icon_name(state, needs_attention);
    let title = format!("Keynear — {}", visibility_labels[visibility_selected]);
    let paired_device_labels = state
        .paired_devices
        .iter()
        .map(paired_device_label)
        .collect();
    let transfer_labels = active_transfers(state)
        .iter()
        .map(|t| format_transfer_menu_label(t))
        .collect();

    TrayMenuSnapshot {
        title,
        icon_name,
        needs_attention,
        visibility_selected,
        visibility_labels,
        paired_device_labels,
        transfer_labels,
        auto_accept_from_paired: state.auto_accept_from_paired,
        has_latest_otp: state.has_latest_otp,
    }
}

fn format_transfer_menu_label(session: &TransferSession) -> String {
    let direction = match session.direction {
        TransferDirection::Send => "Sending",
        TransferDirection::Receive => "Receiving",
    };
    let item_summary = if session.items.is_empty() {
        "files".to_string()
    } else if session.items.len() == 1 {
        session.items[0].name.clone()
    } else {
        format!(
            "{} (+{} more)",
            session.items[0].name,
            session.items.len() - 1
        )
    };
    match session.state {
        TransferState::PendingConsent => {
            format!("Incoming: {} from {}", item_summary, session.peer_device_id)
        }
        TransferState::Active => {
            let pct = session
                .transferred_bytes
                .saturating_mul(100)
                .checked_div(session.total_bytes)
                .unwrap_or(0)
                .min(100);
            format!("{direction}: {item_summary} ({pct}%)")
        }
        TransferState::Completed => format!("Completed: {item_summary}"),
        TransferState::Cancelled => format!("Cancelled: {item_summary}"),
        TransferState::Failed => format!("Failed: {item_summary}"),
    }
}

pub fn spawn_settings_process(extra_args: &[String]) -> std::io::Result<()> {
    let bin_path = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join("keynear-settings")))
        .filter(|candidate| candidate.is_file())
        .unwrap_or_else(|| std::path::PathBuf::from("keynear-settings"));

    Command::new(bin_path)
        .args(extra_args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    Ok(())
}

pub struct KeynearTray {
    pub state: TrayState,
    pub cmd_tx: UnboundedSender<TrayCommand>,
}

fn log_tray_registered() {
    info!(event = "tray_registered");
}

fn log_tray_unavailable(err: impl std::fmt::Display) {
    warn!(event = "tray_unavailable", error = %err);
}

pub async fn spawn_tray(
    state: TrayState,
    cmd_tx: UnboundedSender<TrayCommand>,
) -> Option<ksni::Handle<KeynearTray>> {
    let tray = KeynearTray { state, cmd_tx };
    match tray.assume_sni_available(false).spawn().await {
        Ok(handle) => {
            log_tray_registered();
            Some(handle)
        }
        Err(err) => {
            log_tray_unavailable(err);
            None
        }
    }
}
