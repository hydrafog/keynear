pub mod devices;
pub mod store;
pub mod transfers;

use std::path::PathBuf;

use keynear_protocol::{
    DaemonConfig, DeviceRole, NearbyPeer, PairedDevice, PairedDeviceStore, TransferSession,
    VisibilityMode,
};

pub use devices::{orbit_for, MapPin, OrbitPlacement, PendingPairingView};
pub use transfers::{SendItemKind, SendQueueItem};

use crate::portal::{
    confirm_unpair_device, persist_config, refresh_from_daemon, set_device_auto_accept,
    submit_device_rename,
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

impl SettingsModel {
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
                self.set_config_flag(enabled, |c| &mut c.auto_accept_from_paired)
            }
            SettingsMsg::ToggleClipboardSync(enabled) => {
                self.set_config_flag(enabled, |c| &mut c.clipboard_sync_enabled)
            }
            SettingsMsg::ToggleClipboardImages(enabled) => {
                self.set_config_flag(enabled, |c| &mut c.clipboard_images_enabled)
            }
            SettingsMsg::ToggleOtpPassthrough(enabled) => {
                self.set_config_flag(enabled, |c| &mut c.otp_passthrough_enabled)
            }
            SettingsMsg::ToggleAudioHandoff(enabled) => {
                self.set_config_flag(enabled, |c| &mut c.audio_handoff_enabled)
            }
            SettingsMsg::ToggleNotifyAutoAccept(enabled) => {
                self.set_config_flag(enabled, |c| &mut c.notify_auto_accept_complete)
            }
            SettingsMsg::ToggleMimeExpanded => self.mime_editor_open = !self.mime_editor_open,
            SettingsMsg::UpdateMimeInput(raw) => self.new_mime_input = raw,
            SettingsMsg::AddIgnoredMime => {
                const MAX_IPC_MIMES: usize = 128;
                let trimmed = self.new_mime_input.trim().to_string();
                if !trimmed.is_empty() && !self.config.ignored_mime_types.contains(&trimmed) {
                    if self.config.ignored_mime_types.len() >= MAX_IPC_MIMES {
                        self.error_message = Some(format!(
                            "Ignored MIME list cannot exceed {MAX_IPC_MIMES} entries."
                        ));
                        return;
                    }
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
            SettingsMsg::StageSendPaths(paths) => self.handle_stage_send_paths(paths),
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
            SettingsMsg::RemoveSendQueueItem(id) => self.handle_remove_queue_item(&id),
            SettingsMsg::ClearSendPaths => {
                self.send_paths.clear();
                self.send_queue.clear();
                self.send_error = None;
            }
            SettingsMsg::SelectSendDevice(device_id) => {
                self.selected_send_device_id = Some(device_id);
                self.send_error = None;
            }
            SettingsMsg::SendCompleted(result) => self.handle_send_completed(result),
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
            SettingsMsg::AcceptTransfer(transfer_id) => self.handle_accept_transfer(transfer_id),
            SettingsMsg::DeclineTransfer(transfer_id) => self.handle_decline_transfer(transfer_id),
            SettingsMsg::CancelTransfer(transfer_id) => self.handle_cancel_transfer(transfer_id),
        }
    }
}
