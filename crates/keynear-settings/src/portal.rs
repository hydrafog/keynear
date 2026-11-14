use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

use gtk4::glib;
use gtk4::prelude::*;
use keynear_core::{remove_paired_device, validate_display_name, KeynearError, VisibilityMode};
use relm4::ComponentSender;

use crate::ipc::{self, IpcRequest, IpcResponse};
use crate::state::{SendItemKind, SendQueueItem, SettingsModel, SettingsMsg};

static PORTAL_TOKIO_RT: OnceLock<Result<tokio::runtime::Runtime, String>> = OnceLock::new();

fn portal_runtime() -> Result<&'static tokio::runtime::Runtime, String> {
    let entry = PORTAL_TOKIO_RT.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("keynear-portal")
            .enable_all()
            .build()
            .map_err(|e| format!("failed to initialize portal runtime: {e}"))
    });
    match entry {
        Ok(rt) => Ok(rt),
        Err(err) => Err(err.clone()),
    }
}

pub fn file_uri_to_path(uri: &str) -> Result<PathBuf, KeynearError> {
    let after_scheme = uri
        .strip_prefix("file://")
        .ok_or_else(|| KeynearError::Validation {
            field: "uri",
            message: format!("expected file:// URI, got '{uri}'"),
        })?;
    let raw_path = if let Some(rest) = after_scheme.strip_prefix("localhost/") {
        format!("/{rest}")
    } else if after_scheme.starts_with('/') {
        after_scheme.to_string()
    } else {
        return Err(KeynearError::Validation {
            field: "uri",
            message: format!("unsupported non-local file URI '{uri}'"),
        });
    };

    let bytes = raw_path.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if i + 2 >= bytes.len() {
                return Err(KeynearError::Validation {
                    field: "uri",
                    message: "truncated percent-encoding in file URI".to_string(),
                });
            }
            let hi = (bytes[i + 1] as char).to_digit(16);
            let lo = (bytes[i + 2] as char).to_digit(16);
            match (hi, lo) {
                (Some(h), Some(l)) => {
                    decoded.push(((h << 4) | l) as u8);
                    i += 3;
                }
                _ => {
                    return Err(KeynearError::Validation {
                        field: "uri",
                        message: "invalid hex escape in file URI".to_string(),
                    });
                }
            }
        } else {
            decoded.push(bytes[i]);
            i += 1;
        }
    }

    if decoded.is_empty() || decoded.contains(&0) {
        return Err(KeynearError::Validation {
            field: "uri",
            message: "file URI resolves to an empty or NUL path".to_string(),
        });
    }

    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        Ok(PathBuf::from(std::ffi::OsString::from_vec(decoded)))
    }
    #[cfg(not(unix))]
    {
        String::from_utf8(decoded)
            .map(PathBuf::from)
            .map_err(|_| KeynearError::Validation {
                field: "uri",
                message: "file URI is not valid UTF-8".to_string(),
            })
    }
}

async fn pick_paths_via_portal(
    window: &gtk4::Window,
    title: &'static str,
    multiple: bool,
    directory: bool,
) -> Result<Vec<PathBuf>, String> {
    let keep_alive = match window.native() {
        Some(native) => ashpd::WindowIdentifier::from_native(&native).await,
        None => None,
    };
    let portal_id = keep_alive
        .as_ref()
        .and_then(|id| id.to_string().parse::<ashpd::WindowIdentifierType>().ok())
        .map(ashpd::WindowIdentifier::from);
    let rt = portal_runtime()?;
    let (tx, rx) = tokio::sync::oneshot::channel();
    rt.spawn(async move {
        let res = async {
            let req = ashpd::desktop::file_chooser::SelectedFiles::open_file()
                .title(title)
                .modal(true)
                .multiple(multiple)
                .directory(directory)
                .identifier(portal_id);
            let selected = req.send().await.map_err(|e| e.to_string())?;
            match selected.response() {
                Ok(response) => response
                    .uris()
                    .iter()
                    .map(|uri| file_uri_to_path(uri.as_str()).map_err(|e| e.to_string()))
                    .collect::<Result<Vec<PathBuf>, String>>(),
                Err(ashpd::Error::Response(ashpd::desktop::ResponseError::Cancelled)) => {
                    Ok(Vec::new())
                }
                Err(err) => Err(err.to_string()),
            }
        }
        .await;
        let _ = tx.send(res);
    });
    let result = rx
        .await
        .map_err(|_| "File chooser portal task did not complete.".to_string())?;
    drop(keep_alive);
    result
}

pub(crate) fn spawn_portal_picker<F>(
    window: Option<&gtk4::Window>,
    sender: &ComponentSender<SettingsModel>,
    title: &'static str,
    multiple: bool,
    directory: bool,
    on_paths: F,
) where
    F: Fn(Vec<PathBuf>) -> SettingsMsg + 'static,
{
    let Some(win) = window.cloned() else {
        sender.input(SettingsMsg::PortalError(
            "Settings window is not available for file chooser.".to_string(),
        ));
        return;
    };
    let s = sender.clone();
    glib::spawn_future_local(async move {
        match pick_paths_via_portal(&win, title, multiple, directory).await {
            Ok(paths) if !paths.is_empty() => s.input(on_paths(paths)),
            Ok(_) => {}
            Err(err) => s.input(SettingsMsg::PortalError(err)),
        }
    });
}

