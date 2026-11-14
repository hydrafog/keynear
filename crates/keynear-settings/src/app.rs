use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use gtk4::gdk::Display;
use gtk4::glib;
use gtk4::pango::EllipsizeMode;
use gtk4::prelude::*;
use keynear_core::{
    remove_paired_device, validate_display_name, DaemonConfig, DeviceRole, KeynearError,
    NearbyPeer, PairedDevice, PairedDeviceStore, SecretStore, TransferDirection, TransferSession,
    TransferState, VisibilityMode, EVERYONE_NEARBY_TIMEOUT_SECS, PAIRING_TIMEOUT_SECS,
};
use relm4::{ComponentParts, ComponentSender, RelmApp, SimpleComponent};

use crate::ipc::{self, IpcRequest, IpcResponse};

pub const STYLE_CSS: &str = include_str!("../assets/style.css");

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingPairingView {
    pub peer_token: String,
    pub peer_name: String,
    pub verification_code: String,
    pub remaining_secs: u64,
}

#[derive(Clone)]
pub struct SettingsInit {
    pub config: DaemonConfig,
    pub config_path: Option<PathBuf>,
    pub everyone_remaining_secs: Option<u64>,
    pub error_message: Option<String>,
    pub warning_message: Option<String>,
    pub transfer_error: Option<String>,
    pub paired_devices: Vec<PairedDevice>,
    pub nearby_peers: Vec<NearbyPeer>,
    pub transfers: Vec<TransferSession>,
    pub send_paths: Vec<PathBuf>,
    pub selected_send_device_id: Option<String>,
    pub send_picker_active: bool,
    pub device_store: PairedDeviceStore,
    pub secret_store: Option<Arc<dyn SecretStore>>,
    pub ipc_socket_path: Option<PathBuf>,
}

pub struct SettingsModel {
    config: DaemonConfig,
    config_path: Option<PathBuf>,
    visibility_mode: VisibilityMode,
    everyone_remaining_secs: u64,
    incoming_directory: String,
    error_message: Option<String>,
    warning_message: Option<String>,
    transfer_error: Option<String>,
    send_error: Option<String>,
    nearby_peers: Vec<NearbyPeer>,
    paired_devices: Vec<PairedDevice>,
    transfers: Vec<TransferSession>,
    send_paths: Vec<PathBuf>,
    selected_send_device_id: Option<String>,
    send_picker_active: bool,
    sending_in_flight: bool,
    renaming_device_id: Option<String>,
    rename_draft: String,
    pending_pairing: Option<PendingPairingView>,
    pending_unpair_id: Option<String>,
    device_store: PairedDeviceStore,
    secret_store: Option<Arc<dyn SecretStore>>,
    ipc_socket_path: Option<PathBuf>,
    root_window: Option<gtk4::Window>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsMsg {
    SetVisibility(VisibilityMode),
    Tick,
    RefreshPeers,
    RetryDaemon,
    BeginPair(String),
    ConfirmPairing,
    CancelPairing,
    RequestUnpair(String),
    ConfirmUnpair(String),
    CancelUnpair,
    StartRename(String),
    UpdateRenameDraft(String),
    SubmitRename {
        device_id: String,
        display_name: String,
    },
    CancelRename,
    SetAutoAcceptFiles {
        device_id: String,
        enabled: bool,
    },
    SetGlobalAutoAccept(bool),
    PickSendFiles,
    PickSendFolder,
    StageSendPaths(Vec<PathBuf>),
    ClearSendPaths,
    SelectSendDevice(String),
    SendStagedFiles,
    SendCompleted(Result<TransferSession, String>),
    PickIncomingDirectory,
    SetIncomingDirectory(String),
    PortalError(String),
    AcceptTransfer(String),
    DeclineTransfer(String),
    CancelTransfer(String),
}

pub struct SettingsWidgets {
    visibility_value: gtk4::Label,
    vis_hidden_btn: gtk4::Button,
    vis_paired_btn: gtk4::Button,
    vis_everyone_btn: gtk4::Button,
    status_banner: gtk4::Box,
    status_banner_label: gtk4::Label,
    pairing_card: gtk4::Box,
    pairing_peer_label: gtk4::Label,
    pairing_code_label: gtk4::Label,
    pairing_timer_label: gtk4::Label,
    send_card: gtk4::Box,
    send_paths_label: gtk4::Label,
    send_targets_box: gtk4::Box,
    send_clear_btn: gtk4::Button,
    send_submit_btn: gtk4::Button,
    send_error_banner: gtk4::Box,
    send_error_label: gtk4::Label,
    transfers_error_banner: gtk4::Box,
    transfers_error_label: gtk4::Label,
    transfers_list_box: gtk4::Box,
    nearby_list_box: gtk4::Box,
    paired_list_box: gtk4::Box,
    global_auto_accept_switch: gtk4::Switch,
    dir_value: gtk4::Label,
}

pub fn format_visibility_label(mode: VisibilityMode, remaining_secs: u64) -> String {
    match mode {
        VisibilityMode::Hidden => "Hidden".to_string(),
        VisibilityMode::PairedOnly => "Paired devices only".to_string(),
        VisibilityMode::EveryoneNearby => {
            let mins = remaining_secs / 60;
            let secs = remaining_secs % 60;
            format!("Everyone ({mins:02}:{secs:02})")
        }
    }
}

fn role_label(role: DeviceRole) -> &'static str {
    match role {
        DeviceRole::Desktop => "Desktop",
        DeviceRole::Mobile => "Mobile",
    }
}

fn clear_box_children(container: &gtk4::Box) {
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }
}

impl SettingsModel {
    pub fn from_init(init: SettingsInit) -> Self {
        let remaining = if init.config.visibility_mode == VisibilityMode::EveryoneNearby {
            init.everyone_remaining_secs
                .unwrap_or(EVERYONE_NEARBY_TIMEOUT_SECS)
        } else {
            0
        };
        let selected_send_device_id = init.selected_send_device_id.or_else(|| {
            if init.paired_devices.len() == 1 {
                Some(init.paired_devices[0].device_id.clone())
            } else {
                None
            }
        });
        Self {
            visibility_mode: init.config.visibility_mode,
            everyone_remaining_secs: remaining,
            incoming_directory: init.config.incoming_directory.clone(),
            config: init.config,
            config_path: init.config_path,
            error_message: init.error_message,
            warning_message: init.warning_message,
            transfer_error: init.transfer_error,
            send_error: None,
            nearby_peers: init.nearby_peers,
            paired_devices: init.paired_devices,
            transfers: init.transfers,
            send_paths: init.send_paths,
            selected_send_device_id,
            send_picker_active: init.send_picker_active,
            sending_in_flight: false,
            renaming_device_id: None,
            rename_draft: String::new(),
            pending_pairing: None,
            pending_unpair_id: None,
            device_store: init.device_store,
            secret_store: init.secret_store,
            ipc_socket_path: init.ipc_socket_path,
            root_window: None,
        }
    }

    #[cfg(test)]
    pub fn visibility_mode(&self) -> VisibilityMode {
        self.visibility_mode
    }

    pub fn visibility_status_text(&self) -> String {
        format_visibility_label(self.visibility_mode, self.everyone_remaining_secs)
    }

