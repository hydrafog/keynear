use keynear_ipc::{self as ipc, IpcRequest, IpcResponse};

use crate::state::SettingsModel;

fn apply_status(model: &mut SettingsModel, status: keynear_ipc::DaemonStatus) {
    model.visibility_mode = status.visibility_mode;
    model.config.visibility_mode = status.visibility_mode;
    model.everyone_remaining_secs = status.everyone_remaining_secs.unwrap_or(0);
    model.config.auto_accept_from_paired = status.auto_accept_from_paired;
    model.incoming_directory = status.incoming_directory.clone();
    model.config.incoming_directory = status.incoming_directory;
    model.transfer_error = None;
}

pub(crate) fn persist_config(model: &mut SettingsModel) {
    if let Some(ref path) = model.config_path {
        if let Err(err) = model.config.save_to_path(path) {
            model.error_message = Some(err.to_string());
            return;
        }
    }
    if let Some(ref sock_path) = model.ipc_socket_path {
        match ipc::send_request_to(
            sock_path,
            &IpcRequest::UpdateConfig {
                config: model.config.clone(),
            },
        ) {
            Ok(IpcResponse::Config(updated)) => {
                model.incoming_directory = updated.incoming_directory.clone();
                model.config = updated;
                model.transfer_error = None;
            }
            Ok(IpcResponse::Error { message }) => {
                model.error_message = Some(message);
            }
            Ok(other) => {
                model.error_message = Some(format!("Unexpected daemon IPC response: {other:?}"));
            }
            Err(err) => {
                model.error_message = Some(format!("Failed to persist config to daemon: {err}"));
            }
        }
    }
}

pub(crate) fn sync_visibility(model: &mut SettingsModel, mode: keynear_protocol::VisibilityMode) {
    if let Some(ref path) = model.config_path {
        if let Err(err) = model.config.save_to_path(path) {
            model.error_message = Some(err.to_string());
        }
    }
    if let Some(ref sock_path) = model.ipc_socket_path {
        match ipc::send_request_to(sock_path, &IpcRequest::SetVisibility { mode }) {
            Ok(IpcResponse::Status(status)) => {
                model.visibility_mode = status.visibility_mode;
                model.config.visibility_mode = status.visibility_mode;
                model.everyone_remaining_secs = status.everyone_remaining_secs.unwrap_or(0);
                model.transfer_error = None;
            }
            Ok(IpcResponse::Error { message }) => {
                model.error_message = Some(message);
            }
            Ok(other) => {
                model.error_message = Some(format!("Unexpected daemon IPC response: {other:?}"));
            }
            Err(err) => {
                model.error_message = Some(format!("Failed to persist config to daemon: {err}"));
            }
        }
    }
}

fn refresh_config(model: &mut SettingsModel, sock_path: &std::path::Path, surface_error: bool) {
    match ipc::send_request_to(sock_path, &IpcRequest::GetConfig) {
        Ok(IpcResponse::Config(cfg)) => {
            model.incoming_directory = cfg.incoming_directory.clone();
            model.config = cfg;
        }
        Ok(IpcResponse::Error { message }) => {
            model.transfer_error = Some(message);
        }
        Ok(_) => {}
        Err(err) => {
            if surface_error {
                model.transfer_error = Some(err.to_string());
            }
        }
    }
}

fn refresh_transfers(model: &mut SettingsModel, sock_path: &std::path::Path, surface_error: bool) {
    match ipc::send_request_to(sock_path, &IpcRequest::ListTransfers) {
        Ok(IpcResponse::Transfers(list)) => {
            model.transfers = list;
            model.transfer_error = None;
        }
        Ok(IpcResponse::Error { message }) => {
            model.transfer_error = Some(message);
        }
        Ok(_) => {
            model.transfer_error = Some("Unexpected daemon IPC response.".to_string());
        }
        Err(err) => {
            if surface_error {
                model.transfer_error = Some(err.to_string());
            }
        }
    }
}

pub(crate) fn refresh_from_daemon(model: &mut SettingsModel, surface_error: bool) {
    let Some(sock_path) = model.ipc_socket_path.clone() else {
        if surface_error {
            model.transfer_error = Some("Daemon IPC socket is not configured.".to_string());
        }
        return;
    };
    match ipc::send_request_to(&sock_path, &IpcRequest::Status) {
        Ok(IpcResponse::Status(status)) => apply_status(model, status),
        Ok(IpcResponse::Error { message }) => {
            model.transfer_error = Some(message);
            return;
        }
        Ok(_) => {
            model.transfer_error = Some("Unexpected daemon IPC response.".to_string());
            return;
        }
        Err(err) => {
            if surface_error {
                model.transfer_error = Some(format!(
                    "Keynear daemon unavailable ({err}); start keynear-linux and retry."
                ));
            }
            return;
        }
    }
    refresh_config(model, &sock_path, surface_error);
    refresh_transfers(model, &sock_path, surface_error);
}

pub(crate) fn send_transfer_action(model: &mut SettingsModel, request: IpcRequest) {
    let Some(ref sock_path) = model.ipc_socket_path else {
        model.transfer_error = Some("Daemon IPC socket is not configured.".to_string());
        return;
    };
    match ipc::send_request_to(sock_path, &request) {
        Ok(IpcResponse::Transfer(updated)) => {
            if let Some(existing) = model
                .transfers
                .iter_mut()
                .find(|t| t.transfer_id == updated.transfer_id)
            {
                *existing = updated;
            } else {
                model.transfers.insert(0, updated);
            }
            model.transfer_error = None;
        }
        Ok(IpcResponse::Error { message }) => {
            model.transfer_error = Some(message);
        }
        Ok(_) => {
            model.transfer_error = Some("Unexpected daemon IPC response.".to_string());
        }
        Err(err) => {
            model.transfer_error = Some(err.to_string());
        }
    }
}
