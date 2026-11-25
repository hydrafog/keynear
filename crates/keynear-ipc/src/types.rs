use crate::*;

#[derive(Debug, Clone, PartialEq)]
pub enum IpcRequest {
    SendFiles {
        device_id: String,
        paths: Vec<PathBuf>,
    },
    AcceptTransfer {
        transfer_id: String,
    },
    DeclineTransfer {
        transfer_id: String,
    },
    CancelTransfer {
        transfer_id: String,
    },
    ListTransfers,
    Status,
    ListDevices,
    SetVisibility {
        mode: VisibilityMode,
    },
    GetConfig,
    UpdateConfig {
        config: DaemonConfig,
    },
    RenameDevice {
        device_id: String,
        display_name: String,
    },
    SetDeviceAutoAccept {
        device_id: String,
        enabled: bool,
    },
    UnpairDevice {
        device_id: String,
    },
    RespondPairing {
        approve: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonStatus {
    pub device_id: String,
    pub endpoint_id: String,
    pub listen_addrs: Vec<String>,
    pub visibility_mode: VisibilityMode,
    pub mdns_degraded: bool,
    pub active_transfer_count: u32,
    pub paired_device_count: u32,
    pub everyone_remaining_secs: Option<u64>,
    pub auto_accept_from_paired: bool,
    pub incoming_directory: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum IpcResponse {
    Transfer(TransferSession),
    Transfers(Vec<TransferSession>),
    Status(DaemonStatus),
    Devices(Vec<PairedDevice>),
    Device(PairedDevice),
    Config(DaemonConfig),
    Unpaired { device_id: String },
    PairingDecision { approved: bool },
    Error { message: String },
}