    #[cfg(test)]
    pub fn paired_devices(&self) -> &[PairedDevice] {
        &self.paired_devices
    }

    #[cfg(test)]
    pub fn transfers(&self) -> &[TransferSession] {
        &self.transfers
    }

    #[cfg(test)]
    pub fn transfer_error(&self) -> Option<&str> {
        self.transfer_error.as_deref()
    }

    #[cfg(test)]
    pub fn send_paths(&self) -> &[PathBuf] {
        &self.send_paths
    }

    #[cfg(test)]
    pub fn selected_send_device_id(&self) -> Option<&str> {
        self.selected_send_device_id.as_deref()
    }

    #[cfg(test)]
    pub fn send_error(&self) -> Option<&str> {
        self.send_error.as_deref()
    }

    #[cfg(test)]
    pub fn auto_accept_from_paired(&self) -> bool {
        self.config.auto_accept_from_paired
    }

    #[cfg(test)]
    pub fn incoming_directory(&self) -> &str {
        &self.incoming_directory
    }

    #[cfg(test)]
    pub fn pending_pairing(&self) -> Option<&PendingPairingView> {
        self.pending_pairing.as_ref()
    }

    pub fn pending_unpair_id(&self) -> Option<&str> {
        self.pending_unpair_id.as_deref()
    }

    #[cfg(test)]
    pub fn error_message(&self) -> Option<&str> {
        self.error_message.as_deref()
    }

    #[cfg(test)]
    pub fn set_pending_pairing_for_test(&mut self, prompt: PendingPairingView) {
        self.pending_pairing = Some(prompt);
    }

    fn persist_config(&mut self) {
        if let Some(ref path) = self.config_path {
            if let Err(err) = self.config.save_to_path(path) {
                self.error_message = Some(err.to_string());
                return;
            }
        }
        if let Some(ref sock_path) = self.ipc_socket_path {
            match ipc::send_request_to(
                sock_path,
                &IpcRequest::UpdateConfig {
                    config: self.config.clone(),
                },
            ) {
                Ok(IpcResponse::Config(updated)) => {
                    self.incoming_directory = updated.incoming_directory.clone();
                    self.config = updated;
                    self.transfer_error = None;
                }
                Ok(IpcResponse::Error { message }) => {
                    self.error_message = Some(message);
                }
                Ok(_) => {}
                Err(_) => {
                    // Daemon may be offline while editing config locally.
                }
            }
        }
    }

    fn apply_visibility_mode(&mut self, mode: VisibilityMode) {
        self.visibility_mode = mode;
        self.config.visibility_mode = mode;
        self.everyone_remaining_secs = if mode == VisibilityMode::EveryoneNearby {
            EVERYONE_NEARBY_TIMEOUT_SECS
        } else {
            0
        };
        if let Some(ref path) = self.config_path {
            if let Err(err) = self.config.save_to_path(path) {
                self.error_message = Some(err.to_string());
            }
        }
        if let Some(ref sock_path) = self.ipc_socket_path {
            match ipc::send_request_to(sock_path, &IpcRequest::SetVisibility { mode }) {
                Ok(IpcResponse::Status(status)) => {
                    self.visibility_mode = status.visibility_mode;
                    self.config.visibility_mode = status.visibility_mode;
                    self.everyone_remaining_secs = status.everyone_remaining_secs.unwrap_or(0);
                    self.transfer_error = None;
                }
                Ok(IpcResponse::Error { message }) => {
                    self.error_message = Some(message);
                }
                Ok(_) => {}
                Err(_) => {
                    // Daemon may be offline while configuring settings locally.
                }
            }
        }
    }

    fn refresh_from_daemon(&mut self, surface_error: bool) {
        let Some(ref sock_path) = self.ipc_socket_path else {
            if surface_error {
                self.transfer_error = Some("Daemon IPC socket is not configured.".to_string());
            }
            return;
        };
        match ipc::send_request_to(sock_path, &IpcRequest::Status) {
            Ok(IpcResponse::Status(status)) => {
                self.visibility_mode = status.visibility_mode;
                self.config.visibility_mode = status.visibility_mode;
                self.everyone_remaining_secs = status.everyone_remaining_secs.unwrap_or(0);
                self.config.auto_accept_from_paired = status.auto_accept_from_paired;
                self.incoming_directory = status.incoming_directory.clone();
                self.config.incoming_directory = status.incoming_directory;
                self.transfer_error = None;
            }
            Ok(IpcResponse::Error { message }) => {
                self.transfer_error = Some(message);
                return;
            }
            Ok(_) => {
                self.transfer_error = Some("Unexpected daemon IPC response.".to_string());
                return;
            }
            Err(err) => {
                if surface_error {
                    self.transfer_error = Some(format!(
                        "Keynear daemon unavailable ({err}); start keynear-linux and retry."
                    ));
                }
                return;
            }
        }
        match ipc::send_request_to(sock_path, &IpcRequest::ListTransfers) {
            Ok(IpcResponse::Transfers(list)) => {
                self.transfers = list;
                self.transfer_error = None;
            }
            Ok(IpcResponse::Error { message }) => {
                self.transfer_error = Some(message);
            }
            Ok(_) => {
                self.transfer_error = Some("Unexpected daemon IPC response.".to_string());
            }
            Err(err) => {
                if surface_error {
                    self.transfer_error = Some(err.to_string());
                }
            }
        }
    }

    fn refresh_transfers_from_ipc(&mut self) {
        self.refresh_from_daemon(false);
    }

    fn send_transfer_action(&mut self, request: IpcRequest) {
        let Some(ref sock_path) = self.ipc_socket_path else {
            self.transfer_error = Some("Daemon IPC socket is not configured.".to_string());
            return;
        };
        match ipc::send_request_to(sock_path, &request) {
            Ok(IpcResponse::Transfer(updated)) => {
                if let Some(existing) = self
                    .transfers
                    .iter_mut()
                    .find(|t| t.transfer_id == updated.transfer_id)
                {
                    *existing = updated;
                } else {
                    self.transfers.insert(0, updated);
                }
                self.transfer_error = None;
            }
            Ok(IpcResponse::Error { message }) => {
                self.transfer_error = Some(message);
            }
            Ok(_) => {
                self.transfer_error = Some("Unexpected daemon IPC response.".to_string());
            }
            Err(err) => {
                self.transfer_error = Some(err.to_string());
            }
        }
    }

