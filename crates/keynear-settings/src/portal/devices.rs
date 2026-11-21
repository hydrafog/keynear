use keynear_ipc::{self as ipc, IpcRequest, IpcResponse};
use keynear_protocol::{validate_display_name, KeynearError};
use relm4::ComponentSender;

use crate::state::{SettingsModel, SettingsMsg};

use super::picker::materialize_queue_paths;

fn try_daemon_unpair(model: &SettingsModel, device_id: &str) -> Result<bool, String> {
    let Some(ref sock_path) = model.ipc_socket_path else {
        return Ok(false);
    };
    match ipc::send_request_to(
        sock_path,
        &IpcRequest::UnpairDevice {
            device_id: device_id.to_string(),
        },
    ) {
        Ok(IpcResponse::Unpaired { .. }) => Ok(true),
        Ok(IpcResponse::Error { message }) => Err(message),
        Ok(other) => Err(format!("Unexpected daemon IPC response: {other:?}")),
        Err(KeynearError::Transport(_)) => Ok(false),
        Err(err) => Err(err.to_string()),
    }
}

pub(crate) fn confirm_unpair_device(model: &mut SettingsModel, device_id: &str) {
    let removed_name = model
        .paired_devices
        .iter()
        .find(|d| d.device_id == device_id)
        .map(|d| d.display_name.clone())
        .unwrap_or_else(|| "device".to_string());
    let daemon_handled = match try_daemon_unpair(model, device_id) {
        Ok(handled) => handled,
        Err(message) => {
            model.error_message = Some(message);
            return;
        }
    };
    if !daemon_handled {
        if let Err(err) = model.device_store.remove(device_id) {
            model.error_message = Some(err.to_string());
            return;
        }
    }
    model.paired_devices.retain(|d| d.device_id != device_id);
    if model.selected_send_device_id.as_deref() == Some(device_id) {
        model.selected_send_device_id = model.paired_devices.first().map(|d| d.device_id.clone());
    }
    if model.renaming_device_id.as_deref() == Some(device_id) {
        model.renaming_device_id = None;
        model.rename_draft.clear();
    }
    if matches!(
        model.sub_screen,
        Some(crate::state::SubScreen::DeviceDetail(ref id)) if id == device_id
    ) {
        model.sub_screen = None;
    }
    model.pending_unpair_id = None;
    model.error_message = None;
    model.show_snackbar(format!("Removed {removed_name}."));
}

fn local_rename(
    model: &SettingsModel,
    device_id: &str,
    clean: &str,
) -> Result<Option<keynear_protocol::PairedDevice>, KeynearError> {
    match model.device_store.rename(device_id, clean) {
        Ok(device) => Ok(Some(device)),
        Err(KeynearError::Validation {
            field: "deviceId", ..
        }) => Ok(model
            .paired_devices
            .iter()
            .find(|d| d.device_id == device_id)
            .cloned()
            .map(|mut dev| {
                dev.display_name = clean.to_string();
                dev
            })),
        Err(err) => Err(err),
    }
}

fn daemon_rename(
    model: &mut SettingsModel,
    device_id: &str,
    clean: &str,
) -> Result<Option<keynear_protocol::PairedDevice>, ()> {
    let Some(ref sock_path) = model.ipc_socket_path else {
        return Ok(None);
    };
    match ipc::send_request_to(
        sock_path,
        &IpcRequest::RenameDevice {
            device_id: device_id.to_string(),
            display_name: clean.to_string(),
        },
    ) {
        Ok(IpcResponse::Device(dev)) => Ok(Some(dev)),
        Ok(IpcResponse::Error { message }) => {
            model.error_message = Some(message);
            Err(())
        }
        Ok(_) | Err(_) => Ok(None),
    }
}

pub(crate) fn submit_device_rename(
    model: &mut SettingsModel,
    device_id: String,
    display_name: String,
) {
    let clean = match validate_display_name(&display_name) {
        Ok(clean) => clean,
        Err(err) => {
            model.error_message = Some(err.to_string());
            return;
        }
    };
    let mut renamed = match local_rename(model, &device_id, &clean) {
        Ok(device) => device,
        Err(err) => {
            model.error_message = Some(err.to_string());
            return;
        }
    };
    match daemon_rename(model, &device_id, &clean) {
        Ok(Some(dev)) => renamed = Some(dev),
        Ok(None) => {}
        Err(()) => return,
    }
    if let Ok(devices) = model.device_store.load() {
        if !devices.is_empty() {
            model.paired_devices = devices;
        }
    }
    if let Some(dev) = renamed {
        if let Some(slot) = model
            .paired_devices
            .iter_mut()
            .find(|d| d.device_id == dev.device_id)
        {
            *slot = dev;
        }
    }
    model.rename_draft = clean;
    model.error_message = None;
    model.show_snackbar("Device name saved.");
}

pub(crate) fn set_device_auto_accept(model: &mut SettingsModel, device_id: String, enabled: bool) {
    let stored = match model.device_store.get(&device_id) {
        Ok(Some(dev)) => Some(dev),
        Ok(None) => model
            .paired_devices
            .iter()
            .find(|d| d.device_id == device_id)
            .cloned(),
        Err(err) => {
            model.error_message = Some(err.to_string());
            return;
        }
    };
    let Some(mut updated) = stored else {
        model.error_message = Some(format!("Paired device '{device_id}' not found."));
        return;
    };
    if updated.auto_accept_files == enabled {
        return;
    }
    updated.auto_accept_files = enabled;
    match model.device_store.upsert(updated) {
        Ok(devices) => {
            model.paired_devices = devices;
            model.error_message = None;
        }
        Err(err) => {
            model.error_message = Some(err.to_string());
            return;
        }
    }
    if let Some(ref sock_path) = model.ipc_socket_path {
        let _ = ipc::send_request_to(
            sock_path,
            &IpcRequest::SetDeviceAutoAccept { device_id, enabled },
        );
    }
}

pub(crate) fn dispatch_send_for_device(
    model: &mut SettingsModel,
    device_id: String,
    sender: &ComponentSender<SettingsModel>,
) {
    if model.sending_in_flight {
        return;
    }
    model.selected_send_device_id = Some(device_id.clone());
    if model.send_queue.is_empty() && model.send_paths.is_empty() {
        model.send_error = Some("Select at least one file, folder, or note first.".to_string());
        return;
    }
    let Some(sock_path) = model.ipc_socket_path.clone() else {
        model.send_error = Some("Daemon IPC socket is not configured.".to_string());
        return;
    };
    let (paths, cleanup_dir) = match materialize_queue_paths(&model.send_queue, &model.send_paths) {
        Ok(res) => res,
        Err(err) => {
            model.send_error = Some(err);
            return;
        }
    };
    model.sending_in_flight = true;
    model.send_error = None;
    let s = sender.clone();
    std::thread::spawn(move || {
        let result =
            match ipc::send_request_to(&sock_path, &IpcRequest::SendFiles { device_id, paths }) {
                Ok(IpcResponse::Transfer(session)) => Ok(session),
                Ok(IpcResponse::Error { message }) => Err(message),
                Ok(_) => Err("Unexpected daemon IPC response.".to_string()),
                Err(err) => Err(format!(
                    "Keynear daemon unavailable ({err}); start keynear-linux and retry."
                )),
            };
        if let Some(dir) = cleanup_dir {
            let _ = std::fs::remove_dir_all(dir);
        }
        s.input(SettingsMsg::SendCompleted(result));
    });
}
