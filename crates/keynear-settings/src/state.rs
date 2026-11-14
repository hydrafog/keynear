use std::path::PathBuf;
use std::sync::Arc;

use gtk4::prelude::*;
use keynear_core::{
    DaemonConfig, DeviceRole, NearbyPeer, PairedDevice, PairedDeviceStore, SecretStore,
    TransferSession, VisibilityMode, EVERYONE_NEARBY_TIMEOUT_SECS, PAIRING_TIMEOUT_SECS,
};

use crate::ipc::IpcRequest;
use crate::portal::{
    confirm_unpair_device, item_from_path, persist_config, refresh_from_daemon,
    send_transfer_action, set_device_auto_accept, submit_device_rename, sync_visibility,
};

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
pub(crate) struct MapPin {
    pub(crate) id: String,
    pub(crate) display_name: String,
    pub(crate) state_label: String,
    pub(crate) rssi: i16,
    pub(crate) is_paired: bool,
    pub(crate) is_online: bool,
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
    pub(crate) config: DaemonConfig,
    pub(crate) config_path: Option<PathBuf>,
    pub(crate) visibility_mode: VisibilityMode,
    pub(crate) everyone_remaining_secs: u64,
    pub(crate) incoming_directory: String,
    pub(crate) error_message: Option<String>,
    pub(crate) warning_message: Option<String>,
    pub(crate) transfer_error: Option<String>,
    pub(crate) send_error: Option<String>,
    pub(crate) nearby_peers: Vec<NearbyPeer>,
    pub(crate) paired_devices: Vec<PairedDevice>,
    pub(crate) transfers: Vec<TransferSession>,
    pub(crate) send_paths: Vec<PathBuf>,
    pub(crate) send_queue: Vec<SendQueueItem>,
    pub(crate) next_queue_seq: u64,
    pub(crate) selected_send_device_id: Option<String>,
    pub(crate) sending_in_flight: bool,
    pub(crate) active_tab: ActiveTab,
    pub(crate) sub_screen: Option<SubScreen>,
    pub(crate) sheet_expanded: bool,
    pub(crate) text_composer_open: bool,
    pub(crate) text_draft: String,
    pub(crate) mime_editor_open: bool,
    pub(crate) new_mime_input: String,
    pub(crate) dark_theme: bool,
    pub(crate) snackbar_message: Option<String>,
    pub(crate) snackbar_ttl_secs: u64,
    pub(crate) renaming_device_id: Option<String>,
    pub(crate) rename_draft: String,
    pub(crate) pending_pairing: Option<PendingPairingView>,
    pub(crate) pending_unpair_id: Option<String>,
    pub(crate) device_store: PairedDeviceStore,
    pub(crate) secret_store: Option<Arc<dyn SecretStore>>,
    pub(crate) ipc_socket_path: Option<PathBuf>,
    pub(crate) root_window: Option<gtk4::Window>,
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

pub(crate) fn role_label(role: DeviceRole) -> &'static str {
    match role {
        DeviceRole::Desktop => "Desktop",
        DeviceRole::Mobile => "Mobile",
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

    pub(crate) fn show_snackbar(&mut self, message: impl Into<String>) {
        self.snackbar_message = Some(message.into());
        self.snackbar_ttl_secs = 3;
    }

    pub(crate) fn map_pins(&self) -> Vec<MapPin> {
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
        if self.visibility_mode != VisibilityMode::Hidden {
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

    pub(crate) fn selected_detail_device(&self) -> Option<&PairedDevice> {
        match self.sub_screen {
            Some(SubScreen::DeviceDetail(ref device_id)) => self
                .paired_devices
                .iter()
                .find(|d| &d.device_id == device_id),
            _ => None,
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
        sync_visibility(self, mode);
        let snack = match self.visibility_mode {
            VisibilityMode::Hidden => "Visibility set to hidden.",
            VisibilityMode::PairedOnly => "Listening for paired computers.",
            VisibilityMode::EveryoneNearby => "Visible to everyone nearby for 10 minutes.",
        };
        self.show_snackbar(snack);
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

    pub fn apply_msg(&mut self, msg: SettingsMsg) {
        match msg {
            SettingsMsg::SetVisibility(mode) => self.apply_visibility_mode(mode),
            SettingsMsg::Tick => self.handle_tick(),
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
            SettingsMsg::ToggleSheetExpanded => self.sheet_expanded = !self.sheet_expanded,
            SettingsMsg::RefreshPeers => {
                match self.device_store.load() {
                    Ok(devices) => {
                        self.paired_devices = devices;
                        self.error_message = None;
                    }
                    Err(err) => self.error_message = Some(err.to_string()),
                }
                refresh_from_daemon(self, false);
            }
            SettingsMsg::RetryDaemon => {
                if let Ok(devices) = self.device_store.load() {
                    self.paired_devices = devices;
                }
                refresh_from_daemon(self, true);
            }
            SettingsMsg::BeginPair(peer_token) => self.begin_pair(&peer_token),
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
            SettingsMsg::RequestUnpair(device_id) => self.pending_unpair_id = Some(device_id),
            SettingsMsg::CancelUnpair => self.pending_unpair_id = None,
            SettingsMsg::ConfirmUnpair(device_id) => confirm_unpair_device(self, &device_id),
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
            SettingsMsg::UpdateRenameDraft(text) => self.rename_draft = text,
            SettingsMsg::SubmitRename {
                device_id,
                display_name,
            } => submit_device_rename(self, device_id, display_name),
            SettingsMsg::SetAutoAcceptFiles { device_id, enabled } => {
                set_device_auto_accept(self, device_id, enabled);
            }
            SettingsMsg::SetGlobalAutoAccept(enabled) => {
                if self.config.auto_accept_from_paired != enabled {
                    self.config.auto_accept_from_paired = enabled;
                    self.error_message = None;
                    persist_config(self);
                }
            }
            SettingsMsg::ToggleClipboardSync(enabled) => {
                if self.config.clipboard_sync_enabled != enabled {
                    self.config.clipboard_sync_enabled = enabled;
                    self.error_message = None;
                    persist_config(self);
                }
            }
            SettingsMsg::ToggleClipboardImages(enabled) => {
                if self.config.clipboard_images_enabled != enabled {
                    self.config.clipboard_images_enabled = enabled;
                    self.error_message = None;
                    persist_config(self);
                }
            }
            SettingsMsg::ToggleOtpPassthrough(enabled) => {
                if self.config.otp_passthrough_enabled != enabled {
                    self.config.otp_passthrough_enabled = enabled;
                    self.error_message = None;
                    persist_config(self);
                }
            }
            SettingsMsg::ToggleAudioHandoff(enabled) => {
                if self.config.audio_handoff_enabled != enabled {
                    self.config.audio_handoff_enabled = enabled;
                    self.error_message = None;
                    persist_config(self);
                }
            }
            SettingsMsg::ToggleNotifyAutoAccept(enabled) => {
                if self.config.notify_auto_accept_complete != enabled {
                    self.config.notify_auto_accept_complete = enabled;
                    self.error_message = None;
                    persist_config(self);
                }
            }
            SettingsMsg::ToggleMimeExpanded => self.mime_editor_open = !self.mime_editor_open,
            SettingsMsg::UpdateMimeInput(raw) => self.new_mime_input = raw,
            SettingsMsg::AddIgnoredMime => {
                let trimmed = self.new_mime_input.trim().to_string();
                if !trimmed.is_empty() && !self.config.ignored_mime_types.contains(&trimmed) {
                    self.config.ignored_mime_types.push(trimmed);
                    self.new_mime_input.clear();
                    self.error_message = None;
                    persist_config(self);
                }
            }
            SettingsMsg::RemoveIgnoredMime(mime) => {
                let before = self.config.ignored_mime_types.len();
                self.config.ignored_mime_types.retain(|m| m != &mime);
                if self.config.ignored_mime_types.len() != before {
                    self.error_message = None;
                    persist_config(self);
                }
            }
            SettingsMsg::ToggleDarkTheme(enabled) => self.toggle_dark_theme(enabled),
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
            SettingsMsg::ToggleTextComposer => self.text_composer_open = !self.text_composer_open,
            SettingsMsg::UpdateTextDraft(text) => self.text_draft = text,
            SettingsMsg::StageTextSnippet => {
                let draft = self.text_draft.clone();
                if !draft.trim().is_empty() {
                    self.stage_text_item(&draft, false);
                    self.text_draft.clear();
                    self.text_composer_open = false;
                }
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
                    Err(err) => self.send_error = Some(err),
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
                persist_config(self);
                self.show_snackbar(format!("Save folder set to {clean}."));
            }
            SettingsMsg::PortalError(err) => self.error_message = Some(err),
            SettingsMsg::PickSendFiles
            | SettingsMsg::PickSendFolder
            | SettingsMsg::PickIncomingDirectory
            | SettingsMsg::SendQueueToPeer(_) => {}
            SettingsMsg::AcceptTransfer(transfer_id) => {
                send_transfer_action(self, IpcRequest::AcceptTransfer { transfer_id });
            }
            SettingsMsg::DeclineTransfer(transfer_id) => {
                send_transfer_action(self, IpcRequest::DeclineTransfer { transfer_id });
            }
            SettingsMsg::CancelTransfer(transfer_id) => {
                send_transfer_action(self, IpcRequest::CancelTransfer { transfer_id });
            }
        }
    }

    fn handle_tick(&mut self) {
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
        refresh_from_daemon(self, false);
    }

    fn begin_pair(&mut self, peer_token: &str) {
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

    fn toggle_dark_theme(&mut self, enabled: bool) {
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
}