    pub fn apply_msg(&mut self, msg: SettingsMsg) {
        match msg {
            SettingsMsg::SetVisibility(mode) => {
                self.apply_visibility_mode(mode);
            }
            SettingsMsg::Tick => {
                if self.visibility_mode == VisibilityMode::EveryoneNearby {
                    if self.everyone_remaining_secs <= 1 {
                        self.visibility_mode = VisibilityMode::PairedOnly;
                        self.everyone_remaining_secs = 0;
                    } else {
                        self.everyone_remaining_secs -= 1;
                    }
                }
                if let Some(ref mut prompt) = self.pending_pairing {
                    if prompt.remaining_secs <= 1 {
                        self.pending_pairing = None;
                        self.error_message =
                            Some("Pairing verification timed out after 120 seconds.".to_string());
                    } else {
                        prompt.remaining_secs -= 1;
                    }
                }
                self.refresh_transfers_from_ipc();
            }
            SettingsMsg::RefreshPeers => {
                match self.device_store.load() {
                    Ok(devices) => {
                        self.paired_devices = devices;
                        self.error_message = None;
                    }
                    Err(err) => {
                        self.error_message = Some(err.to_string());
                    }
                }
                self.refresh_from_daemon(false);
            }
            SettingsMsg::RetryDaemon => {
                if let Ok(devices) = self.device_store.load() {
                    self.paired_devices = devices;
                }
                self.refresh_from_daemon(true);
            }
            SettingsMsg::BeginPair(peer_token) => {
                if let Some(peer) = self
                    .nearby_peers
                    .iter()
                    .find(|p| p.presence_token == peer_token)
                {
                    let code_num: u32 = peer.presence_token.bytes().fold(100_007u32, |acc, b| {
                        acc.wrapping_mul(31).wrapping_add(u32::from(b))
                    }) % 1_000_000;
                    let raw = format!("{code_num:06}");
                    self.pending_pairing = Some(PendingPairingView {
                        peer_token: peer.presence_token.clone(),
                        peer_name: peer.display_name.clone(),
                        verification_code: format!("{} {}", &raw[..3], &raw[3..6]),
                        remaining_secs: PAIRING_TIMEOUT_SECS,
                    });
                    self.error_message = None;
                }
            }
            SettingsMsg::ConfirmPairing => {
                self.pending_pairing = None;
                self.warning_message = Some(
                    "Pairing approval queued; verify matching 6-digit code on the peer device."
                        .to_string(),
                );
            }
            SettingsMsg::CancelPairing => {
                self.pending_pairing = None;
            }
            SettingsMsg::RequestUnpair(device_id) => {
                self.pending_unpair_id = Some(device_id);
            }
            SettingsMsg::CancelUnpair => {
                self.pending_unpair_id = None;
            }
            SettingsMsg::ConfirmUnpair(device_id) => {
                if let Some(ref store) = self.secret_store {
                    if let Err(err) =
                        remove_paired_device(&self.device_store, store.as_ref(), &device_id)
                    {
                        self.error_message = Some(err.to_string());
                        return;
                    }
                } else if let Err(err) = self.device_store.remove(&device_id) {
                    self.error_message = Some(err.to_string());
                    return;
                }
                self.paired_devices.retain(|d| d.device_id != device_id);
                if self.selected_send_device_id.as_deref() == Some(device_id.as_str()) {
                    self.selected_send_device_id = None;
                }
                if self.renaming_device_id.as_deref() == Some(device_id.as_str()) {
                    self.renaming_device_id = None;
                    self.rename_draft.clear();
                }
                self.pending_unpair_id = None;
                self.error_message = None;
            }
            SettingsMsg::StartRename(device_id) => {
                if let Some(dev) = self
                    .paired_devices
                    .iter()
                    .find(|d| d.device_id == device_id)
                {
                    self.renaming_device_id = Some(dev.device_id.clone());
                    self.rename_draft = dev.display_name.clone();
                    self.error_message = None;
                }
            }
            SettingsMsg::UpdateRenameDraft(text) => {
                self.rename_draft = text;
            }
            SettingsMsg::CancelRename => {
                self.renaming_device_id = None;
                self.rename_draft.clear();
            }
            SettingsMsg::SubmitRename {
                device_id,
                display_name,
            } => {
                let clean = match validate_display_name(&display_name) {
                    Ok(clean) => clean,
                    Err(err) => {
                        self.error_message = Some(err.to_string());
                        return;
                    }
                };
                let mut renamed = match self.device_store.rename(&device_id, &clean) {
                    Ok(device) => Some(device),
                    Err(KeynearError::Validation {
                        field: "deviceId", ..
                    }) => self
                        .paired_devices
                        .iter()
                        .find(|d| d.device_id == device_id)
                        .cloned()
                        .map(|mut dev| {
                            dev.display_name = clean.clone();
                            dev
                        }),
                    Err(err) => {
                        self.error_message = Some(err.to_string());
                        return;
                    }
                };
                if let Some(ref sock_path) = self.ipc_socket_path {
                    match ipc::send_request_to(
                        sock_path,
                        &IpcRequest::RenameDevice {
                            device_id: device_id.clone(),
                            display_name: clean.clone(),
                        },
                    ) {
                        Ok(IpcResponse::Device(dev)) => renamed = Some(dev),
                        Ok(IpcResponse::Error { message }) => {
                            self.error_message = Some(message);
                            return;
                        }
                        Ok(_) => {}
                        Err(_) => {}
                    }
                }
                if let Ok(devices) = self.device_store.load() {
                    if !devices.is_empty() {
                        self.paired_devices = devices;
                    }
                }
                if let Some(dev) = renamed {
                    if let Some(slot) = self
                        .paired_devices
                        .iter_mut()
                        .find(|d| d.device_id == dev.device_id)
                    {
                        *slot = dev;
                    }
                }
                self.renaming_device_id = None;
                self.rename_draft.clear();
                self.error_message = None;
            }
            SettingsMsg::SetAutoAcceptFiles { device_id, enabled } => {
                let stored = match self.device_store.get(&device_id) {
                    Ok(Some(dev)) => Some(dev),
                    Ok(None) => self
                        .paired_devices
                        .iter()
                        .find(|d| d.device_id == device_id)
                        .cloned(),
                    Err(err) => {
                        self.error_message = Some(err.to_string());
                        return;
                    }
                };
                let Some(mut updated) = stored else {
                    self.error_message = Some(format!("Paired device '{device_id}' not found."));
                    return;
                };
                updated.auto_accept_files = enabled;
                match self.device_store.upsert(updated) {
                    Ok(devices) => {
                        self.paired_devices = devices;
                        self.error_message = None;
                    }
                    Err(err) => {
                        self.error_message = Some(err.to_string());
                        return;
                    }
                }
                if let Some(ref sock_path) = self.ipc_socket_path {
                    let _ = ipc::send_request_to(
                        sock_path,
                        &IpcRequest::SetDeviceAutoAccept { device_id, enabled },
                    );
                }
            }
            SettingsMsg::SetGlobalAutoAccept(enabled) => {
                if self.config.auto_accept_from_paired == enabled {
                    return;
                }
                self.config.auto_accept_from_paired = enabled;
                self.error_message = None;
                self.persist_config();
            }
            SettingsMsg::StageSendPaths(paths) => {
                for path in paths {
                    if !self.send_paths.contains(&path) {
                        self.send_paths.push(path);
                    }
                }
                self.send_error = None;
            }
            SettingsMsg::ClearSendPaths => {
                self.send_paths.clear();
                self.send_error = None;
            }
            SettingsMsg::SelectSendDevice(device_id) => {
                self.selected_send_device_id = Some(device_id);
                self.send_error = None;
            }
            SettingsMsg::SendCompleted(result) => {
                self.sending_in_flight = false;
                match result {
                    Ok(session) => {
                        if let Some(existing) = self
                            .transfers
                            .iter_mut()
                            .find(|t| t.transfer_id == session.transfer_id)
                        {
                            *existing = session;
                        } else {
                            self.transfers.insert(0, session);
                        }
                        self.send_paths.clear();
                        self.send_error = None;
                        self.transfer_error = None;
                    }
                    Err(err) => {
                        self.send_error = Some(err);
                    }
                }
            }
            SettingsMsg::SetIncomingDirectory(dir) => {
                let clean = dir.trim().to_string();
                if clean.is_empty() {
                    self.error_message = Some("Incoming directory must not be empty.".to_string());
                    return;
                }
                self.incoming_directory = clean.clone();
                self.config.incoming_directory = clean;
                self.error_message = None;
                self.persist_config();
            }
            SettingsMsg::PortalError(err) => {
                self.error_message = Some(err);
            }
            SettingsMsg::PickSendFiles
            | SettingsMsg::PickSendFolder
            | SettingsMsg::SendStagedFiles
            | SettingsMsg::PickIncomingDirectory => {}
            SettingsMsg::AcceptTransfer(transfer_id) => {
                self.send_transfer_action(IpcRequest::AcceptTransfer { transfer_id });
            }
            SettingsMsg::DeclineTransfer(transfer_id) => {
                self.send_transfer_action(IpcRequest::DeclineTransfer { transfer_id });
            }
            SettingsMsg::CancelTransfer(transfer_id) => {
                self.send_transfer_action(IpcRequest::CancelTransfer { transfer_id });
            }
        }
    }
}

