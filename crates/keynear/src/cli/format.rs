use std::path::{Path, PathBuf};

use keynear_ipc::{send_request_to, DaemonStatus, IpcRequest, IpcResponse};
use keynear_protocol::{
    default_config_path, DaemonConfig, KeynearError, PairedDevice, TransferDirection,
    TransferSession, TransferState, VisibilityMode,
};

use super::load_daemon_config;

fn format_visibility_mode(mode: VisibilityMode) -> &'static str {
    match mode {
        VisibilityMode::Hidden => "hidden",
        VisibilityMode::PairedOnly => "paired",
        VisibilityMode::EveryoneNearby => "everyone",
    }
}

pub fn cli_action_to_ipc_request(action: &super::CliAction) -> Option<IpcRequest> {
    use super::CliAction;

    match action {
        CliAction::Send {
            device_id: Some(device_id),
            paths,
        } => Some(IpcRequest::SendFiles {
            device_id: device_id.clone(),
            paths: paths.clone(),
        }),
        CliAction::Accept { transfer_id } => Some(IpcRequest::AcceptTransfer {
            transfer_id: transfer_id.clone(),
        }),
        CliAction::Decline { transfer_id } => Some(IpcRequest::DeclineTransfer {
            transfer_id: transfer_id.clone(),
        }),
        CliAction::Cancel { transfer_id } => Some(IpcRequest::CancelTransfer {
            transfer_id: transfer_id.clone(),
        }),
        CliAction::Transfers => Some(IpcRequest::ListTransfers),
        CliAction::Status => Some(IpcRequest::Status),
        CliAction::Devices => Some(IpcRequest::ListDevices),
        CliAction::SetVisibility { mode } => Some(IpcRequest::SetVisibility { mode: *mode }),
        CliAction::RespondPairing { approve } => {
            Some(IpcRequest::RespondPairing { approve: *approve })
        }
        CliAction::RenameDevice {
            device_id,
            display_name,
        } => Some(IpcRequest::RenameDevice {
            device_id: device_id.clone(),
            display_name: display_name.clone(),
        }),
        CliAction::Send {
            device_id: None, ..
        }
        | CliAction::SendPicker { .. }
        | CliAction::Settings { .. }
        | CliAction::Version
        | CliAction::Help
        | CliAction::Run { .. } => None,
    }
}

pub fn format_device_summary(device: &PairedDevice) -> String {
    format!(
        "{}\t{}\t{:?}\tauto_accept={}\t{}",
        device.device_id,
        device.display_name,
        device.device_role,
        device.auto_accept_files,
        device.last_lan_address
    )
}

pub fn format_status_summary(status: &DaemonStatus) -> String {
    let remaining = status
        .everyone_remaining_secs
        .map(|secs| format!("{secs}s"))
        .unwrap_or_else(|| "none".to_string());
    let addrs = if status.listen_addrs.is_empty() {
        "none".to_string()
    } else {
        status.listen_addrs.join(",")
    };
    format!(
        "device_id={}\nendpoint_id={}\nvisibility={}\neveryone_remaining={}\nauto_accept_from_paired={}\nincoming_directory={}\nactive_transfers={}\npaired_devices={}\nmdns_degraded={}\nlisten_addrs={}",
        status.device_id,
        status.endpoint_id,
        format_visibility_mode(status.visibility_mode),
        remaining,
        status.auto_accept_from_paired,
        status.incoming_directory,
        status.active_transfer_count,
        status.paired_device_count,
        status.mdns_degraded,
        addrs
    )
}

pub fn format_config_summary(config: &DaemonConfig) -> String {
    format!(
        "visibility={}\nauto_accept_from_paired={}\nincoming_directory={}\nnotify_auto_accept_complete={}",
        format_visibility_mode(config.visibility_mode),
        config.auto_accept_from_paired,
        config.incoming_directory,
        config.notify_auto_accept_complete
    )
}

fn transfer_items_summary(session: &TransferSession) -> String {
    if session.items.is_empty() {
        "transfer".to_string()
    } else if session.items.len() == 1 {
        session.items[0].name.clone()
    } else {
        format!(
            "{} (+{} more)",
            session.items[0].name,
            session.items.len() - 1
        )
    }
}

pub fn format_transfer_summary(session: &TransferSession) -> String {
    let dir = match session.direction {
        TransferDirection::Send => "send",
        TransferDirection::Receive => "receive",
    };
    let state = match session.state {
        TransferState::PendingConsent => "pending_consent",
        TransferState::Active => "active",
        TransferState::Completed => "completed",
        TransferState::Cancelled => "cancelled",
        TransferState::Failed => "failed",
    };
    let items = transfer_items_summary(session);
    format!(
        "{}\t{}\t{}\t{}\t{}/{}\t{}",
        session.transfer_id,
        dir,
        session.peer_device_id,
        state,
        session.transferred_bytes,
        session.total_bytes,
        items
    )
}

pub fn run_ipc_command(socket_path: &Path, request: &IpcRequest) -> Result<String, KeynearError> {
    match send_request_to(socket_path, request)? {
        IpcResponse::Transfer(session) => Ok(format_transfer_summary(&session)),
        IpcResponse::Transfers(sessions) => {
            if sessions.is_empty() {
                Ok("No active transfers".to_string())
            } else {
                let lines: Vec<String> = sessions.iter().map(format_transfer_summary).collect();
                Ok(lines.join("\n"))
            }
        }
        IpcResponse::Status(status) => Ok(format_status_summary(&status)),
        IpcResponse::Devices(devices) => {
            if devices.is_empty() {
                Ok("No paired devices".to_string())
            } else {
                let lines: Vec<String> = devices.iter().map(format_device_summary).collect();
                Ok(lines.join("\n"))
            }
        }
        IpcResponse::Device(device) => Ok(format_device_summary(&device)),
        IpcResponse::Config(config) => Ok(format_config_summary(&config)),
        IpcResponse::Unpaired { device_id } => Ok(format!("unpaired\t{device_id}")),
        IpcResponse::PairingDecision { approved } => Ok(if approved {
            "pairing_decision=approved"
        } else {
            "pairing_decision=rejected"
        }
        .to_string()),
        IpcResponse::Error { message } => Err(KeynearError::Transfer(message)),
    }
}

pub fn run_settings_command(
    socket_path: &Path,
    config_path: Option<&PathBuf>,
    validate_only: bool,
) -> Result<String, KeynearError> {
    let resolved_path = config_path.cloned().unwrap_or_else(default_config_path);
    if validate_only {
        let config = load_daemon_config(config_path)?;
        config.validate()?;
        return Ok(format!(
            "config_valid=true\nconfig_path={}\n{}",
            resolved_path.display(),
            format_config_summary(&config)
        ));
    }
    let config = if config_path.is_none() {
        match send_request_to(socket_path, &IpcRequest::GetConfig) {
            Ok(IpcResponse::Config(cfg)) => cfg,
            _ => load_daemon_config(None)?,
        }
    } else {
        load_daemon_config(config_path)?
    };
    Ok(format!(
        "config_path={}\n{}",
        resolved_path.display(),
        format_config_summary(&config)
    ))
}