pub(crate) fn estimate_path_bytes(path: &Path) -> u64 {
    std::fs::metadata(path)
        .map(|m| if m.is_dir() { 4096 } else { m.len() })
        .unwrap_or(0)
}

pub(crate) fn item_from_path(seq: u64, path: PathBuf) -> SendQueueItem {
    let is_dir = std::fs::metadata(&path)
        .map(|m| m.is_dir())
        .unwrap_or(false);
    let raw_name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string());
    let display = if is_dir && !raw_name.ends_with('/') {
        format!("{raw_name}/")
    } else {
        raw_name
    };
    let size_bytes = estimate_path_bytes(&path);
    SendQueueItem {
        id: format!("path-{seq}"),
        name: display,
        size_bytes,
        kind: if is_dir {
            SendItemKind::Folder
        } else {
            SendItemKind::File
        },
        source_path: Some(path),
        text_payload: None,
    }
}

pub(crate) fn materialize_queue_paths(
    send_queue: &[SendQueueItem],
    send_paths: &[PathBuf],
) -> Result<(Vec<PathBuf>, Option<PathBuf>), String> {
    let mut paths = Vec::new();
    let mut temp_dir: Option<PathBuf> = None;

    for (idx, item) in send_queue.iter().enumerate() {
        if let Some(ref p) = item.source_path {
            paths.push(p.clone());
        } else if let Some(ref body) = item.text_payload {
            let dir = match temp_dir {
                Some(ref d) => d.clone(),
                None => {
                    let nonce = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .map(|d| d.as_nanos())
                        .unwrap_or(0);
                    let d = std::env::temp_dir().join(format!("keynear-gtk-send-{nonce}"));
                    std::fs::create_dir_all(&d)
                        .map_err(|e| format!("Unable to prepare send staging: {e}"))?;
                    temp_dir = Some(d.clone());
                    d
                }
            };
            let sanitized: String = item
                .name
                .chars()
                .map(|c| {
                    if c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_' {
                        c
                    } else {
                        '_'
                    }
                })
                .collect();
            let dest = dir.join(format!("{idx}_{sanitized}"));
            std::fs::write(&dest, body.as_bytes())
                .map_err(|e| format!("Unable to stage text item: {e}"))?;
            paths.push(dest);
        }
    }
    if paths.is_empty() && !send_paths.is_empty() {
        paths.extend(send_paths.iter().cloned());
    }
    Ok((paths, temp_dir))
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
            Ok(_) | Err(_) => {}
        }
    }
}

pub(crate) fn sync_visibility(model: &mut SettingsModel, mode: VisibilityMode) {
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
            Ok(_) | Err(_) => {}
        }
    }
}

pub(crate) fn refresh_from_daemon(model: &mut SettingsModel, surface_error: bool) {
    let Some(ref sock_path) = model.ipc_socket_path else {
        if surface_error {
            model.transfer_error = Some("Daemon IPC socket is not configured.".to_string());
        }
        return;
    };
    match ipc::send_request_to(sock_path, &IpcRequest::Status) {
        Ok(IpcResponse::Status(status)) => {
            model.visibility_mode = status.visibility_mode;
            model.config.visibility_mode = status.visibility_mode;
            model.everyone_remaining_secs = status.everyone_remaining_secs.unwrap_or(0);
            model.config.auto_accept_from_paired = status.auto_accept_from_paired;
            model.incoming_directory = status.incoming_directory.clone();
            model.config.incoming_directory = status.incoming_directory;
            model.transfer_error = None;
        }
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

pub(crate) fn confirm_unpair_device(model: &mut SettingsModel, device_id: &str) {
    let removed_name = model
        .paired_devices
        .iter()
        .find(|d| d.device_id == device_id)
        .map(|d| d.display_name.clone())
        .unwrap_or_else(|| "device".to_string());
    if let Some(ref store) = model.secret_store {
        if let Err(err) = remove_paired_device(&model.device_store, store.as_ref(), device_id) {
            model.error_message = Some(err.to_string());
            return;
        }
    } else if let Err(err) = model.device_store.remove(device_id) {
        model.error_message = Some(err.to_string());
        return;
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
    let mut renamed = match model.device_store.rename(&device_id, &clean) {
        Ok(device) => Some(device),
        Err(KeynearError::Validation {
            field: "deviceId", ..
        }) => model
            .paired_devices
            .iter()
            .find(|d| d.device_id == device_id)
            .cloned()
            .map(|mut dev| {
                dev.display_name = clean.clone();
                dev
            }),
        Err(err) => {
            model.error_message = Some(err.to_string());
            return;
        }
    };
    if let Some(ref sock_path) = model.ipc_socket_path {
        match ipc::send_request_to(
            sock_path,
            &IpcRequest::RenameDevice {
                device_id: device_id.clone(),
                display_name: clean.clone(),
            },
        ) {
            Ok(IpcResponse::Device(dev)) => renamed = Some(dev),
            Ok(IpcResponse::Error { message }) => {
                model.error_message = Some(message);
                return;
            }
            Ok(_) | Err(_) => {}
        }
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