fn install_css(window: &gtk4::Window) {
    let provider = gtk4::CssProvider::new();
    provider.load_from_data(STYLE_CSS);
    if let Some(display) = Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
    if let Some(settings) = gtk4::Settings::default() {
        if settings.is_gtk_application_prefer_dark_theme() {
            window.add_css_class("dark");
        }
    }
}

fn rebuild_nearby_rows(
    container: &gtk4::Box,
    peers: &[NearbyPeer],
    sender: &ComponentSender<SettingsModel>,
) {
    clear_box_children(container);
    if peers.is_empty() {
        let empty = gtk4::Label::builder()
            .label("No nearby devices discovered yet.")
            .xalign(0.0)
            .wrap(true)
            .build();
        empty.add_css_class("keynear-description");
        container.append(&empty);
        return;
    }

    for peer in peers {
        let row = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(12)
            .build();
        row.add_css_class("keynear-tonal");

        let info_col = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(2)
            .hexpand(true)
            .build();

        let name = gtk4::Label::builder()
            .label(&peer.display_name)
            .xalign(0.0)
            .ellipsize(EllipsizeMode::End)
            .build();
        name.add_css_class("keynear-description");

        let meta_text = match peer.battery_level {
            Some(pct) => format!(
                "{} • {} dBm • {pct}% battery",
                role_label(peer.device_role),
                peer.rssi
            ),
            None => format!("{} • {} dBm", role_label(peer.device_role), peer.rssi),
        };
        let meta = gtk4::Label::builder()
            .label(meta_text)
            .xalign(0.0)
            .ellipsize(EllipsizeMode::End)
            .build();
        meta.add_css_class("keynear-caption");

        info_col.append(&name);
        info_col.append(&meta);
        row.append(&info_col);

        if !peer.is_paired {
            let pair_btn = gtk4::Button::builder().label("Pair").build();
            pair_btn.add_css_class("keynear-primary");
            let token = peer.presence_token.clone();
            let s = sender.clone();
            pair_btn.connect_clicked(move |_| {
                s.input(SettingsMsg::BeginPair(token.clone()));
            });
            row.append(&pair_btn);
        }

        container.append(&row);
    }
}

fn paired_rows_signature(
    devices: &[PairedDevice],
    pending_unpair_id: Option<&str>,
    renaming_device_id: Option<&str>,
) -> String {
    let mut parts = Vec::with_capacity(devices.len() + 2);
    parts.push(pending_unpair_id.unwrap_or("-").to_string());
    parts.push(renaming_device_id.unwrap_or("-").to_string());
    for dev in devices {
        parts.push(format!(
            "{}:{}:{}",
            dev.device_id, dev.display_name, dev.auto_accept_files
        ));
    }
    parts.join("|")
}

