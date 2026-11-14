use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveTab {
    Receive,
    Send,
    Settings,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubScreen {
    DeviceDetail(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendItemKind {
    File,
    Folder,
    Clipboard,
    Text,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SendQueueItem {
    pub id: String,
    pub name: String,
    pub size_bytes: u64,
    pub kind: SendItemKind,
    pub source_path: Option<PathBuf>,
    pub text_payload: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrbitPlacement {
    pub radius_px: i32,
    pub duration_secs: u64,
    pub phase_secs: u64,
    pub offset_x: i32,
    pub offset_y: i32,
}

pub fn orbit_for(rssi: i16, index: usize, count: usize) -> OrbitPlacement {
    let clamped = f64::from(rssi.clamp(-90, -40));
    let strength = ((-clamped - 40.0) / 50.0).clamp(0.0, 1.0);
    let radius = 128.0 + strength * 48.0;
    let angle_rad =
        (index as f64 / count.max(1) as f64) * std::f64::consts::TAU - std::f64::consts::FRAC_PI_2;
    OrbitPlacement {
        radius_px: radius.round() as i32,
        duration_secs: 40,
        phase_secs: ((index as f64 / count.max(1) as f64) * 40.0).round() as u64,
        offset_x: (radius * angle_rad.cos()).round() as i32,
        offset_y: (radius * angle_rad.sin()).round() as i32,
    }
}

pub fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

pub fn format_countdown_short(secs: u64) -> String {
    let mins = secs / 60;
    let rem = secs % 60;
    format!("{mins}:{rem:02}")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingPairingView {
    pub peer_token: String,
    pub peer_name: String,
    pub verification_code: String,
    pub remaining_secs: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MapPin {
    id: String,
    display_name: String,
    state_label: String,
    rssi: i16,
    is_paired: bool,
    is_online: bool,
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
    send_queue: Vec<SendQueueItem>,
    next_queue_seq: u64,
    selected_send_device_id: Option<String>,
    sending_in_flight: bool,
    active_tab: ActiveTab,
    sub_screen: Option<SubScreen>,
    sheet_expanded: bool,
    text_composer_open: bool,
    text_draft: String,
    mime_editor_open: bool,
    new_mime_input: String,
    dark_theme: bool,
    snackbar_message: Option<String>,
    snackbar_ttl_secs: u64,
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
    SwitchTab(ActiveTab),
    NavigateBack,
    OpenDeviceDetail(String),
    ToggleSheetExpanded,
    BeginPair(String),
    ConfirmPairing,
    CancelPairing,
    ResetBleBond(String),
    RequestUnpair(String),
    ConfirmUnpair(String),
    CancelUnpair,
    #[cfg(test)]
    StartRename(String),
    UpdateRenameDraft(String),
    SubmitRename {
        device_id: String,
        display_name: String,
    },
    SetAutoAcceptFiles {
        device_id: String,
        enabled: bool,
    },
    SetGlobalAutoAccept(bool),
    ToggleClipboardSync(bool),
    ToggleClipboardImages(bool),
    ToggleOtpPassthrough(bool),
    ToggleAudioHandoff(bool),
    ToggleNotifyAutoAccept(bool),
    ToggleMimeExpanded,
    UpdateMimeInput(String),
    AddIgnoredMime,
    RemoveIgnoredMime(String),
    ToggleDarkTheme(bool),
    PickSendFiles,
    PickSendFolder,
    StageSendPaths(Vec<PathBuf>),
    StageClipboardForSend,
    StageClipboardText(String),
    ToggleTextComposer,
    UpdateTextDraft(String),
    StageTextSnippet,
    RemoveSendQueueItem(String),
    ClearSendPaths,
    #[cfg(test)]
    SelectSendDevice(String),
    SendQueueToPeer(String),
    SendCompleted(Result<TransferSession, String>),
    PickIncomingDirectory,
    SetIncomingDirectory(String),
    PortalError(String),
    AcceptTransfer(String),
    DeclineTransfer(String),
    CancelTransfer(String),
}

pub struct SettingsWidgets {
    topbar_back_btn: gtk4::Button,
    topbar_title: gtk4::Label,
    status_banner: gtk4::Box,
    status_banner_label: gtk4::Label,
    main_stack: gtk4::Stack,
    vis_hidden_btn: gtk4::Button,
    vis_paired_btn: gtk4::Button,
    vis_everyone_btn: gtk4::Button,
    self_node_circle: gtk4::Box,
    self_name_label: gtk4::Label,
    self_countdown_label: gtk4::Label,
    radar_pins_overlay: gtk4::Overlay,
    pairing_card: gtk4::Box,
    pairing_peer_label: gtk4::Label,
    pairing_code_label: gtk4::Label,
    pairing_timer_label: gtk4::Label,
    transfers_error_banner: gtk4::Box,
    transfers_error_label: gtk4::Label,
    transfers_list_box: gtk4::Box,
    sheet_count_label: gtk4::Label,
    sheet_toggle_label: gtk4::Label,
    sheet_scroll: gtk4::ScrolledWindow,
    sheet_list_box: gtk4::Box,
    send_text_tile: gtk4::Button,
    send_text_composer_box: gtk4::Box,
    send_text_entry: gtk4::Entry,
    send_queue_heading: gtk4::Label,
    send_clear_btn: gtk4::Button,
    send_queue_list_box: gtk4::Box,
    send_error_banner: gtk4::Box,
    send_error_label: gtk4::Label,
    send_targets_box: gtk4::Box,
    save_folder_btn: gtk4::Button,
    global_auto_accept_switch: gtk4::Switch,
    clipboard_sync_switch: gtk4::Switch,
    clipboard_images_switch: gtk4::Switch,
    otp_passthrough_switch: gtk4::Switch,
    mime_count_label: gtk4::Label,
    mime_chevron_label: gtk4::Label,
    mime_editor_box: gtk4::Box,
    mime_chips_box: gtk4::Box,
    mime_entry: gtk4::Entry,
    audio_handoff_switch: gtk4::Switch,
    notify_auto_accept_switch: gtk4::Switch,
    dark_theme_switch: gtk4::Switch,
    detail_icon_box: gtk4::Box,
    detail_name_label: gtk4::Label,
    detail_status_label: gtk4::Label,
    detail_name_entry: gtk4::Entry,
    detail_auto_accept_switch: gtk4::Switch,
    detail_key_value: gtk4::Label,
    detail_ble_value: gtk4::Label,
    detail_lan_value: gtk4::Label,
    detail_remove_btn: gtk4::Button,
    detail_confirm_box: gtk4::Box,
    detail_confirm_label: gtk4::Label,
    snackbar_box: gtk4::Box,
    snackbar_label: gtk4::Label,
    nav_receive_pill: gtk4::Box,
    nav_receive_text: gtk4::Label,
    nav_send_pill: gtk4::Box,
    nav_send_text: gtk4::Label,
    nav_settings_pill: gtk4::Box,
    nav_settings_text: gtk4::Label,
}

#[cfg(test)]
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

fn estimate_path_bytes(path: &Path) -> u64 {
    std::fs::metadata(path)
        .map(|m| if m.is_dir() { 4096 } else { m.len() })
        .unwrap_or(0)
}

fn item_from_path(seq: u64, path: PathBuf) -> SendQueueItem {
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

impl SettingsModel {
    pub fn from_init(init: SettingsInit) -> Self {
        let remaining = if init.config.visibility_mode == VisibilityMode::EveryoneNearby {
            init.everyone_remaining_secs
                .unwrap_or(EVERYONE_NEARBY_TIMEOUT_SECS)
        } else {
            0
        };
        let selected_send_device_id = init
            .selected_send_device_id
            .or_else(|| init.paired_devices.first().map(|dev| dev.device_id.clone()));
        let initial_tab = if init.send_picker_active || !init.send_paths.is_empty() {
            ActiveTab::Send
        } else {
            ActiveTab::Receive
        };
        let mut queue = Vec::with_capacity(init.send_paths.len());
        let mut seq = 1u64;
        for path in &init.send_paths {
            queue.push(item_from_path(seq, path.clone()));
            seq += 1;
        }
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
            send_queue: queue,
            next_queue_seq: seq,
            selected_send_device_id,
            sending_in_flight: false,
            active_tab: initial_tab,
            sub_screen: None,
            sheet_expanded: false,
            text_composer_open: false,
            text_draft: String::new(),
            mime_editor_open: false,
            new_mime_input: String::new(),
            dark_theme: if gtk4::is_initialized() {
                gtk4::Settings::default()
                    .map(|s| s.is_gtk_application_prefer_dark_theme())
                    .unwrap_or(false)
            } else {
                false
            },
            snackbar_message: None,
            snackbar_ttl_secs: 0,
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

    #[cfg(test)]
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
    pub fn send_queue(&self) -> &[SendQueueItem] {
        &self.send_queue
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

    #[cfg(test)]
    pub fn pending_unpair_id(&self) -> Option<&str> {
        self.pending_unpair_id.as_deref()
    }

    #[cfg(test)]
    pub fn error_message(&self) -> Option<&str> {
        self.error_message.as_deref()
    }

    #[cfg(test)]
    pub fn active_tab(&self) -> ActiveTab {
        self.active_tab
    }

    #[cfg(test)]
    pub fn sub_screen(&self) -> Option<&SubScreen> {
        self.sub_screen.as_ref()
    }

    #[cfg(test)]
    pub fn ignored_mime_types(&self) -> &[String] {
        &self.config.ignored_mime_types
    }

    #[cfg(test)]
    pub fn dark_theme(&self) -> bool {
        self.dark_theme
    }

    #[cfg(test)]
    pub fn set_pending_pairing_for_test(&mut self, prompt: PendingPairingView) {
        self.pending_pairing = Some(prompt);
    }

    fn show_snackbar(&mut self, message: impl Into<String>) {
        self.snackbar_message = Some(message.into());
        self.snackbar_ttl_secs = 3;
    }

    fn map_pins(&self) -> Vec<MapPin> {
        if self.visibility_mode == VisibilityMode::Hidden {
            return Vec::new();
        }
        let mut pins = Vec::new();
        for (idx, dev) in self.paired_devices.iter().enumerate() {
            let online = !dev.last_lan_address.is_empty();
            pins.push(MapPin {
                id: dev.device_id.clone(),
                display_name: dev.display_name.clone(),
                state_label: if online {
                    "Wi-Fi".to_string()
                } else {
                    "Standby".to_string()
                },
                rssi: if online { -50 - (idx as i16 * 5) } else { -76 },
                is_paired: true,
                is_online: online,
            });
        }
        if self.visibility_mode == VisibilityMode::EveryoneNearby {
            for peer in &self.nearby_peers {
                if !peer.is_paired {
                    pins.push(MapPin {
                        id: peer.presence_token.clone(),
                        display_name: peer.display_name.clone(),
                        state_label: "Tap to pair".to_string(),
                        rssi: peer.rssi,
                        is_paired: false,
                        is_online: true,
                    });
                }
            }
        } else {
            // Surface paired-mode nearby unpaired peers only if already paired or in nearby list
            for peer in &self.nearby_peers {
                if !peer.is_paired {
                    pins.push(MapPin {
                        id: peer.presence_token.clone(),
                        display_name: peer.display_name.clone(),
                        state_label: "Tap to pair".to_string(),
                        rssi: peer.rssi,
                        is_paired: false,
                        is_online: true,
                    });
                }
            }
        }
        pins
    }

    fn selected_detail_device(&self) -> Option<&PairedDevice> {
        match self.sub_screen {
            Some(SubScreen::DeviceDetail(ref device_id)) => self
                .paired_devices
                .iter()
                .find(|d| &d.device_id == device_id),
            _ => None,
        }
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
        let snack = match self.visibility_mode {
            VisibilityMode::Hidden => "Visibility set to hidden.",
            VisibilityMode::PairedOnly => "Listening for paired computers.",
            VisibilityMode::EveryoneNearby => "Visible to everyone nearby for 10 minutes.",
        };
        self.show_snackbar(snack);
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

    fn stage_text_item(&mut self, text: &str, is_clipboard: bool) {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return;
        }
        let seq = self.next_queue_seq;
        self.next_queue_seq += 1;
        let name = if is_clipboard {
            "clipboard-text.txt".to_string()
        } else {
            let preview: String = trimmed.chars().take(28).collect();
            format!("{preview}.txt")
        };
        self.send_queue.push(SendQueueItem {
            id: format!("text-{seq}"),
            name,
            size_bytes: trimmed.len() as u64,
            kind: if is_clipboard {
                SendItemKind::Clipboard
            } else {
                SendItemKind::Text
            },
            source_path: None,
            text_payload: Some(trimmed.to_string()),
        });
        self.send_error = None;
    }

    fn materialize_queue_paths(&self) -> Result<(Vec<PathBuf>, Option<PathBuf>), String> {
        let mut paths = Vec::new();
        let mut temp_dir: Option<PathBuf> = None;

        for (idx, item) in self.send_queue.iter().enumerate() {
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
        if paths.is_empty() && !self.send_paths.is_empty() {
            paths.extend(self.send_paths.iter().cloned());
        }
        Ok((paths, temp_dir))
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
                        self.show_snackbar("Everyone nearby expired. Reverted to Paired only.");
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
                if self.snackbar_ttl_secs > 0 {
                    self.snackbar_ttl_secs -= 1;
                    if self.snackbar_ttl_secs == 0 {
                        self.snackbar_message = None;
                    }
                }
                self.refresh_transfers_from_ipc();
            }
            SettingsMsg::SwitchTab(tab) => {
                self.sub_screen = None;
                self.active_tab = tab;
                self.pending_unpair_id = None;
            }
            SettingsMsg::NavigateBack => {
                self.sub_screen = None;
                self.pending_unpair_id = None;
            }
            SettingsMsg::OpenDeviceDetail(device_id) => {
                if let Some(dev) = self
                    .paired_devices
                    .iter()
                    .find(|d| d.device_id == device_id)
                {
                    self.rename_draft = dev.display_name.clone();
                    self.renaming_device_id = Some(dev.device_id.clone());
                    self.pending_unpair_id = None;
                    self.sub_screen = Some(SubScreen::DeviceDetail(dev.device_id.clone()));
                }
            }
            SettingsMsg::ToggleSheetExpanded => {
                self.sheet_expanded = !self.sheet_expanded;
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
                self.show_snackbar("Pairing approval queued.");
            }
            SettingsMsg::CancelPairing => {
                self.pending_pairing = None;
                self.show_snackbar("Pairing request rejected.");
            }
            SettingsMsg::ResetBleBond(device_id) => {
                let name = self
                    .paired_devices
                    .iter()
                    .find(|d| d.device_id == device_id)
                    .map(|d| d.display_name.clone())
                    .unwrap_or(device_id);
                self.show_snackbar(format!("Cleared Bluetooth bond for {name}."));
            }
            SettingsMsg::RequestUnpair(device_id) => {
                self.pending_unpair_id = Some(device_id);
            }
            SettingsMsg::CancelUnpair => {
                self.pending_unpair_id = None;
            }
            SettingsMsg::ConfirmUnpair(device_id) => {
                let removed_name = self
                    .paired_devices
                    .iter()
                    .find(|d| d.device_id == device_id)
                    .map(|d| d.display_name.clone())
                    .unwrap_or_else(|| "device".to_string());
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
                    self.selected_send_device_id =
                        self.paired_devices.first().map(|d| d.device_id.clone());
                }
                if self.renaming_device_id.as_deref() == Some(device_id.as_str()) {
                    self.renaming_device_id = None;
                    self.rename_draft.clear();
                }
                if matches!(self.sub_screen, Some(SubScreen::DeviceDetail(ref id)) if id == &device_id)
                {
                    self.sub_screen = None;
                }
                self.pending_unpair_id = None;
                self.error_message = None;
                self.show_snackbar(format!("Removed {removed_name}."));
            }
            #[cfg(test)]
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
                self.rename_draft = clean;
                self.error_message = None;
                self.show_snackbar("Device name saved.");
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
                if updated.auto_accept_files == enabled {
                    return;
                }
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
            SettingsMsg::ToggleClipboardSync(enabled) => {
                if self.config.clipboard_sync_enabled == enabled {
                    return;
                }
                self.config.clipboard_sync_enabled = enabled;
                self.error_message = None;
                self.persist_config();
            }
            SettingsMsg::ToggleClipboardImages(enabled) => {
                if self.config.clipboard_images_enabled == enabled {
                    return;
                }
                self.config.clipboard_images_enabled = enabled;
                self.error_message = None;
                self.persist_config();
            }
            SettingsMsg::ToggleOtpPassthrough(enabled) => {
                if self.config.otp_passthrough_enabled == enabled {
                    return;
                }
                self.config.otp_passthrough_enabled = enabled;
                self.error_message = None;
                self.persist_config();
            }
            SettingsMsg::ToggleAudioHandoff(enabled) => {
                if self.config.audio_handoff_enabled == enabled {
                    return;
                }
                self.config.audio_handoff_enabled = enabled;
                self.error_message = None;
                self.persist_config();
            }
            SettingsMsg::ToggleNotifyAutoAccept(enabled) => {
                if self.config.notify_auto_accept_complete == enabled {
                    return;
                }
                self.config.notify_auto_accept_complete = enabled;
                self.error_message = None;
                self.persist_config();
            }
            SettingsMsg::ToggleMimeExpanded => {
                self.mime_editor_open = !self.mime_editor_open;
            }
            SettingsMsg::UpdateMimeInput(raw) => {
                self.new_mime_input = raw;
            }
            SettingsMsg::AddIgnoredMime => {
                let trimmed = self.new_mime_input.trim().to_string();
                if trimmed.is_empty() || self.config.ignored_mime_types.contains(&trimmed) {
                    return;
                }
                self.config.ignored_mime_types.push(trimmed);
                self.new_mime_input.clear();
                self.error_message = None;
                self.persist_config();
            }
            SettingsMsg::RemoveIgnoredMime(mime) => {
                let before = self.config.ignored_mime_types.len();
                self.config.ignored_mime_types.retain(|m| m != &mime);
                if self.config.ignored_mime_types.len() != before {
                    self.error_message = None;
                    self.persist_config();
                }
            }
            SettingsMsg::ToggleDarkTheme(enabled) => {
                if self.dark_theme == enabled {
                    return;
                }
                self.dark_theme = enabled;
                if let Some(ref win) = self.root_window {
                    if enabled {
                        win.add_css_class("dark");
                    } else {
                        win.remove_css_class("dark");
                    }
                }
            }
            SettingsMsg::StageSendPaths(paths) => {
                for path in paths {
                    if !self.send_paths.contains(&path) {
                        self.send_paths.push(path.clone());
                    }
                    let seq = self.next_queue_seq;
                    self.next_queue_seq += 1;
                    self.send_queue.push(item_from_path(seq, path));
                }
                self.send_error = None;
            }
            SettingsMsg::StageClipboardForSend => {
                self.stage_text_item("Clipboard text snapshot", true);
            }
            SettingsMsg::StageClipboardText(text) => {
                self.stage_text_item(&text, true);
                self.show_snackbar("Clipboard text added to send queue.");
            }
            SettingsMsg::ToggleTextComposer => {
                self.text_composer_open = !self.text_composer_open;
            }
            SettingsMsg::UpdateTextDraft(text) => {
                self.text_draft = text;
            }
            SettingsMsg::StageTextSnippet => {
                let draft = self.text_draft.clone();
                if draft.trim().is_empty() {
                    return;
                }
                self.stage_text_item(&draft, false);
                self.text_draft.clear();
                self.text_composer_open = false;
            }
            SettingsMsg::RemoveSendQueueItem(id) => {
                if let Some(pos) = self.send_queue.iter().position(|item| item.id == id) {
                    let removed = self.send_queue.remove(pos);
                    if let Some(path) = removed.source_path {
                        self.send_paths.retain(|p| p != &path);
                    }
                }
            }
            SettingsMsg::ClearSendPaths => {
                self.send_paths.clear();
                self.send_queue.clear();
                self.send_error = None;
            }
            #[cfg(test)]
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
                        self.send_queue.clear();
                        self.send_error = None;
                        self.transfer_error = None;
                        self.show_snackbar("Transfer started.");
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
                self.config.incoming_directory = clean.clone();
                self.error_message = None;
                self.persist_config();
                self.show_snackbar(format!("Save folder set to {clean}."));
            }
            SettingsMsg::PortalError(err) => {
                self.error_message = Some(err);
            }
            SettingsMsg::PickSendFiles
            | SettingsMsg::PickSendFolder
            | SettingsMsg::PickIncomingDirectory
            | SettingsMsg::SendQueueToPeer(_) => {}
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

fn install_css(window: &gtk4::Window, dark_theme: bool) {
    let provider = gtk4::CssProvider::new();
    provider.load_from_data(STYLE_CSS);
    if let Some(display) = Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
    if dark_theme {
        window.add_css_class("dark");
    }
}

fn dispatch_send_for_device(
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
    let (paths, cleanup_dir) = match model.materialize_queue_paths() {
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

fn apply_overlay_offset(widget: &gtk4::Widget, dx: i32, dy: i32) {
    let px = (f64::from(dx) * 0.72).round() as i32;
    let py = (f64::from(dy) * 0.68).round() as i32;
    widget.set_halign(gtk4::Align::Center);
    widget.set_valign(gtk4::Align::Center);
    if px >= 0 {
        widget.set_margin_start(px * 2);
        widget.set_margin_end(0);
    } else {
        widget.set_margin_start(0);
        widget.set_margin_end((-px) * 2);
    }
    if py >= 0 {
        widget.set_margin_top(py * 2);
        widget.set_margin_bottom(0);
    } else {
        widget.set_margin_top(0);
        widget.set_margin_bottom((-py) * 2);
    }
}

fn rebuild_radar_pins(
    overlay: &gtk4::Overlay,
    pins: &[MapPin],
    sender: &ComponentSender<SettingsModel>,
) {
    let sig: String = pins
        .iter()
        .map(|p| format!("{}:{}:{}:{}", p.id, p.display_name, p.state_label, p.rssi))
        .collect::<Vec<_>>()
        .join("|");
    if overlay.widget_name() == sig {
        return;
    }
    overlay.set_widget_name(&sig);

    while let Some(child) = overlay.first_child() {
        overlay.remove_overlay(&child);
    }

    for (idx, pin) in pins.iter().enumerate() {
        let orbit = orbit_for(pin.rssi, idx, pins.len());
        let pin_col = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(6)
            .halign(gtk4::Align::Center)
            .build();

        let circle_btn = gtk4::Button::builder()
            .icon_name("computer-symbolic")
            .halign(gtk4::Align::Center)
            .tooltip_text(&pin.display_name)
            .build();
        circle_btn.add_css_class("kn-peer-pin");

        let s = sender.clone();
        let target_id = pin.id.clone();
        let is_paired = pin.is_paired;
        circle_btn.connect_clicked(move |_| {
            if is_paired {
                s.input(SettingsMsg::OpenDeviceDetail(target_id.clone()));
            } else {
                s.input(SettingsMsg::BeginPair(target_id.clone()));
            }
        });

        let pill = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(2)
            .halign(gtk4::Align::Center)
            .build();
        pill.add_css_class("kn-peer-pill");

        let name_lbl = gtk4::Label::builder()
            .label(&pin.display_name)
            .ellipsize(EllipsizeMode::End)
            .max_width_chars(16)
            .build();
        name_lbl.add_css_class("keynear-caption");

        let state_lbl = gtk4::Label::builder()
            .label(format!("{} · {} dBm", pin.state_label, pin.rssi))
            .build();
        state_lbl.add_css_class("keynear-caption");

        pill.append(&name_lbl);
        pill.append(&state_lbl);

        pin_col.append(&circle_btn);
        pin_col.append(&pill);
        apply_overlay_offset(pin_col.upcast_ref(), orbit.offset_x, orbit.offset_y);
        overlay.add_overlay(&pin_col);
    }
}

fn rebuild_sheet_rows(
    container: &gtk4::Box,
    pins: &[MapPin],
    is_hidden: bool,
    sender: &ComponentSender<SettingsModel>,
) {
    let sig = format!(
        "{}|{}",
        is_hidden,
        pins.iter()
            .map(|p| format!("{}:{}:{}", p.id, p.display_name, p.is_paired))
            .collect::<Vec<_>>()
            .join("|")
    );
    if container.widget_name() == sig {
        return;
    }
    container.set_widget_name(&sig);
    clear_box_children(container);

    if is_hidden || pins.is_empty() {
        let empty = gtk4::Label::builder()
            .label(if is_hidden {
                "Discovery is hidden. Select Paired or Everyone on the map above."
            } else {
                "No computers in range yet."
            })
            .xalign(0.0)
            .wrap(true)
            .build();
        empty.add_css_class("keynear-description");
        container.append(&empty);
        return;
    }

    for pin in pins {
        let row_btn = gtk4::Button::builder().build();
        row_btn.add_css_class("keynear-secondary");

        let row = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(12)
            .build();

        let badge = gtk4::Box::builder().valign(gtk4::Align::Center).build();
        badge.add_css_class("kn-loading-tonal-badge");
        let icon = gtk4::Image::from_icon_name("computer-symbolic");
        badge.append(&icon);

        let info_col = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(2)
            .hexpand(true)
            .build();
        let name = gtk4::Label::builder()
            .label(&pin.display_name)
            .xalign(0.0)
            .ellipsize(EllipsizeMode::End)
            .build();
        name.add_css_class("keynear-description");

        let meta_txt = if pin.is_paired {
            format!("{} · {} dBm", pin.state_label, pin.rssi)
        } else {
            format!("Pair · {} · {} dBm", pin.state_label, pin.rssi)
        };
        let meta = gtk4::Label::builder()
            .label(meta_txt)
            .xalign(0.0)
            .ellipsize(EllipsizeMode::End)
            .build();
        meta.add_css_class("keynear-caption");
        info_col.append(&name);
        info_col.append(&meta);

        let chev = gtk4::Image::from_icon_name("go-next-symbolic");
        row.append(&badge);
        row.append(&info_col);
        row.append(&chev);
        row_btn.set_child(Some(&row));

        let s = sender.clone();
        let target_id = pin.id.clone();
        let is_paired = pin.is_paired;
        row_btn.connect_clicked(move |_| {
            if is_paired {
                s.input(SettingsMsg::OpenDeviceDetail(target_id.clone()));
            } else {
                s.input(SettingsMsg::BeginPair(target_id.clone()));
            }
        });
        container.append(&row_btn);
    }
}

fn rebuild_send_queue_rows(
    container: &gtk4::Box,
    items: &[SendQueueItem],
    sender: &ComponentSender<SettingsModel>,
) {
    let sig: String = items
        .iter()
        .map(|i| format!("{}:{}:{}", i.id, i.name, i.size_bytes))
        .collect::<Vec<_>>()
        .join("|");
    if container.widget_name() == sig {
        return;
    }
    container.set_widget_name(&sig);
    clear_box_children(container);

    if items.is_empty() {
        let empty = gtk4::Label::builder()
            .label("No items selected. Tap File, Folder, Clipboard, or Text above.")
            .xalign(0.0)
            .wrap(true)
            .build();
        empty.add_css_class("keynear-caption");
        container.append(&empty);
        return;
    }

    for item in items {
        let row = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(10)
            .build();
        row.add_css_class("kn-queue-row");

        let icon_name = match item.kind {
            SendItemKind::Folder => "folder-symbolic",
            SendItemKind::Clipboard => "edit-paste-symbolic",
            SendItemKind::File | SendItemKind::Text => "text-x-generic-symbolic",
        };
        row.append(&gtk4::Image::from_icon_name(icon_name));

        let name = gtk4::Label::builder()
            .label(&item.name)
            .xalign(0.0)
            .ellipsize(EllipsizeMode::End)
            .hexpand(true)
            .build();
        name.add_css_class("keynear-description");

        let size_lbl = gtk4::Label::builder()
            .label(format_bytes(item.size_bytes))
            .xalign(1.0)
            .build();
        size_lbl.add_css_class("keynear-caption");

        let remove_btn = gtk4::Button::builder()
            .icon_name("window-close-symbolic")
            .tooltip_text(format!("Remove {}", item.name))
            .build();
        remove_btn.add_css_class("keynear-danger-btn");
        let item_id = item.id.clone();
        let s = sender.clone();
        remove_btn.connect_clicked(move |_| {
            s.input(SettingsMsg::RemoveSendQueueItem(item_id.clone()));
        });

        row.append(&name);
        row.append(&size_lbl);
        row.append(&remove_btn);
        container.append(&row);
    }
}

fn rebuild_send_targets(
    container: &gtk4::Box,
    devices: &[PairedDevice],
    sending_in_flight: bool,
    has_queue: bool,
    sender: &ComponentSender<SettingsModel>,
) {
    let sig = format!(
        "{sending_in_flight}:{has_queue}:{}",
        devices
            .iter()
            .map(|d| format!("{}:{}:{}", d.device_id, d.display_name, d.last_lan_address))
            .collect::<Vec<_>>()
            .join("|")
    );
    if container.widget_name() == sig {
        return;
    }
    container.set_widget_name(&sig);
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

    for (idx, dev) in devices.iter().enumerate() {
        let row = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(12)
            .build();
        row.add_css_class("keynear-tonal");

        let icon_box = gtk4::Box::builder().valign(gtk4::Align::Center).build();
        icon_box.append(&gtk4::Image::from_icon_name(match dev.device_role {
            DeviceRole::Desktop => "computer-symbolic",
            DeviceRole::Mobile => "phone-symbolic",
        }));

        let info = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(2)
            .hexpand(true)
            .build();
        let name = gtk4::Label::builder()
            .label(&dev.display_name)
            .xalign(0.0)
            .ellipsize(EllipsizeMode::End)
            .build();
        name.add_css_class("keynear-description");

        let sub = gtk4::Label::builder()
            .label(if dev.last_lan_address.is_empty() {
                "Bluetooth standby"
            } else {
                "Connected via Wi-Fi"
            })
            .xalign(0.0)
            .ellipsize(EllipsizeMode::End)
            .build();
        sub.add_css_class("keynear-caption");
        info.append(&name);
        info.append(&sub);

        let send_btn = gtk4::Button::builder()
            .label("Send")
            .sensitive(!sending_in_flight && has_queue)
            .valign(gtk4::Align::Center)
            .build();
        send_btn.add_css_class(if idx == 0 {
            "kn-loading-btn"
        } else {
            "kn-loading-tonal-btn"
        });
        let id = dev.device_id.clone();
        let s = sender.clone();
        send_btn.connect_clicked(move |_| {
            s.input(SettingsMsg::SendQueueToPeer(id.clone()));
        });

        row.append(&icon_box);
        row.append(&info);
        row.append(&send_btn);
        container.append(&row);
    }
}

fn rebuild_mime_chips(
    container: &gtk4::Box,
    mimes: &[String],
    sender: &ComponentSender<SettingsModel>,
) {
    let sig = mimes.join("|");
    if container.widget_name() == sig {
        return;
    }
    container.set_widget_name(&sig);
    clear_box_children(container);

    for mime in mimes {
        let row = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(8)
            .build();
        row.add_css_class("kn-chip");

        let lbl = gtk4::Label::builder()
            .label(mime)
            .xalign(0.0)
            .ellipsize(EllipsizeMode::End)
            .hexpand(true)
            .build();
        lbl.add_css_class("keynear-caption");

        let rm_btn = gtk4::Button::builder()
            .icon_name("window-close-symbolic")
            .tooltip_text(format!("Remove {mime}"))
            .build();
        rm_btn.add_css_class("keynear-danger-btn");
        let mime_owned = mime.clone();
        let s = sender.clone();
        rm_btn.connect_clicked(move |_| {
            s.input(SettingsMsg::RemoveIgnoredMime(mime_owned.clone()));
        });

        row.append(&lbl);
        row.append(&rm_btn);
        container.append(&row);
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
    let active_or_pending: Vec<&TransferSession> = transfers
        .iter()
        .filter(|t| {
            matches!(
                t.state,
                TransferState::PendingConsent | TransferState::Active
            )
        })
        .collect();
    if active_or_pending.is_empty() {
        container.set_visible(false);
        return;
    }
    container.set_visible(true);

    for session in active_or_pending {
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
                "{} / {}",
                format_bytes(session.transferred_bytes),
                format_bytes(session.total_bytes)
            ))
            .xalign(1.0)
            .build();
        bytes_label.add_css_class("keynear-caption");

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
                accept_btn.add_css_class("kn-loading-btn");
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

fn build_switch_row<F>(
    title: &str,
    description: &str,
    initial: bool,
    sender: &ComponentSender<SettingsModel>,
    make_msg: F,
) -> (gtk4::Box, gtk4::Switch)
where
    F: Fn(bool) -> SettingsMsg + 'static,
{
    let row = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(12)
        .build();

    let text_col = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(2)
        .hexpand(true)
        .build();
    let title_lbl = gtk4::Label::builder()
        .label(title)
        .xalign(0.0)
        .wrap(true)
        .build();
    title_lbl.add_css_class("keynear-description");
    let desc_lbl = gtk4::Label::builder()
        .label(description)
        .xalign(0.0)
        .wrap(true)
        .build();
    desc_lbl.add_css_class("keynear-caption");
    text_col.append(&title_lbl);
    text_col.append(&desc_lbl);

    let sw = gtk4::Switch::builder()
        .active(initial)
        .valign(gtk4::Align::Center)
        .tooltip_text(title)
        .build();
    sw.add_css_class("keynear-switch");
    let s = sender.clone();
    sw.connect_state_set(move |_, enabled| {
        s.input(make_msg(enabled));
        gtk4::glib::Propagation::Proceed
    });

    row.append(&text_col);
    row.append(&sw);
    (row, sw)
}

fn build_selection_tile<F>(
    icon_name: &str,
    label: &str,
    sender: &ComponentSender<SettingsModel>,
    make_msg: F,
) -> gtk4::Button
where
    F: Fn() -> SettingsMsg + 'static,
{
    let btn = gtk4::Button::builder().hexpand(true).build();
    btn.add_css_class("kn-selection-tile");
    let col = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(8)
        .halign(gtk4::Align::Center)
        .valign(gtk4::Align::Center)
        .build();
    let icon = gtk4::Image::builder()
        .icon_name(icon_name)
        .pixel_size(28)
        .build();
    let lbl = gtk4::Label::builder().label(label).build();
    lbl.add_css_class("keynear-description");
    col.append(&icon);
    col.append(&lbl);
    btn.set_child(Some(&col));
    let s = sender.clone();
    btn.connect_clicked(move |_| {
        s.input(make_msg());
    });
    btn
}

fn build_nav_tab(
    icon_name: &str,
    label: &str,
    tab: ActiveTab,
    sender: &ComponentSender<SettingsModel>,
) -> (gtk4::Button, gtk4::Box, gtk4::Label) {
    let btn = gtk4::Button::builder().hexpand(true).build();
    btn.add_css_class("kn-nav-btn");
    let col = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(4)
        .halign(gtk4::Align::Center)
        .build();

    let pill = gtk4::Box::builder().halign(gtk4::Align::Center).build();
    pill.add_css_class("kn-nav-pill-inactive");
    pill.append(&gtk4::Image::from_icon_name(icon_name));

    let text = gtk4::Label::builder().label(label).build();
    text.add_css_class("keynear-caption");

    col.append(&pill);
    col.append(&text);
    btn.set_child(Some(&col));

    let s = sender.clone();
    btn.connect_clicked(move |_| {
        s.input(SettingsMsg::SwitchTab(tab));
    });
    (btn, pill, text)
}

impl SimpleComponent for SettingsModel {
    type Init = SettingsInit;
    type Input = SettingsMsg;
    type Output = ();
    type Root = gtk4::Window;
    type Widgets = SettingsWidgets;

    fn init_root() -> Self::Root {
        gtk4::Window::builder()
            .title("Keynear")
            .default_width(520)
            .default_height(780)
            .resizable(true)
            .build()
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        root.add_css_class("keynear-window");
        let mut model = Self::from_init(init);
        install_css(&root, model.dark_theme);
        model.root_window = Some(root.clone());

        let outer_shell = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(0)
            .vexpand(true)
            .hexpand(true)
            .build();

        // TopAppBar
        let topbar = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(12)
            .build();
        topbar.add_css_class("kn-topbar");

        let topbar_back_btn = gtk4::Button::builder()
            .icon_name("go-previous-symbolic")
            .tooltip_text("Navigate back")
            .visible(false)
            .build();
        topbar_back_btn.add_css_class("kn-back-btn");
        {
            let s = sender.clone();
            topbar_back_btn.connect_clicked(move |_| {
                s.input(SettingsMsg::NavigateBack);
            });
        }

        let topbar_title = gtk4::Label::builder()
            .label("Keynear")
            .xalign(0.0)
            .ellipsize(EllipsizeMode::End)
            .hexpand(true)
            .build();
        topbar_title.add_css_class("keynear-title");

        let refresh_btn = gtk4::Button::builder()
            .icon_name("view-refresh-symbolic")
            .tooltip_text("Refresh daemon status")
            .build();
        refresh_btn.add_css_class("kn-back-btn");
        {
            let s = sender.clone();
            refresh_btn.connect_clicked(move |_| {
                s.input(SettingsMsg::RefreshPeers);
            });
        }

        topbar.append(&topbar_back_btn);
        topbar.append(&topbar_title);
        topbar.append(&refresh_btn);
        outer_shell.append(&topbar);

        let status_banner = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .margin_start(16)
            .margin_end(16)
            .margin_bottom(8)
            .visible(false)
            .build();
        let status_banner_label = gtk4::Label::builder().xalign(0.0).wrap(true).build();
        status_banner.append(&status_banner_label);
        outer_shell.append(&status_banner);

        let root_overlay = gtk4::Overlay::builder().vexpand(true).hexpand(true).build();

        let main_stack = gtk4::Stack::builder()
            .transition_type(gtk4::StackTransitionType::Crossfade)
            .transition_duration(150)
            .vexpand(true)
            .hexpand(true)
            .build();

        // ======================== RECEIVE PAGE ========================
        let receive_page = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(12)
            .margin_start(16)
            .margin_end(16)
            .margin_bottom(8)
            .vexpand(true)
            .build();

        // 3-way segmented visibility well
        let vis_well = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(6)
            .homogeneous(true)
            .build();
        vis_well.add_css_class("kn-track-well");

        let vis_hidden_btn = gtk4::Button::builder().label("Hidden").build();
        {
            let s = sender.clone();
            vis_hidden_btn.connect_clicked(move |_| {
                s.input(SettingsMsg::SetVisibility(VisibilityMode::Hidden));
            });
        }
        let vis_paired_btn = gtk4::Button::builder().label("Paired").build();
        {
            let s = sender.clone();
            vis_paired_btn.connect_clicked(move |_| {
                s.input(SettingsMsg::SetVisibility(VisibilityMode::PairedOnly));
            });
        }
        let vis_everyone_btn = gtk4::Button::builder().label("Everyone").build();
        {
            let s = sender.clone();
            vis_everyone_btn.connect_clicked(move |_| {
                s.input(SettingsMsg::SetVisibility(VisibilityMode::EveryoneNearby));
            });
        }
        vis_well.append(&vis_hidden_btn);
        vis_well.append(&vis_paired_btn);
        vis_well.append(&vis_everyone_btn);
        receive_page.append(&vis_well);

        // Radar / Orbit Map canvas
        let radar_box = gtk4::Box::builder().vexpand(true).hexpand(true).build();
        radar_box.add_css_class("kn-map-canvas");

        let radar_pins_overlay = gtk4::Overlay::builder().vexpand(true).hexpand(true).build();

        let center_node_col = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(8)
            .halign(gtk4::Align::Center)
            .valign(gtk4::Align::Center)
            .build();

        let self_node_circle = gtk4::Box::builder()
            .halign(gtk4::Align::Center)
            .valign(gtk4::Align::Center)
            .build();
        self_node_circle.add_css_class("kn-center-node-active");
        let self_icon = gtk4::Image::builder()
            .icon_name("computer-symbolic")
            .pixel_size(28)
            .halign(gtk4::Align::Center)
            .valign(gtk4::Align::Center)
            .hexpand(true)
            .build();
        self_node_circle.append(&self_icon);

        let self_pill = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(6)
            .halign(gtk4::Align::Center)
            .build();
        self_pill.add_css_class("kn-peer-pill");
        let self_name_label = gtk4::Label::builder().label("Linux desktop").build();
        self_name_label.add_css_class("keynear-caption");
        let self_countdown_label = gtk4::Label::builder().label("").visible(false).build();
        self_countdown_label.add_css_class("kn-loading-text");
        self_pill.append(&self_name_label);
        self_pill.append(&self_countdown_label);

        center_node_col.append(&self_node_circle);
        center_node_col.append(&self_pill);
        radar_pins_overlay.set_child(Some(&center_node_col));
        radar_box.append(&radar_pins_overlay);
        receive_page.append(&radar_box);

        // Inline Pairing Prompt card (if active)
        let pairing_card = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(10)
            .visible(false)
            .build();
        pairing_card.add_css_class("keynear-card");
        let pairing_heading = gtk4::Label::builder()
            .label("Verify 6-digit pairing code")
            .xalign(0.0)
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
        confirm_pair_btn.add_css_class("kn-loading-btn");
        {
            let s = sender.clone();
            confirm_pair_btn.connect_clicked(move |_| {
                s.input(SettingsMsg::ConfirmPairing);
            });
        }
        let cancel_pair_btn = gtk4::Button::builder().label("Cancel").build();
        cancel_pair_btn.add_css_class("keynear-secondary");
        {
            let s = sender.clone();
            cancel_pair_btn.connect_clicked(move |_| {
                s.input(SettingsMsg::CancelPairing);
            });
        }
        pairing_actions.append(&confirm_pair_btn);
        pairing_actions.append(&cancel_pair_btn);
        pairing_card.append(&pairing_heading);
        pairing_card.append(&pairing_peer_label);
        pairing_card.append(&pairing_code_label);
        pairing_card.append(&pairing_timer_label);
        pairing_card.append(&pairing_actions);
        receive_page.append(&pairing_card);

        // Inline daemon / active transfers box on Receive
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
        receive_page.append(&transfers_error_banner);

        let transfers_list_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(8)
            .visible(false)
            .build();
        receive_page.append(&transfers_list_box);

        // Bottom expandable Devices in range sheet
        let sheet_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(8)
            .build();
        sheet_box.add_css_class("kn-bottom-sheet");

        let sheet_header_btn = gtk4::Button::builder().build();
        sheet_header_btn.add_css_class("keynear-secondary");
        let sheet_hdr_col = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(8)
            .build();
        let sheet_handle = gtk4::Box::builder().halign(gtk4::Align::Center).build();
        sheet_handle.add_css_class("kn-sheet-handle");
        let sheet_row = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(8)
            .build();
        let sheet_count_label = gtk4::Label::builder()
            .label("0 computer(s) in range")
            .xalign(0.0)
            .hexpand(true)
            .build();
        sheet_count_label.add_css_class("keynear-description");
        let sheet_toggle_label = gtk4::Label::builder().label("All devices").build();
        sheet_toggle_label.add_css_class("kn-loading-text");
        sheet_row.append(&sheet_count_label);
        sheet_row.append(&sheet_toggle_label);
        sheet_hdr_col.append(&sheet_handle);
        sheet_hdr_col.append(&sheet_row);
        sheet_header_btn.set_child(Some(&sheet_hdr_col));
        {
            let s = sender.clone();
            sheet_header_btn.connect_clicked(move |_| {
                s.input(SettingsMsg::ToggleSheetExpanded);
            });
        }

        let sheet_scroll = gtk4::ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .min_content_height(132)
            .max_content_height(132)
            .build();
        let sheet_list_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(6)
            .build();
        sheet_scroll.set_child(Some(&sheet_list_box));

        sheet_box.append(&sheet_header_btn);
        sheet_box.append(&sheet_scroll);
        receive_page.append(&sheet_box);
        main_stack.add_named(&receive_page, Some("receive"));

        // ======================== SEND PAGE ========================
        let send_scroll = gtk4::ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .vexpand(true)
            .build();
        let send_page = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(16)
            .margin_start(16)
            .margin_end(16)
            .margin_top(8)
            .margin_bottom(24)
            .build();

        let sel_heading = gtk4::Label::builder()
            .label("SELECTION")
            .xalign(0.0)
            .build();
        sel_heading.add_css_class("kn-section-heading");
        send_page.append(&sel_heading);

        let tile_grid = gtk4::Grid::builder()
            .row_spacing(12)
            .column_spacing(12)
            .column_homogeneous(true)
            .build();
        let file_tile = build_selection_tile("text-x-generic-symbolic", "File", &sender, || {
            SettingsMsg::PickSendFiles
        });
        let folder_tile = build_selection_tile("folder-symbolic", "Folder", &sender, || {
            SettingsMsg::PickSendFolder
        });
        let clip_tile = build_selection_tile("edit-paste-symbolic", "Clipboard", &sender, || {
            SettingsMsg::StageClipboardForSend
        });
        let send_text_tile = build_selection_tile("list-add-symbolic", "Text", &sender, || {
            SettingsMsg::ToggleTextComposer
        });
        tile_grid.attach(&file_tile, 0, 0, 1, 1);
        tile_grid.attach(&folder_tile, 1, 0, 1, 1);
        tile_grid.attach(&clip_tile, 0, 1, 1, 1);
        tile_grid.attach(&send_text_tile, 1, 1, 1, 1);
        send_page.append(&tile_grid);

        let send_text_composer_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(8)
            .visible(false)
            .build();
        send_text_composer_box.add_css_class("keynear-card");
        let send_text_entry = gtk4::Entry::builder()
            .placeholder_text("Paste URL or note")
            .hexpand(true)
            .build();
        send_text_entry.add_css_class("keynear-entry");
        {
            let s = sender.clone();
            send_text_entry.connect_changed(move |e| {
                s.input(SettingsMsg::UpdateTextDraft(e.text().to_string()));
            });
        }
        {
            let s = sender.clone();
            send_text_entry.connect_activate(move |_| {
                s.input(SettingsMsg::StageTextSnippet);
            });
        }
        let send_text_add_btn = gtk4::Button::builder().label("Add").build();
        send_text_add_btn.add_css_class("kn-loading-btn");
        {
            let s = sender.clone();
            send_text_add_btn.connect_clicked(move |_| {
                s.input(SettingsMsg::StageTextSnippet);
            });
        }
        send_text_composer_box.append(&send_text_entry);
        send_text_composer_box.append(&send_text_add_btn);
        send_page.append(&send_text_composer_box);

        let queue_card = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(10)
            .build();
        queue_card.add_css_class("keynear-card");

        let queue_hdr = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(8)
            .build();
        let send_queue_heading = gtk4::Label::builder()
            .label("0 item(s) (0 B)")
            .xalign(0.0)
            .hexpand(true)
            .build();
        send_queue_heading.add_css_class("keynear-description");
        let send_clear_btn = gtk4::Button::builder().label("Clear all").build();
        send_clear_btn.add_css_class("keynear-danger-btn");
        {
            let s = sender.clone();
            send_clear_btn.connect_clicked(move |_| {
                s.input(SettingsMsg::ClearSendPaths);
            });
        }
        queue_hdr.append(&send_queue_heading);
        queue_hdr.append(&send_clear_btn);
        queue_card.append(&queue_hdr);

        let send_error_banner = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .visible(false)
            .build();
        send_error_banner.add_css_class("keynear-error");
        let send_error_label = gtk4::Label::builder().xalign(0.0).wrap(true).build();
        send_error_banner.append(&send_error_label);
        queue_card.append(&send_error_banner);

        let send_queue_list_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(8)
            .build();
        queue_card.append(&send_queue_list_box);
        send_page.append(&queue_card);

        let send_to_heading = gtk4::Label::builder().label("SEND TO").xalign(0.0).build();
        send_to_heading.add_css_class("kn-section-heading");
        send_page.append(&send_to_heading);

        let targets_card = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(8)
            .build();
        targets_card.add_css_class("keynear-card");
        let send_targets_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(8)
            .build();
        targets_card.append(&send_targets_box);
        send_page.append(&targets_card);

        send_scroll.set_child(Some(&send_page));
        main_stack.add_named(&send_scroll, Some("send"));

        // ======================== SETTINGS PAGE ========================
        let settings_scroll = gtk4::ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .vexpand(true)
            .build();
        let settings_page = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(16)
            .margin_start(16)
            .margin_end(16)
            .margin_top(8)
            .margin_bottom(24)
            .build();

        // GENERAL & STORAGE
        let gen_heading = gtk4::Label::builder()
            .label("GENERAL & STORAGE")
            .xalign(0.0)
            .build();
        gen_heading.add_css_class("kn-section-heading");
        settings_page.append(&gen_heading);

        let gen_card = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(12)
            .build();
        gen_card.add_css_class("keynear-card");

        let folder_row = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(12)
            .build();
        let folder_col = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(2)
            .hexpand(true)
            .build();
        let folder_title = gtk4::Label::builder()
            .label("Save folder")
            .xalign(0.0)
            .build();
        folder_title.add_css_class("keynear-description");
        let folder_desc = gtk4::Label::builder()
            .label("Where received files are saved")
            .xalign(0.0)
            .build();
        folder_desc.add_css_class("keynear-caption");
        folder_col.append(&folder_title);
        folder_col.append(&folder_desc);
        let save_folder_btn = gtk4::Button::builder()
            .label(&model.incoming_directory)
            .valign(gtk4::Align::Center)
            .build();
        save_folder_btn.add_css_class("keynear-secondary");
        {
            let s = sender.clone();
            save_folder_btn.connect_clicked(move |_| {
                s.input(SettingsMsg::PickIncomingDirectory);
            });
        }
        folder_row.append(&folder_col);
        folder_row.append(&save_folder_btn);
        gen_card.append(&folder_row);

        let (global_auto_row, global_auto_accept_switch) = build_switch_row(
            "Auto-accept from paired devices",
            "Save files from paired devices without asking",
            model.config.auto_accept_from_paired,
            &sender,
            SettingsMsg::SetGlobalAutoAccept,
        );
        gen_card.append(&global_auto_row);
        settings_page.append(&gen_card);

        // CLIPBOARD
        let clip_heading = gtk4::Label::builder()
            .label("CLIPBOARD")
            .xalign(0.0)
            .build();
        clip_heading.add_css_class("kn-section-heading");
        settings_page.append(&clip_heading);

        let clip_card = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(12)
            .build();
        clip_card.add_css_class("keynear-card");

        let (clip_sync_row, clipboard_sync_switch) = build_switch_row(
            "Clipboard sync",
            "Keep clipboard text in sync",
            model.config.clipboard_sync_enabled,
            &sender,
            SettingsMsg::ToggleClipboardSync,
        );
        let (clip_img_row, clipboard_images_switch) = build_switch_row(
            "Sync clipboard images",
            "Include images in clipboard sync",
            model.config.clipboard_images_enabled,
            &sender,
            SettingsMsg::ToggleClipboardImages,
        );
        let (otp_row, otp_passthrough_switch) = build_switch_row(
            "2FA code pass-through",
            "Pass verification codes to the computer",
            model.config.otp_passthrough_enabled,
            &sender,
            SettingsMsg::ToggleOtpPassthrough,
        );
        clip_card.append(&clip_sync_row);
        clip_card.append(&clip_img_row);
        clip_card.append(&otp_row);

        let mime_header_btn = gtk4::Button::builder().build();
        mime_header_btn.add_css_class("keynear-secondary");
        let mime_hdr_row = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(10)
            .build();
        let mime_hdr_col = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(2)
            .hexpand(true)
            .build();
        let mime_title = gtk4::Label::builder()
            .label("Ignored clipboard formats")
            .xalign(0.0)
            .build();
        mime_title.add_css_class("keynear-description");
        let mime_desc = gtk4::Label::builder()
            .label("Formats that are never shared")
            .xalign(0.0)
            .build();
        mime_desc.add_css_class("keynear-caption");
        mime_hdr_col.append(&mime_title);
        mime_hdr_col.append(&mime_desc);
        let mime_count_label = gtk4::Label::builder().label("0").build();
        mime_count_label.add_css_class("keynear-caption");
        let mime_chevron_label = gtk4::Label::builder().label("v").build();
        mime_chevron_label.add_css_class("keynear-caption");
        mime_hdr_row.append(&mime_hdr_col);
        mime_hdr_row.append(&mime_count_label);
        mime_hdr_row.append(&mime_chevron_label);
        mime_header_btn.set_child(Some(&mime_hdr_row));
        {
            let s = sender.clone();
            mime_header_btn.connect_clicked(move |_| {
                s.input(SettingsMsg::ToggleMimeExpanded);
            });
        }
        clip_card.append(&mime_header_btn);

        let mime_editor_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(10)
            .visible(false)
            .build();
        let mime_chips_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(6)
            .build();
        let mime_form = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(8)
            .build();
        let mime_entry = gtk4::Entry::builder()
            .placeholder_text("application/x-secret")
            .hexpand(true)
            .build();
        mime_entry.add_css_class("keynear-entry");
        {
            let s = sender.clone();
            mime_entry.connect_changed(move |e| {
                s.input(SettingsMsg::UpdateMimeInput(e.text().to_string()));
            });
        }
        {
            let s = sender.clone();
            mime_entry.connect_activate(move |_| {
                s.input(SettingsMsg::AddIgnoredMime);
            });
        }
        let mime_add_btn = gtk4::Button::builder().label("Add").build();
        mime_add_btn.add_css_class("kn-loading-tonal-btn");
        {
            let s = sender.clone();
            mime_add_btn.connect_clicked(move |_| {
                s.input(SettingsMsg::AddIgnoredMime);
            });
        }
        mime_form.append(&mime_entry);
        mime_form.append(&mime_add_btn);
        mime_editor_box.append(&mime_chips_box);
        mime_editor_box.append(&mime_form);
        clip_card.append(&mime_editor_box);
        settings_page.append(&clip_card);

        // AUDIO & NOTIFICATIONS
        let audio_heading = gtk4::Label::builder()
            .label("AUDIO & NOTIFICATIONS")
            .xalign(0.0)
            .build();
        audio_heading.add_css_class("kn-section-heading");
        settings_page.append(&audio_heading);

        let audio_card = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(12)
            .build();
        audio_card.add_css_class("keynear-card");
        let (audio_row, audio_handoff_switch) = build_switch_row(
            "Smart audio handoff",
            "Move audio between devices",
            model.config.audio_handoff_enabled,
            &sender,
            SettingsMsg::ToggleAudioHandoff,
        );
        let (notify_row, notify_auto_accept_switch) = build_switch_row(
            "Completion notifications",
            "Notify when files finish saving",
            model.config.notify_auto_accept_complete,
            &sender,
            SettingsMsg::ToggleNotifyAutoAccept,
        );
        audio_card.append(&audio_row);
        audio_card.append(&notify_row);
        settings_page.append(&audio_card);

        // APPEARANCE
        let app_heading = gtk4::Label::builder()
            .label("APPEARANCE")
            .xalign(0.0)
            .build();
        app_heading.add_css_class("kn-section-heading");
        settings_page.append(&app_heading);

        let app_card = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(12)
            .build();
        app_card.add_css_class("keynear-card");
        let (dark_row, dark_theme_switch) = build_switch_row(
            "Dark theme",
            "Switch the app color theme",
            model.dark_theme,
            &sender,
            SettingsMsg::ToggleDarkTheme,
        );
        app_card.append(&dark_row);
        settings_page.append(&app_card);

        settings_scroll.set_child(Some(&settings_page));
        main_stack.add_named(&settings_scroll, Some("settings"));

        // ======================== DEVICE DETAIL SUBSCREEN ========================
        let detail_scroll = gtk4::ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .vexpand(true)
            .build();
        let detail_page = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(14)
            .margin_start(16)
            .margin_end(16)
            .margin_top(8)
            .margin_bottom(24)
            .build();

        let detail_top_card = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(14)
            .build();
        detail_top_card.add_css_class("keynear-card");
        let detail_icon_box = gtk4::Box::builder().valign(gtk4::Align::Center).build();
        detail_icon_box.add_css_class("kn-loading-tonal-badge");
        detail_icon_box.append(
            &gtk4::Image::builder()
                .icon_name("computer-symbolic")
                .pixel_size(28)
                .build(),
        );

        let detail_top_col = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(4)
            .hexpand(true)
            .build();
        let detail_name_label = gtk4::Label::builder()
            .xalign(0.0)
            .ellipsize(EllipsizeMode::End)
            .build();
        detail_name_label.add_css_class("keynear-heading");
        let detail_status_label = gtk4::Label::builder().xalign(0.0).wrap(true).build();
        detail_status_label.add_css_class("keynear-caption");
        detail_top_col.append(&detail_name_label);
        detail_top_col.append(&detail_status_label);
        detail_top_card.append(&detail_icon_box);
        detail_top_card.append(&detail_top_col);
        detail_page.append(&detail_top_card);

        let detail_mid_card = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(14)
            .build();
        detail_mid_card.add_css_class("keynear-card");

        let detail_name_lbl = gtk4::Label::builder()
            .label("Computer name")
            .xalign(0.0)
            .build();
        detail_name_lbl.add_css_class("keynear-caption");
        let detail_name_row = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(8)
            .build();
        let detail_name_entry = gtk4::Entry::builder()
            .placeholder_text("Computer name")
            .hexpand(true)
            .build();
        detail_name_entry.add_css_class("keynear-entry");
        {
            let s = sender.clone();
            detail_name_entry.connect_changed(move |e| {
                s.input(SettingsMsg::UpdateRenameDraft(e.text().to_string()));
            });
        }
        let detail_save_btn = gtk4::Button::builder().label("Save").build();
        detail_save_btn.add_css_class("kn-loading-btn");
        {
            let s = sender.clone();
            let entry_ref = detail_name_entry.clone();
            detail_save_btn.connect_clicked(move |_| {
                let id = entry_ref.widget_name().to_string();
                if !id.is_empty() {
                    s.input(SettingsMsg::SubmitRename {
                        device_id: id,
                        display_name: entry_ref.text().to_string(),
                    });
                }
            });
        }
        detail_name_row.append(&detail_name_entry);
        detail_name_row.append(&detail_save_btn);
        detail_mid_card.append(&detail_name_lbl);
        detail_mid_card.append(&detail_name_row);

        let detail_auto_row = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(12)
            .build();
        let detail_auto_col = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(2)
            .hexpand(true)
            .build();
        let detail_auto_t = gtk4::Label::builder()
            .label("Auto-accept files")
            .xalign(0.0)
            .build();
        detail_auto_t.add_css_class("keynear-description");
        let detail_auto_d = gtk4::Label::builder()
            .label("Save files from this device without asking")
            .xalign(0.0)
            .build();
        detail_auto_d.add_css_class("keynear-caption");
        detail_auto_col.append(&detail_auto_t);
        detail_auto_col.append(&detail_auto_d);
        let detail_auto_accept_switch = gtk4::Switch::builder().valign(gtk4::Align::Center).build();
        detail_auto_accept_switch.add_css_class("keynear-switch");
        {
            let s = sender.clone();
            let entry_ref = detail_name_entry.clone();
            detail_auto_accept_switch.connect_state_set(move |_, enabled| {
                let id = entry_ref.widget_name().to_string();
                if !id.is_empty() {
                    s.input(SettingsMsg::SetAutoAcceptFiles {
                        device_id: id,
                        enabled,
                    });
                }
                gtk4::glib::Propagation::Proceed
            });
        }
        detail_auto_row.append(&detail_auto_col);
        detail_auto_row.append(&detail_auto_accept_switch);
        detail_mid_card.append(&detail_auto_row);

        let (detail_key_row, detail_key_value) = meta_row("Key fingerprint");
        let (detail_ble_row, detail_ble_value) = meta_row("Bluetooth address");
        let (detail_lan_row, detail_lan_value) = meta_row("Local Wi-Fi endpoint");
        detail_mid_card.append(&detail_key_row);
        detail_mid_card.append(&detail_ble_row);
        detail_mid_card.append(&detail_lan_row);
        detail_page.append(&detail_mid_card);

        let reset_ble_btn = gtk4::Button::builder()
            .label("Reset Bluetooth bond")
            .build();
        reset_ble_btn.add_css_class("keynear-secondary");
        {
            let s = sender.clone();
            let entry_ref = detail_name_entry.clone();
            reset_ble_btn.connect_clicked(move |_| {
                let id = entry_ref.widget_name().to_string();
                if !id.is_empty() {
                    s.input(SettingsMsg::ResetBleBond(id));
                }
            });
        }
        detail_page.append(&reset_ble_btn);

        let detail_remove_btn = gtk4::Button::builder().label("Remove device").build();
        detail_remove_btn.add_css_class("keynear-danger-btn");
        {
            let s = sender.clone();
            let entry_ref = detail_name_entry.clone();
            detail_remove_btn.connect_clicked(move |_| {
                let id = entry_ref.widget_name().to_string();
                if !id.is_empty() {
                    s.input(SettingsMsg::RequestUnpair(id));
                }
            });
        }
        detail_page.append(&detail_remove_btn);

        let detail_confirm_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(10)
            .visible(false)
            .build();
        detail_confirm_box.add_css_class("keynear-error");
        let detail_confirm_label = gtk4::Label::builder().xalign(0.0).wrap(true).build();
        let detail_confirm_actions = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(8)
            .homogeneous(true)
            .build();
        let detail_confirm_btn = gtk4::Button::builder().label("Remove device").build();
        detail_confirm_btn.add_css_class("kn-error-btn");
        {
            let s = sender.clone();
            let entry_ref = detail_name_entry.clone();
            detail_confirm_btn.connect_clicked(move |_| {
                let id = entry_ref.widget_name().to_string();
                if !id.is_empty() {
                    s.input(SettingsMsg::ConfirmUnpair(id));
                }
            });
        }
        let detail_cancel_btn = gtk4::Button::builder().label("Cancel").build();
        detail_cancel_btn.add_css_class("keynear-secondary");
        {
            let s = sender.clone();
            detail_cancel_btn.connect_clicked(move |_| {
                s.input(SettingsMsg::CancelUnpair);
            });
        }
        detail_confirm_actions.append(&detail_confirm_btn);
        detail_confirm_actions.append(&detail_cancel_btn);
        detail_confirm_box.append(&detail_confirm_label);
        detail_confirm_box.append(&detail_confirm_actions);
        detail_page.append(&detail_confirm_box);

        detail_scroll.set_child(Some(&detail_page));
        main_stack.add_named(&detail_scroll, Some("deviceDetail"));

        root_overlay.set_child(Some(&main_stack));

        // Floating Snackbar overlay
        let snackbar_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .halign(gtk4::Align::Center)
            .valign(gtk4::Align::End)
            .margin_bottom(14)
            .visible(false)
            .build();
        snackbar_box.add_css_class("kn-snackbar-pill");
        let snackbar_label = gtk4::Label::builder().label("").wrap(true).build();
        snackbar_box.append(&snackbar_label);
        root_overlay.add_overlay(&snackbar_box);

        outer_shell.append(&root_overlay);

        // ======================== BOTTOM NAVIGATION ========================
        let bottom_nav = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(8)
            .homogeneous(true)
            .build();
        bottom_nav.add_css_class("kn-bottom-nav");

        let (nav_receive_btn, nav_receive_pill, nav_receive_text) = build_nav_tab(
            "folder-download-symbolic",
            "Receive",
            ActiveTab::Receive,
            &sender,
        );
        let (nav_send_btn, nav_send_pill, nav_send_text) =
            build_nav_tab("mail-send-symbolic", "Send", ActiveTab::Send, &sender);
        let (nav_settings_btn, nav_settings_pill, nav_settings_text) = build_nav_tab(
            "emblem-system-symbolic",
            "Settings",
            ActiveTab::Settings,
            &sender,
        );

        bottom_nav.append(&nav_receive_btn);
        bottom_nav.append(&nav_send_btn);
        bottom_nav.append(&nav_settings_btn);
        outer_shell.append(&bottom_nav);

        root.set_child(Some(&outer_shell));

        {
            let s = sender.clone();
            glib::timeout_add_seconds_local(1, move || {
                s.input(SettingsMsg::Tick);
                glib::ControlFlow::Continue
            });
        }

        let mut widgets = SettingsWidgets {
            topbar_back_btn,
            topbar_title,
            status_banner,
            status_banner_label,
            main_stack,
            vis_hidden_btn,
            vis_paired_btn,
            vis_everyone_btn,
            self_node_circle,
            self_name_label,
            self_countdown_label,
            radar_pins_overlay,
            pairing_card,
            pairing_peer_label,
            pairing_code_label,
            pairing_timer_label,
            transfers_error_banner,
            transfers_error_label,
            transfers_list_box,
            sheet_count_label,
            sheet_toggle_label,
            sheet_scroll,
            sheet_list_box,
            send_text_tile,
            send_text_composer_box,
            send_text_entry,
            send_queue_heading,
            send_clear_btn,
            send_queue_list_box,
            send_error_banner,
            send_error_label,
            send_targets_box,
            save_folder_btn,
            global_auto_accept_switch,
            clipboard_sync_switch,
            clipboard_images_switch,
            otp_passthrough_switch,
            mime_count_label,
            mime_chevron_label,
            mime_editor_box,
            mime_chips_box,
            mime_entry,
            audio_handoff_switch,
            notify_auto_accept_switch,
            dark_theme_switch,
            detail_icon_box,
            detail_name_label,
            detail_status_label,
            detail_name_entry,
            detail_auto_accept_switch,
            detail_key_value,
            detail_ble_value,
            detail_lan_value,
            detail_remove_btn,
            detail_confirm_box,
            detail_confirm_label,
            snackbar_box,
            snackbar_label,
            nav_receive_pill,
            nav_receive_text,
            nav_send_pill,
            nav_send_text,
            nav_settings_pill,
            nav_settings_text,
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
            SettingsMsg::StageClipboardForSend => {
                if let Some(display) = Display::default() {
                    let s = sender.clone();
                    display
                        .clipboard()
                        .read_text_async(gtk4::gio::Cancellable::NONE, move |res| {
                            let text = res
                                .ok()
                                .flatten()
                                .map(|g| g.to_string())
                                .unwrap_or_default();
                            if text.trim().is_empty() {
                                s.input(SettingsMsg::PortalError(
                                    "Clipboard does not contain text to stage.".to_string(),
                                ));
                            } else {
                                s.input(SettingsMsg::StageClipboardText(text));
                            }
                        });
                } else {
                    self.apply_msg(SettingsMsg::StageClipboardForSend);
                }
            }
            SettingsMsg::SendQueueToPeer(device_id) => {
                dispatch_send_for_device(self, device_id, &sender);
            }
            other => self.apply_msg(other),
        }
    }

    fn update_view(&self, widgets: &mut Self::Widgets, sender: ComponentSender<Self>) {
        // TopAppBar title & back button
        let title_str = match self.sub_screen {
            Some(SubScreen::DeviceDetail(_)) => self
                .selected_detail_device()
                .map(|d| d.display_name.as_str())
                .unwrap_or("Device details"),
            None => match self.active_tab {
                ActiveTab::Receive => "Keynear",
                ActiveTab::Send => "Send files",
                ActiveTab::Settings => "Settings",
            },
        };
        widgets.topbar_title.set_label(title_str);
        widgets
            .topbar_back_btn
            .set_visible(self.sub_screen.is_some());

        let stack_page = match self.sub_screen {
            Some(SubScreen::DeviceDetail(_)) => "deviceDetail",
            None => match self.active_tab {
                ActiveTab::Receive => "receive",
                ActiveTab::Send => "send",
                ActiveTab::Settings => "settings",
            },
        };
        if widgets.main_stack.visible_child_name().as_deref() != Some(stack_page) {
            widgets.main_stack.set_visible_child_name(stack_page);
        }

        // Top warning/error banner
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

        // Receive visibility segmented buttons
        let vis_buttons = [
            (&widgets.vis_hidden_btn, VisibilityMode::Hidden),
            (&widgets.vis_paired_btn, VisibilityMode::PairedOnly),
            (&widgets.vis_everyone_btn, VisibilityMode::EveryoneNearby),
        ];
        for (btn, mode) in vis_buttons {
            btn.remove_css_class("kn-seg-btn-active");
            btn.remove_css_class("kn-seg-btn");
            btn.add_css_class(if self.visibility_mode == mode {
                "kn-seg-btn-active"
            } else {
                "kn-seg-btn"
            });
        }

        // Center node active vs hidden + Everyone countdown
        widgets
            .self_node_circle
            .remove_css_class("kn-center-node-active");
        widgets
            .self_node_circle
            .remove_css_class("kn-center-node-hidden");
        widgets
            .self_node_circle
            .add_css_class(if self.visibility_mode == VisibilityMode::Hidden {
                "kn-center-node-hidden"
            } else {
                "kn-center-node-active"
            });
        let _ = &widgets.self_name_label;
        if self.visibility_mode == VisibilityMode::EveryoneNearby {
            widgets
                .self_countdown_label
                .set_label(&format_countdown_short(self.everyone_remaining_secs));
            widgets.self_countdown_label.set_visible(true);
        } else {
            widgets.self_countdown_label.set_visible(false);
        }

        let pins = self.map_pins();
        rebuild_radar_pins(&widgets.radar_pins_overlay, &pins, &sender);

        // Inline pairing prompt
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

        // Expandable sheet
        widgets
            .sheet_count_label
            .set_label(&format!("{} computer(s) in range", pins.len()));
        widgets
            .sheet_toggle_label
            .set_label(if self.sheet_expanded {
                "Collapse"
            } else {
                "All devices"
            });
        let target_h = if self.sheet_expanded { 240 } else { 132 };
        widgets.sheet_scroll.set_min_content_height(target_h);
        widgets.sheet_scroll.set_max_content_height(target_h);
        rebuild_sheet_rows(
            &widgets.sheet_list_box,
            &pins,
            self.visibility_mode == VisibilityMode::Hidden,
            &sender,
        );

        // Send tab
        widgets.send_text_tile.remove_css_class("kn-selection-tile");
        widgets
            .send_text_tile
            .remove_css_class("kn-selection-tile-active");
        widgets
            .send_text_tile
            .add_css_class(if self.text_composer_open {
                "kn-selection-tile-active"
            } else {
                "kn-selection-tile"
            });
        widgets
            .send_text_composer_box
            .set_visible(self.text_composer_open);
        if widgets.send_text_entry.text().as_str() != self.text_draft
            && !widgets.send_text_entry.has_focus()
        {
            widgets.send_text_entry.set_text(&self.text_draft);
        }

        let total_bytes: u64 = self.send_queue.iter().map(|i| i.size_bytes).sum();
        widgets.send_queue_heading.set_label(&format!(
            "{} item(s) ({})",
            self.send_queue.len(),
            format_bytes(total_bytes)
        ));
        widgets
            .send_clear_btn
            .set_visible(!self.send_queue.is_empty());
        widgets
            .send_clear_btn
            .set_sensitive(!self.send_queue.is_empty() && !self.sending_in_flight);

        if let Some(ref err) = self.send_error {
            widgets.send_error_label.set_label(err);
            widgets.send_error_banner.set_visible(true);
        } else {
            widgets.send_error_banner.set_visible(false);
        }
        rebuild_send_queue_rows(&widgets.send_queue_list_box, &self.send_queue, &sender);
        rebuild_send_targets(
            &widgets.send_targets_box,
            &self.paired_devices,
            self.sending_in_flight,
            !self.send_queue.is_empty(),
            &sender,
        );

        // Settings tab
        widgets.save_folder_btn.set_label(&self.incoming_directory);
        sync_switch(
            &widgets.global_auto_accept_switch,
            self.config.auto_accept_from_paired,
        );
        sync_switch(
            &widgets.clipboard_sync_switch,
            self.config.clipboard_sync_enabled,
        );
        sync_switch(
            &widgets.clipboard_images_switch,
            self.config.clipboard_images_enabled,
        );
        sync_switch(
            &widgets.otp_passthrough_switch,
            self.config.otp_passthrough_enabled,
        );
        widgets
            .mime_count_label
            .set_label(&self.config.ignored_mime_types.len().to_string());
        widgets
            .mime_chevron_label
            .set_label(if self.mime_editor_open { "^" } else { "v" });
        widgets.mime_editor_box.set_visible(self.mime_editor_open);
        if widgets.mime_entry.text().as_str() != self.new_mime_input
            && !widgets.mime_entry.has_focus()
        {
            widgets.mime_entry.set_text(&self.new_mime_input);
        }
        rebuild_mime_chips(
            &widgets.mime_chips_box,
            &self.config.ignored_mime_types,
            &sender,
        );
        sync_switch(
            &widgets.audio_handoff_switch,
            self.config.audio_handoff_enabled,
        );
        sync_switch(
            &widgets.notify_auto_accept_switch,
            self.config.notify_auto_accept_complete,
        );
        sync_switch(&widgets.dark_theme_switch, self.dark_theme);

        // Device detail sub-screen
        if let Some(dev) = self.selected_detail_device() {
            let online = !dev.last_lan_address.is_empty();
            widgets
                .detail_icon_box
                .remove_css_class("kn-loading-tonal-badge");
            widgets.detail_icon_box.remove_css_class("keynear-tonal");
            widgets.detail_icon_box.add_css_class(if online {
                "kn-loading-tonal-badge"
            } else {
                "keynear-tonal"
            });
            widgets.detail_name_label.set_label(&dev.display_name);
            let status_txt = format!(
                "{} · {} ({})",
                role_label(dev.device_role),
                if online {
                    "Connected via Wi-Fi"
                } else {
                    "Bluetooth standby"
                },
                dev.paired_at
            );
            widgets.detail_status_label.set_label(&status_txt);

            if widgets.detail_name_entry.widget_name() != dev.device_id {
                widgets.detail_name_entry.set_widget_name(&dev.device_id);
                widgets.detail_name_entry.set_text(&self.rename_draft);
            } else if widgets.detail_name_entry.text().as_str() != self.rename_draft
                && !widgets.detail_name_entry.has_focus()
            {
                widgets.detail_name_entry.set_text(&self.rename_draft);
            }
            sync_switch(&widgets.detail_auto_accept_switch, dev.auto_accept_files);

            let key_prefix: String = dev.static_x25519_pubkey.chars().take(19).collect();
            widgets
                .detail_key_value
                .set_label(&format!("{key_prefix}…"));
            widgets
                .detail_ble_value
                .set_label(if dev.last_ble_address.is_empty() {
                    "Unavailable"
                } else {
                    &dev.last_ble_address
                });
            widgets
                .detail_lan_value
                .set_label(if dev.last_lan_address.is_empty() {
                    "Standby"
                } else {
                    &dev.last_lan_address
                });

            let confirming = self.pending_unpair_id.as_deref() == Some(dev.device_id.as_str());
            widgets.detail_remove_btn.set_visible(!confirming);
            widgets.detail_confirm_box.set_visible(confirming);
            if confirming {
                widgets.detail_confirm_label.set_label(&format!(
                    "Remove {}? You will need to confirm a new 6-digit code on both devices to pair again.",
                    dev.display_name
                ));
            }
        }

        // Snackbar
        if let Some(ref snack) = self.snackbar_message {
            widgets.snackbar_label.set_label(snack);
            widgets.snackbar_box.set_visible(true);
        } else {
            widgets.snackbar_box.set_visible(false);
        }

        // BottomNav pills
        let navs = [
            (
                &widgets.nav_receive_pill,
                &widgets.nav_receive_text,
                self.sub_screen.is_none() && self.active_tab == ActiveTab::Receive,
            ),
            (
                &widgets.nav_send_pill,
                &widgets.nav_send_text,
                self.sub_screen.is_none() && self.active_tab == ActiveTab::Send,
            ),
            (
                &widgets.nav_settings_pill,
                &widgets.nav_settings_text,
                self.sub_screen.is_none() && self.active_tab == ActiveTab::Settings,
            ),
        ];
        for (pill, txt, active) in navs {
            pill.remove_css_class("kn-nav-pill-active");
            pill.remove_css_class("kn-nav-pill-inactive");
            pill.add_css_class(if active {
                "kn-nav-pill-active"
            } else {
                "kn-nav-pill-inactive"
            });
            txt.remove_css_class("keynear-description");
            txt.remove_css_class("keynear-caption");
            txt.add_css_class(if active {
                "keynear-description"
            } else {
                "keynear-caption"
            });
        }
    }
}

fn sync_switch(sw: &gtk4::Switch, desired: bool) {
    if sw.is_active() != desired {
        sw.set_active(desired);
    }
}

fn meta_row(label: &str) -> (gtk4::Box, gtk4::Label) {
    let row = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(12)
        .build();
    let key = gtk4::Label::builder()
        .label(label)
        .xalign(0.0)
        .hexpand(true)
        .build();
    key.add_css_class("keynear-caption");
    let val = gtk4::Label::builder()
        .label("")
        .xalign(1.0)
        .ellipsize(EllipsizeMode::End)
        .build();
    val.add_css_class("keynear-caption");
    row.append(&key);
    row.append(&val);
    (row, val)
}

pub fn run_gui(init: SettingsInit) {
    let app = RelmApp::new("org.keynear.settings").with_args(Vec::new());
    app.run::<SettingsModel>(init);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn orbit_for_matches_prototype_formula() {
        let near = orbit_for(-40, 0, 2);
        assert_eq!(near.radius_px, 128);
        assert_eq!(near.duration_secs, 40);
        assert_eq!(near.phase_secs, 0);
        assert_eq!(near.offset_x, 0);
        assert_eq!(near.offset_y, -128);

        let far = orbit_for(-90, 1, 2);
        assert_eq!(far.radius_px, 176);
        assert_eq!(far.phase_secs, 20);
        assert_eq!(far.offset_x, 0);
        assert_eq!(far.offset_y, 176);
    }

    #[test]
    fn prototype_tabs_send_queue_and_settings_persist() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("keynear-settings-proto-{nonce}"));
        fs::create_dir_all(&dir).expect("mkdir");
        let cfg_path = dir.join("config.toml");
        let cfg = DaemonConfig::default();
        cfg.save_to_path(&cfg_path).expect("save cfg");
        let store = PairedDeviceStore::new(dir.join("devices.toml"));

        let mut model = SettingsModel::from_init(SettingsInit {
            config: cfg,
            config_path: Some(cfg_path.clone()),
            everyone_remaining_secs: None,
            error_message: None,
            warning_message: None,
            transfer_error: None,
            paired_devices: vec![PairedDevice {
                device_id: "phone-1".to_string(),
                display_name: "Pixel".to_string(),
                device_role: DeviceRole::Mobile,
                static_x25519_pubkey: "abcd".repeat(16),
                last_ble_address: "AA:BB:CC:DD:EE:FF".to_string(),
                last_lan_address: "192.168.1.20:4433".to_string(),
                last_endpoint_id: String::new(),
                capability_flags: 1,
                auto_accept_files: false,
                paired_at: "Nov 14, 2026".to_string(),
            }],
            nearby_peers: Vec::new(),
            transfers: Vec::new(),
            send_paths: Vec::new(),
            selected_send_device_id: None,
            send_picker_active: false,
            device_store: store,
            secret_store: None,
            ipc_socket_path: None,
        });

        assert_eq!(model.active_tab(), ActiveTab::Receive);
        model.apply_msg(SettingsMsg::OpenDeviceDetail("phone-1".to_string()));
        assert_eq!(
            model.sub_screen(),
            Some(&SubScreen::DeviceDetail("phone-1".to_string()))
        );
        model.apply_msg(SettingsMsg::NavigateBack);
        assert!(model.sub_screen().is_none());

        model.apply_msg(SettingsMsg::SwitchTab(ActiveTab::Send));
        model.apply_msg(SettingsMsg::StageClipboardForSend);
        model.apply_msg(SettingsMsg::ToggleTextComposer);
        model.apply_msg(SettingsMsg::UpdateTextDraft(
            "https://keynear.example/spec".to_string(),
        ));
        model.apply_msg(SettingsMsg::StageTextSnippet);
        assert_eq!(model.send_queue().len(), 2);

        let first_id = model.send_queue()[0].id.clone();
        model.apply_msg(SettingsMsg::RemoveSendQueueItem(first_id));
        assert_eq!(model.send_queue().len(), 1);
        model.apply_msg(SettingsMsg::ClearSendPaths);
        assert!(model.send_queue().is_empty());

        model.apply_msg(SettingsMsg::SwitchTab(ActiveTab::Settings));
        model.apply_msg(SettingsMsg::ToggleClipboardSync(false));
        model.apply_msg(SettingsMsg::ToggleClipboardImages(false));
        model.apply_msg(SettingsMsg::ToggleOtpPassthrough(false));
        model.apply_msg(SettingsMsg::ToggleAudioHandoff(false));
        model.apply_msg(SettingsMsg::ToggleNotifyAutoAccept(false));
        model.apply_msg(SettingsMsg::UpdateMimeInput(
            "application/x-secret".to_string(),
        ));
        model.apply_msg(SettingsMsg::AddIgnoredMime);
        assert!(model
            .ignored_mime_types()
            .contains(&"application/x-secret".to_string()));
        model.apply_msg(SettingsMsg::RemoveIgnoredMime(
            "application/x-secret".to_string(),
        ));
        assert!(!model
            .ignored_mime_types()
            .contains(&"application/x-secret".to_string()));
        model.apply_msg(SettingsMsg::ToggleDarkTheme(true));
        assert!(model.dark_theme());

        let reloaded = DaemonConfig::load_or_default(&cfg_path).expect("reload");
        assert!(!reloaded.clipboard_sync_enabled);
        assert!(!reloaded.clipboard_images_enabled);
        assert!(!reloaded.otp_passthrough_enabled);
        assert!(!reloaded.audio_handoff_enabled);
        assert!(!reloaded.notify_auto_accept_complete);

        let _ = fs::remove_dir_all(&dir);
    }
}
