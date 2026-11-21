mod messages;

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
        messages::apply_msg(self, msg);
    }
}