fn rebuild_paired_rows(
    container: &gtk4::Box,
    devices: &[PairedDevice],
    pending_unpair_id: Option<&str>,
    renaming_device_id: Option<&str>,
    rename_draft: &str,
    sender: &ComponentSender<SettingsModel>,
) {
    let sig = paired_rows_signature(devices, pending_unpair_id, renaming_device_id);
    if container.widget_name() == sig {
        return;
    }
    container.set_widget_name(&sig);
    clear_box_children(container);
    if devices.is_empty() {
        let empty_card = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(4)
            .build();
        empty_card.add_css_class("keynear-tonal");

        let empty_title = gtk4::Label::builder()
            .label("No paired devices yet")
            .xalign(0.0)
            .wrap(true)
            .build();
        empty_title.add_css_class("keynear-heading");

        let empty_desc = gtk4::Label::builder()
            .label("Switch visibility to Everyone nearby to pair a device over Bluetooth LE.")
            .xalign(0.0)
            .wrap(true)
            .max_width_chars(65)
            .build();
        empty_desc.add_css_class("keynear-description");

        empty_card.append(&empty_title);
        empty_card.append(&empty_desc);
        container.append(&empty_card);
        return;
    }

    for device in devices {
        let outer = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(8)
            .build();
        outer.add_css_class("keynear-tonal");

        let row = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(12)
            .build();

        let info_col = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(2)
            .hexpand(true)
            .build();

        let name = gtk4::Label::builder()
            .label(&device.display_name)
            .xalign(0.0)
            .ellipsize(EllipsizeMode::End)
            .build();
        name.add_css_class("keynear-description");

        let key_prefix: String = device.static_x25519_pubkey.chars().take(12).collect();
        let meta = gtk4::Label::builder()
            .label(format!(
                "{} • Key {}…",
                role_label(device.device_role),
                key_prefix
            ))
            .xalign(0.0)
            .ellipsize(EllipsizeMode::End)
            .build();
        meta.add_css_class("keynear-caption");

        info_col.append(&name);
        info_col.append(&meta);
        row.append(&info_col);

        if pending_unpair_id == Some(device.device_id.as_str()) {
            let confirm_btn = gtk4::Button::builder().label("Confirm unpair").build();
            confirm_btn.add_css_class("keynear-danger-btn");
            let id_confirm = device.device_id.clone();
            let s_confirm = sender.clone();
            confirm_btn.connect_clicked(move |_| {
                s_confirm.input(SettingsMsg::ConfirmUnpair(id_confirm.clone()));
            });

            let cancel_btn = gtk4::Button::builder().label("Cancel").build();
            cancel_btn.add_css_class("keynear-secondary");
            let s_cancel = sender.clone();
            cancel_btn.connect_clicked(move |_| {
                s_cancel.input(SettingsMsg::CancelUnpair);
            });

            row.append(&confirm_btn);
            row.append(&cancel_btn);
        } else {
            let rename_btn = gtk4::Button::builder().label("Rename").build();
            rename_btn.add_css_class("keynear-secondary");
            let id_rename = device.device_id.clone();
            let s_rename = sender.clone();
            rename_btn.connect_clicked(move |_| {
                s_rename.input(SettingsMsg::StartRename(id_rename.clone()));
            });

            let unpair_btn = gtk4::Button::builder().label("Unpair").build();
            unpair_btn.add_css_class("keynear-secondary");
            let id_req = device.device_id.clone();
            let s_req = sender.clone();
            unpair_btn.connect_clicked(move |_| {
                s_req.input(SettingsMsg::RequestUnpair(id_req.clone()));
            });

            row.append(&rename_btn);
            row.append(&unpair_btn);
        }

        outer.append(&row);

        if renaming_device_id == Some(device.device_id.as_str()) {
            let rename_row = gtk4::Box::builder()
                .orientation(gtk4::Orientation::Horizontal)
                .spacing(8)
                .build();
            let entry = gtk4::Entry::builder()
                .text(rename_draft)
                .placeholder_text("Device name")
                .hexpand(true)
                .build();
            entry.add_css_class("keynear-entry");
            {
                let s_draft = sender.clone();
                entry.connect_changed(move |e| {
                    s_draft.input(SettingsMsg::UpdateRenameDraft(e.text().to_string()));
                });
            }
            {
                let id_enter = device.device_id.clone();
                let s_enter = sender.clone();
                entry.connect_activate(move |e| {
                    s_enter.input(SettingsMsg::SubmitRename {
                        device_id: id_enter.clone(),
                        display_name: e.text().to_string(),
                    });
                });
            }

            let save_btn = gtk4::Button::builder().label("Save").build();
            save_btn.add_css_class("keynear-primary");
            {
                let id_save = device.device_id.clone();
                let s_save = sender.clone();
                let entry_ref = entry.clone();
                save_btn.connect_clicked(move |_| {
                    s_save.input(SettingsMsg::SubmitRename {
                        device_id: id_save.clone(),
                        display_name: entry_ref.text().to_string(),
                    });
                });
            }

            let cancel_btn = gtk4::Button::builder().label("Cancel").build();
            cancel_btn.add_css_class("keynear-secondary");
            {
                let s_cancel = sender.clone();
                cancel_btn.connect_clicked(move |_| {
                    s_cancel.input(SettingsMsg::CancelRename);
                });
            }

            rename_row.append(&entry);
            rename_row.append(&save_btn);
            rename_row.append(&cancel_btn);
            outer.append(&rename_row);
        }

        let auto_accept_row = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(12)
            .build();
        let auto_accept_label = gtk4::Label::builder()
            .label("Auto-accept files")
            .xalign(0.0)
            .wrap(true)
            .hexpand(true)
            .build();
        auto_accept_label.add_css_class("keynear-description");

        let auto_accept_switch = gtk4::Switch::builder()
            .active(device.auto_accept_files)
            .valign(gtk4::Align::Center)
            .tooltip_text(format!("Auto-accept files from {}", device.display_name))
            .build();
        auto_accept_switch.add_css_class("keynear-switch");
        let device_id_for_switch = device.device_id.clone();
        let sender_for_switch = sender.clone();
        auto_accept_switch.connect_state_set(move |_, enabled| {
            sender_for_switch.input(SettingsMsg::SetAutoAcceptFiles {
                device_id: device_id_for_switch.clone(),
                enabled,
            });
            gtk4::glib::Propagation::Proceed
        });

        auto_accept_row.append(&auto_accept_label);
        auto_accept_row.append(&auto_accept_switch);
        outer.append(&auto_accept_row);

        if pending_unpair_id == Some(device.device_id.as_str()) {
            let warn = gtk4::Label::builder()
                .label("This removes the device and deletes its stored keyring secrets.")
                .xalign(0.0)
                .wrap(true)
                .build();
            warn.add_css_class("keynear-warning");
            outer.append(&warn);
        }

        container.append(&outer);
    }
}

fn rebuild_send_targets(
    container: &gtk4::Box,
    devices: &[PairedDevice],
    selected_id: Option<&str>,
    sender: &ComponentSender<SettingsModel>,
) {
    clear_box_children(container);
    if devices.is_empty() {
        let empty = gtk4::Label::builder()
            .label("Pair a device first to send files.")
            .xalign(0.0)
            .wrap(true)
            .build();
        empty.add_css_class("keynear-caption");
        container.append(&empty);
        return;
    }

    for dev in devices {
        let selected = selected_id == Some(dev.device_id.as_str());
        let btn = gtk4::Button::builder().label(&dev.display_name).build();
        btn.add_css_class(if selected {
            "keynear-primary"
        } else {
            "keynear-secondary"
        });
        let id = dev.device_id.clone();
        let s = sender.clone();
        btn.connect_clicked(move |_| {
            s.input(SettingsMsg::SelectSendDevice(id.clone()));
        });
        container.append(&btn);
    }
}

fn summarize_send_paths(paths: &[PathBuf]) -> String {
    if paths.is_empty() {
        return "No files or folders staged.".to_string();
    }
    let names: Vec<String> = paths
        .iter()
        .take(3)
        .map(|p| {
            p.file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| p.display().to_string())
        })
        .collect();
    if paths.len() <= 3 {
        format!("{} item(s): {}", paths.len(), names.join(", "))
    } else {
        format!(
            "{} items: {}, +{} more",
            paths.len(),
            names.join(", "),
            paths.len() - 3
        )
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

fn spawn_portal_picker<F>(
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

fn transfer_fraction(session: &TransferSession) -> f64 {
    if session.total_bytes == 0 {
        return match session.state {
            TransferState::Completed => 1.0,
            _ => 0.0,
        };
    }
    (session.transferred_bytes as f64 / session.total_bytes as f64).clamp(0.0, 1.0)
}

fn transfer_summary(session: &TransferSession) -> String {
    let dir = match session.direction {
        TransferDirection::Send => "To",
        TransferDirection::Receive => "From",
    };
    let state = match session.state {
        TransferState::PendingConsent => "Awaiting decision",
        TransferState::Active => "Transferring",
        TransferState::Completed => "Completed",
        TransferState::Cancelled => "Cancelled",
        TransferState::Failed => "Failed",
    };
    let item_label = if session.items.len() == 1 {
        session.items[0].name.as_str()
    } else {
        "files"
    };
    let count_suffix = if session.items.len() > 1 {
        format!(" ({} files)", session.items.len())
    } else {
        String::new()
    };
    format!(
        "{dir} {} - {item_label}{count_suffix} ({state})",
        session.peer_device_id
    )
}

fn rebuild_transfers_rows(
    container: &gtk4::Box,
    transfers: &[TransferSession],
    sender: &ComponentSender<SettingsModel>,
) {
    clear_box_children(container);
    if transfers.is_empty() {
        let empty = gtk4::Label::builder()
            .label("No active transfers")
            .xalign(0.0)
            .wrap(true)
            .build();
        empty.add_css_class("keynear-description");
        container.append(&empty);
        return;
    }

    for session in transfers {
        let row = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(8)
            .build();
        row.add_css_class("keynear-transfer-row");

        let top = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(12)
            .build();
        let title = gtk4::Label::builder()
            .label(transfer_summary(session))
            .xalign(0.0)
            .ellipsize(EllipsizeMode::End)
            .wrap(true)
            .hexpand(true)
            .build();
        title.add_css_class("keynear-description");

        let bytes_label = gtk4::Label::builder()
            .label(format!(
                "{}/{} B",
                session.transferred_bytes, session.total_bytes
            ))
            .xalign(1.0)
            .build();
        bytes_label.add_css_class("keynear-description");

        top.append(&title);
        top.append(&bytes_label);
        row.append(&top);

        let progress = gtk4::ProgressBar::builder().build();
        progress.add_css_class("keynear-progress");
        progress.set_fraction(transfer_fraction(session));
        row.append(&progress);

        let actions = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(8)
            .build();

        match session.state {
            TransferState::PendingConsent => {
                let accept_btn = gtk4::Button::with_label("Accept");
                accept_btn.add_css_class("keynear-primary");
                let accept_id = session.transfer_id.clone();
                let accept_sender = sender.clone();
                accept_btn.connect_clicked(move |_| {
                    accept_sender.input(SettingsMsg::AcceptTransfer(accept_id.clone()));
                });

                let decline_btn = gtk4::Button::with_label("Decline");
                decline_btn.add_css_class("keynear-danger-btn");
                let decline_id = session.transfer_id.clone();
                let decline_sender = sender.clone();
                decline_btn.connect_clicked(move |_| {
                    decline_sender.input(SettingsMsg::DeclineTransfer(decline_id.clone()));
                });

                actions.append(&accept_btn);
                actions.append(&decline_btn);
                row.append(&actions);
            }
            TransferState::Active => {
                let cancel_btn = gtk4::Button::with_label("Cancel");
                cancel_btn.add_css_class("keynear-secondary");
                let cancel_id = session.transfer_id.clone();
                let cancel_sender = sender.clone();
                cancel_btn.connect_clicked(move |_| {
                    cancel_sender.input(SettingsMsg::CancelTransfer(cancel_id.clone()));
                });
                actions.append(&cancel_btn);
                row.append(&actions);
            }
            _ => {}
        }

        container.append(&row);
    }
}

impl SimpleComponent for SettingsModel {
    type Init = SettingsInit;
    type Input = SettingsMsg;
    type Output = ();
    type Root = gtk4::Window;
    type Widgets = SettingsWidgets;

    fn init_root() -> Self::Root {
        gtk4::Window::builder()
            .title("Keynear Settings")
            .default_width(560)
            .resizable(true)
            .build()
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        root.add_css_class("keynear-window");
        install_css(&root);

        let mut model = Self::from_init(init);
        model.root_window = Some(root.clone());

        let scroll = gtk4::ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .propagate_natural_height(true)
            .build();

        let container = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(16)
            .margin_top(24)
            .margin_bottom(24)
            .margin_start(24)
            .margin_end(24)
            .build();

        let header = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(4)
            .build();

        let title = gtk4::Label::builder()
            .label("Keynear")
            .xalign(0.0)
            .wrap(true)
            .build();
        title.add_css_class("keynear-title");

        let subtitle = gtk4::Label::builder()
            .label("Local continuity configuration for nearby devices.")
            .xalign(0.0)
            .wrap(true)
            .max_width_chars(65)
            .build();
        subtitle.add_css_class("keynear-description");

        header.append(&title);
        header.append(&subtitle);
        container.append(&header);

        let status_banner = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(4)
            .visible(false)
            .build();
        let status_banner_label = gtk4::Label::builder().xalign(0.0).wrap(true).build();
        status_banner.append(&status_banner_label);
        container.append(&status_banner);

        let visibility_card = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(12)
            .build();
        visibility_card.add_css_class("keynear-card");

        let vis_heading = gtk4::Label::builder()
            .label("Discovery visibility")
            .xalign(0.0)
            .wrap(true)
            .build();
        vis_heading.add_css_class("keynear-heading");

        let visibility_value = gtk4::Label::builder()
            .label(model.visibility_status_text())
            .xalign(0.0)
            .wrap(true)
            .ellipsize(EllipsizeMode::End)
            .hexpand(true)
            .build();
        visibility_value.add_css_class("keynear-description");

        let vis_actions = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(8)
            .build();

        let vis_hidden_btn = gtk4::Button::builder().label("Hidden").build();
        {
            let sender = sender.clone();
            vis_hidden_btn.connect_clicked(move |_| {
                sender.input(SettingsMsg::SetVisibility(VisibilityMode::Hidden));
            });
        }

        let vis_paired_btn = gtk4::Button::builder().label("Paired only").build();
        {
            let sender = sender.clone();
            vis_paired_btn.connect_clicked(move |_| {
                sender.input(SettingsMsg::SetVisibility(VisibilityMode::PairedOnly));
            });
        }

        let vis_everyone_btn = gtk4::Button::builder().label("Everyone nearby").build();
        {
            let sender = sender.clone();
            vis_everyone_btn.connect_clicked(move |_| {
                sender.input(SettingsMsg::SetVisibility(VisibilityMode::EveryoneNearby));
            });
        }

        let refresh_btn = gtk4::Button::builder()
            .label("Refresh")
            .halign(gtk4::Align::Start)
            .build();
        refresh_btn.add_css_class("keynear-secondary");
        {
            let sender = sender.clone();
            refresh_btn.connect_clicked(move |_| {
                sender.input(SettingsMsg::RefreshPeers);
            });
        }

        vis_actions.append(&vis_hidden_btn);
        vis_actions.append(&vis_paired_btn);
        vis_actions.append(&vis_everyone_btn);
        vis_actions.append(&refresh_btn);

        visibility_card.append(&vis_heading);
        visibility_card.append(&visibility_value);
        visibility_card.append(&vis_actions);

        let send_card = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(12)
            .build();
        send_card.add_css_class("keynear-card");

        let send_heading = gtk4::Label::builder()
            .label("Send files or folders")
            .xalign(0.0)
            .build();
        send_heading.add_css_class("keynear-heading");

        let send_error_banner = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(4)
            .visible(false)
            .build();
        send_error_banner.add_css_class("keynear-error");
        let send_error_label = gtk4::Label::builder().xalign(0.0).wrap(true).build();
        send_error_banner.append(&send_error_label);

        let send_paths_label = gtk4::Label::builder()
            .label(summarize_send_paths(&model.send_paths))
            .xalign(0.0)
            .wrap(true)
            .ellipsize(EllipsizeMode::End)
            .build();
        send_paths_label.add_css_class("keynear-description");

        let send_pick_row = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(8)
            .build();
        let pick_files_btn = gtk4::Button::builder().label("Choose files...").build();
        pick_files_btn.add_css_class("keynear-secondary");
        {
            let s = sender.clone();
            pick_files_btn.connect_clicked(move |_| {
                s.input(SettingsMsg::PickSendFiles);
            });
        }

        let pick_folder_btn = gtk4::Button::builder().label("Choose folder...").build();
        pick_folder_btn.add_css_class("keynear-secondary");
        {
            let s = sender.clone();
            pick_folder_btn.connect_clicked(move |_| {
                s.input(SettingsMsg::PickSendFolder);
            });
        }

        let send_clear_btn = gtk4::Button::builder().label("Clear").build();
        send_clear_btn.add_css_class("keynear-secondary");
        {
            let s = sender.clone();
            send_clear_btn.connect_clicked(move |_| {
                s.input(SettingsMsg::ClearSendPaths);
            });
        }

        send_pick_row.append(&pick_files_btn);
        send_pick_row.append(&pick_folder_btn);
        send_pick_row.append(&send_clear_btn);

        let send_targets_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(8)
            .build();

        let send_submit_btn = gtk4::Button::builder()
            .label("Send to selected device")
            .halign(gtk4::Align::Start)
            .build();
        send_submit_btn.add_css_class("keynear-primary");
        {
            let s = sender.clone();
            send_submit_btn.connect_clicked(move |_| {
                s.input(SettingsMsg::SendStagedFiles);
            });
        }

        send_card.append(&send_heading);
        send_card.append(&send_error_banner);
        send_card.append(&send_paths_label);
        send_card.append(&send_pick_row);
        send_card.append(&send_targets_box);
        send_card.append(&send_submit_btn);

        if model.send_picker_active {
            container.append(&send_card);
            container.append(&visibility_card);
        } else {
            container.append(&visibility_card);
            container.append(&send_card);
        }

        let pairing_card = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(12)
            .visible(false)
            .build();
        pairing_card.add_css_class("keynear-card");

        let pairing_heading = gtk4::Label::builder()
            .label("Verify 6-digit pairing code")
            .xalign(0.0)
            .wrap(true)
            .build();
        pairing_heading.add_css_class("keynear-heading");

        let pairing_peer_label = gtk4::Label::builder().xalign(0.0).wrap(true).build();
        pairing_peer_label.add_css_class("keynear-description");

        let pairing_code_label = gtk4::Label::builder()
            .label("000 000")
            .halign(gtk4::Align::Start)
            .selectable(true)
            .build();
        pairing_code_label.add_css_class("keynear-code");

        let pairing_timer_label = gtk4::Label::builder().xalign(0.0).build();
        pairing_timer_label.add_css_class("keynear-caption");

        let pairing_actions = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(8)
            .build();

        let confirm_pair_btn = gtk4::Button::builder().label("Confirm").build();
        confirm_pair_btn.add_css_class("keynear-primary");
        {
            let sender = sender.clone();
            confirm_pair_btn.connect_clicked(move |_| {
                sender.input(SettingsMsg::ConfirmPairing);
            });
        }

        let cancel_pair_btn = gtk4::Button::builder().label("Cancel").build();
        cancel_pair_btn.add_css_class("keynear-secondary");
        {
            let sender = sender.clone();
            cancel_pair_btn.connect_clicked(move |_| {
                sender.input(SettingsMsg::CancelPairing);
            });
        }

        pairing_actions.append(&confirm_pair_btn);
        pairing_actions.append(&cancel_pair_btn);

        pairing_card.append(&pairing_heading);
        pairing_card.append(&pairing_peer_label);
        pairing_card.append(&pairing_code_label);
        pairing_card.append(&pairing_timer_label);
        pairing_card.append(&pairing_actions);
        container.append(&pairing_card);

        let transfers_card = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(12)
            .build();
        transfers_card.add_css_class("keynear-card");

        let transfers_heading = gtk4::Label::builder()
            .label("Active transfers")
            .xalign(0.0)
            .build();
        transfers_heading.add_css_class("keynear-heading");

        let transfers_error_banner = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(12)
            .visible(false)
            .build();
        transfers_error_banner.add_css_class("keynear-error");
        let transfers_error_label = gtk4::Label::builder()
            .xalign(0.0)
            .wrap(true)
            .hexpand(true)
            .build();
        let transfers_retry_btn = gtk4::Button::builder().label("Retry").build();
        transfers_retry_btn.add_css_class("keynear-secondary");
        {
            let s = sender.clone();
            transfers_retry_btn.connect_clicked(move |_| {
                s.input(SettingsMsg::RetryDaemon);
            });
        }
        transfers_error_banner.append(&transfers_error_label);
        transfers_error_banner.append(&transfers_retry_btn);

        let transfers_list_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(12)
            .build();

        transfers_card.append(&transfers_heading);
        transfers_card.append(&transfers_error_banner);
        transfers_card.append(&transfers_list_box);
        container.append(&transfers_card);

        let nearby_card = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(12)
            .build();
        nearby_card.add_css_class("keynear-card");

        let nearby_heading = gtk4::Label::builder()
            .label("Nearby peers")
            .xalign(0.0)
            .build();
        nearby_heading.add_css_class("keynear-heading");

        let nearby_list_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(8)
            .build();

        nearby_card.append(&nearby_heading);
        nearby_card.append(&nearby_list_box);
        container.append(&nearby_card);

        let paired_card = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(12)
            .build();
        paired_card.add_css_class("keynear-card");

        let paired_heading = gtk4::Label::builder()
            .label("Paired devices")
            .xalign(0.0)
            .build();
        paired_heading.add_css_class("keynear-heading");

        let paired_list_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(8)
            .build();

        paired_card.append(&paired_heading);
        paired_card.append(&paired_list_box);
        container.append(&paired_card);

        let config_card = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(12)
            .build();
        config_card.add_css_class("keynear-card");

        let global_auto_row = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(12)
            .build();
        let global_auto_label = gtk4::Label::builder()
            .label("Auto-accept from paired devices")
            .xalign(0.0)
            .wrap(true)
            .hexpand(true)
            .build();
        global_auto_label.add_css_class("keynear-description");

        let global_auto_accept_switch = gtk4::Switch::builder()
            .active(model.config.auto_accept_from_paired)
            .valign(gtk4::Align::Center)
            .build();
        global_auto_accept_switch.add_css_class("keynear-switch");
        {
            let s = sender.clone();
            global_auto_accept_switch.connect_state_set(move |_, enabled| {
                s.input(SettingsMsg::SetGlobalAutoAccept(enabled));
                gtk4::glib::Propagation::Proceed
            });
        }
        global_auto_row.append(&global_auto_label);
        global_auto_row.append(&global_auto_accept_switch);
        config_card.append(&global_auto_row);

        let dir_heading = gtk4::Label::builder()
            .label("Incoming folder")
            .xalign(0.0)
            .build();
        dir_heading.add_css_class("keynear-caption");

        let dir_row = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(12)
            .build();
        let dir_value = gtk4::Label::builder()
            .label(&model.incoming_directory)
            .xalign(0.0)
            .wrap(true)
            .ellipsize(EllipsizeMode::End)
            .hexpand(true)
            .build();
        dir_value.add_css_class("keynear-description");

        let pick_dir_btn = gtk4::Button::builder().label("Change folder...").build();
        pick_dir_btn.add_css_class("keynear-secondary");
        {
            let s = sender.clone();
            pick_dir_btn.connect_clicked(move |_| {
                s.input(SettingsMsg::PickIncomingDirectory);
            });
        }

        dir_row.append(&dir_value);
        dir_row.append(&pick_dir_btn);

        config_card.append(&dir_heading);
        config_card.append(&dir_row);
        container.append(&config_card);

        let close_btn = gtk4::Button::builder()
            .label("Close")
            .halign(gtk4::Align::End)
            .build();
        close_btn.add_css_class("keynear-secondary");
        {
            let window = root.clone();
            close_btn.connect_clicked(move |_| {
                window.close();
            });
        }
        container.append(&close_btn);

        scroll.set_child(Some(&container));
        root.set_child(Some(&scroll));

        {
            let sender = sender.clone();
            glib::timeout_add_seconds_local(1, move || {
                sender.input(SettingsMsg::Tick);
                glib::ControlFlow::Continue
            });
        }

        let mut widgets = SettingsWidgets {
            visibility_value,
            vis_hidden_btn,
            vis_paired_btn,
            vis_everyone_btn,
            status_banner,
            status_banner_label,
            pairing_card,
            pairing_peer_label,
            pairing_code_label,
            pairing_timer_label,
            send_card,
            send_paths_label,
            send_targets_box,
            send_clear_btn,
            send_submit_btn,
            send_error_banner,
            send_error_label,
            transfers_error_banner,
            transfers_error_label,
            transfers_list_box,
            nearby_list_box,
            paired_list_box,
            global_auto_accept_switch,
            dir_value,
        };
        model.update_view(&mut widgets, sender);
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            SettingsMsg::PickSendFiles => {
                spawn_portal_picker(
                    self.root_window.as_ref(),
                    &sender,
                    "Select files to send via Keynear",
                    true,
                    false,
                    SettingsMsg::StageSendPaths,
                );
            }
            SettingsMsg::PickSendFolder => {
                spawn_portal_picker(
                    self.root_window.as_ref(),
                    &sender,
                    "Select folder to send via Keynear",
                    false,
                    true,
                    SettingsMsg::StageSendPaths,
                );
            }
            SettingsMsg::PickIncomingDirectory => {
                spawn_portal_picker(
                    self.root_window.as_ref(),
                    &sender,
                    "Select Keynear incoming folder",
                    false,
                    true,
                    |paths| {
                        let first = paths
                            .into_iter()
                            .next()
                            .map(|p| p.display().to_string())
                            .unwrap_or_default();
                        SettingsMsg::SetIncomingDirectory(first)
                    },
                );
            }
            SettingsMsg::SendStagedFiles => {
                if self.sending_in_flight {
                    return;
                }
                if self.send_paths.is_empty() {
                    self.send_error = Some("Select at least one file or folder first.".to_string());
                    return;
                }
                let Some(device_id) = self.selected_send_device_id.clone() else {
                    self.send_error = Some("Select a paired target device first.".to_string());
                    return;
                };
                let Some(sock_path) = self.ipc_socket_path.clone() else {
                    self.send_error = Some("Daemon IPC socket is not configured.".to_string());
                    return;
                };
                let paths = self.send_paths.clone();
                self.sending_in_flight = true;
                self.send_error = None;
                let s = sender.clone();
                std::thread::spawn(move || {
                    let result = match ipc::send_request_to(
                        &sock_path,
                        &IpcRequest::SendFiles { device_id, paths },
                    ) {
                        Ok(IpcResponse::Transfer(session)) => Ok(session),
                        Ok(IpcResponse::Error { message }) => Err(message),
                        Ok(_) => Err("Unexpected daemon IPC response.".to_string()),
                        Err(err) => Err(format!(
                            "Keynear daemon unavailable ({err}); start keynear-linux and retry."
                        )),
                    };
                    s.input(SettingsMsg::SendCompleted(result));
                });
            }
            other => self.apply_msg(other),
        }
    }

    fn update_view(&self, widgets: &mut Self::Widgets, sender: ComponentSender<Self>) {
        widgets
            .visibility_value
            .set_label(&self.visibility_status_text());

        let vis_buttons = [
            (&widgets.vis_hidden_btn, VisibilityMode::Hidden),
            (&widgets.vis_paired_btn, VisibilityMode::PairedOnly),
            (&widgets.vis_everyone_btn, VisibilityMode::EveryoneNearby),
        ];
        for (btn, mode) in vis_buttons {
            btn.remove_css_class("keynear-primary");
            btn.remove_css_class("keynear-secondary");
            btn.add_css_class(if self.visibility_mode == mode {
                "keynear-primary"
            } else {
                "keynear-secondary"
            });
        }

        widgets.send_card.set_visible(
            self.send_picker_active
                || !self.paired_devices.is_empty()
                || !self.send_paths.is_empty(),
        );
        widgets
            .send_paths_label
            .set_label(&summarize_send_paths(&self.send_paths));
        widgets
            .send_clear_btn
            .set_sensitive(!self.send_paths.is_empty() && !self.sending_in_flight);
        widgets.send_submit_btn.set_sensitive(
            !self.send_paths.is_empty()
                && self.selected_send_device_id.is_some()
                && !self.sending_in_flight,
        );
        if let Some(ref err) = self.send_error {
            widgets.send_error_label.set_label(err);
            widgets.send_error_banner.set_visible(true);
        } else {
            widgets.send_error_banner.set_visible(false);
        }
        rebuild_send_targets(
            &widgets.send_targets_box,
            &self.paired_devices,
            self.selected_send_device_id.as_deref(),
            &sender,
        );

        if widgets.global_auto_accept_switch.is_active() != self.config.auto_accept_from_paired {
            widgets
                .global_auto_accept_switch
                .set_active(self.config.auto_accept_from_paired);
        }
        widgets.dir_value.set_label(&self.incoming_directory);

        if let Some(ref err) = self.error_message {
            widgets.status_banner.remove_css_class("keynear-warning");
            widgets.status_banner.add_css_class("keynear-error");
            widgets.status_banner_label.set_label(err);
            widgets.status_banner.set_visible(true);
        } else if let Some(ref warn) = self.warning_message {
            widgets.status_banner.remove_css_class("keynear-error");
            widgets.status_banner.add_css_class("keynear-warning");
            widgets.status_banner_label.set_label(warn);
            widgets.status_banner.set_visible(true);
        } else {
            widgets.status_banner.set_visible(false);
        }

        if let Some(ref prompt) = self.pending_pairing {
            widgets
                .pairing_peer_label
                .set_label(&format!("Compare this code with {}:", prompt.peer_name));
            widgets
                .pairing_code_label
                .set_label(&prompt.verification_code);
            widgets
                .pairing_timer_label
                .set_label(&format!("Expires in {}s", prompt.remaining_secs));
            widgets.pairing_card.set_visible(true);
        } else {
            widgets.pairing_card.set_visible(false);
        }

        if let Some(err) = self.transfer_error.as_deref() {
            widgets.transfers_error_label.set_label(err);
            widgets.transfers_error_banner.set_visible(true);
        } else {
            widgets.transfers_error_banner.set_visible(false);
        }

        rebuild_transfers_rows(&widgets.transfers_list_box, &self.transfers, &sender);
        rebuild_nearby_rows(&widgets.nearby_list_box, &self.nearby_peers, &sender);
        rebuild_paired_rows(
            &widgets.paired_list_box,
            &self.paired_devices,
            self.pending_unpair_id(),
            self.renaming_device_id.as_deref(),
            &self.rename_draft,
            &sender,
        );
    }
}

pub fn run_gui(init: SettingsInit) {
    let app = RelmApp::new("org.keynear.settings").with_args(Vec::new());
    app.run::<SettingsModel>(init);
}
