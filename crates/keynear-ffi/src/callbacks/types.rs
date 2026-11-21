use crate::callbacks::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum FfiVisibilityMode {
    Hidden,
    PairedOnly,
    EveryoneNearby,
}

impl From<VisibilityMode> for FfiVisibilityMode {
    fn from(value: VisibilityMode) -> Self {
        match value {
            VisibilityMode::Hidden => Self::Hidden,
            VisibilityMode::PairedOnly => Self::PairedOnly,
            VisibilityMode::EveryoneNearby => Self::EveryoneNearby,
        }
    }
}

impl From<FfiVisibilityMode> for VisibilityMode {
    fn from(value: FfiVisibilityMode) -> Self {
        match value {
            FfiVisibilityMode::Hidden => Self::Hidden,
            FfiVisibilityMode::PairedOnly => Self::PairedOnly,
            FfiVisibilityMode::EveryoneNearby => Self::EveryoneNearby,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum FfiDeviceRole {
    Desktop,
    Mobile,
}

impl From<DeviceRole> for FfiDeviceRole {
    fn from(value: DeviceRole) -> Self {
        match value {
            DeviceRole::Desktop => Self::Desktop,
            DeviceRole::Mobile => Self::Mobile,
        }
    }
}

impl From<FfiDeviceRole> for DeviceRole {
    fn from(value: FfiDeviceRole) -> Self {
        match value {
            FfiDeviceRole::Desktop => Self::Desktop,
            FfiDeviceRole::Mobile => Self::Mobile,
        }
    }
}

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum FfiError {
    #[error("configuration error: {detail}")]
    Config { detail: String },
    #[error("validation error on {field}: {detail}")]
    Validation { field: String, detail: String },
    #[error("control frame size {size} bytes exceeds maximum {max} bytes")]
    FrameTooLarge { size: u64, max: u64 },
    #[error("protocol error: {detail}")]
    Protocol { detail: String },
    #[error("cryptographic error: {detail}")]
    Crypto { detail: String },
    #[error("pairing verification timed out after 120 seconds")]
    PairingExpired,
    #[error("pairing rejected")]
    PairingRejected,
    #[error("keyring error: {detail}")]
    Keyring { detail: String },
    #[error("bluetooth error: {detail}")]
    Bluetooth { detail: String },
    #[error("transport error: {detail}")]
    Transport { detail: String },
    #[error("transfer error: {detail}")]
    Transfer { detail: String },
    #[error("transfer cancelled: {detail}")]
    TransferCancelled { detail: String },
    #[error("unauthorized peer: {detail}")]
    UnauthorizedPeer { detail: String },
    #[error("i/o error: {detail}")]
    Io { detail: String },
}

impl From<KeynearError> for FfiError {
    fn from(err: KeynearError) -> Self {
        match err {
            KeynearError::Config(detail) => Self::Config { detail },
            KeynearError::Validation { field, message } => Self::Validation {
                field: field.to_string(),
                detail: message,
            },
            KeynearError::FrameTooLarge { size, max } => Self::FrameTooLarge {
                size: size as u64,
                max: max as u64,
            },
            KeynearError::Protocol(detail) => Self::Protocol { detail },
            KeynearError::Crypto(detail) => Self::Crypto { detail },
            KeynearError::PairingExpired => Self::PairingExpired,
            KeynearError::PairingRejected => Self::PairingRejected,
            KeynearError::Keyring(detail) => Self::Keyring { detail },
            KeynearError::Bluetooth(detail) => Self::Bluetooth { detail },
            KeynearError::Transport(detail) => Self::Transport { detail },
            KeynearError::Transfer(detail) => Self::Transfer { detail },
            KeynearError::TransferCancelled(detail) => Self::TransferCancelled { detail },
            KeynearError::UnauthorizedPeer(detail) => Self::UnauthorizedPeer { detail },
            KeynearError::Io(io_err) => Self::Io {
                detail: io_err.to_string(),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct CoreStatus {
    pub version: String,
    pub max_control_frame_bytes: u32,
    pub ready: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct FfiDaemonConfig {
    pub visibility_mode: FfiVisibilityMode,
    pub incoming_directory: String,
    pub auto_accept_from_paired: bool,
    pub clipboard_sync_enabled: bool,
    pub clipboard_images_enabled: bool,
    pub otp_passthrough_enabled: bool,
    pub audio_handoff_enabled: bool,
    pub ignored_mime_types: Vec<String>,
    pub notify_auto_accept_complete: bool,
}

impl From<DaemonConfig> for FfiDaemonConfig {
    fn from(value: DaemonConfig) -> Self {
        Self {
            visibility_mode: FfiVisibilityMode::from(value.visibility_mode),
            incoming_directory: value.incoming_directory,
            auto_accept_from_paired: value.auto_accept_from_paired,
            clipboard_sync_enabled: value.clipboard_sync_enabled,
            clipboard_images_enabled: value.clipboard_images_enabled,
            otp_passthrough_enabled: value.otp_passthrough_enabled,
            audio_handoff_enabled: value.audio_handoff_enabled,
            ignored_mime_types: value.ignored_mime_types,
            notify_auto_accept_complete: value.notify_auto_accept_complete,
        }
    }
}

impl From<FfiDaemonConfig> for DaemonConfig {
    fn from(value: FfiDaemonConfig) -> Self {
        Self {
            visibility_mode: VisibilityMode::from(value.visibility_mode),
            incoming_directory: value.incoming_directory,
            auto_accept_from_paired: value.auto_accept_from_paired,
            clipboard_sync_enabled: value.clipboard_sync_enabled,
            clipboard_images_enabled: value.clipboard_images_enabled,
            otp_passthrough_enabled: value.otp_passthrough_enabled,
            audio_handoff_enabled: value.audio_handoff_enabled,
            ignored_mime_types: value
                .ignored_mime_types
                .into_iter()
                .map(|m| m.trim().to_string())
                .filter(|m| !m.is_empty())
                .collect(),
            notify_auto_accept_complete: value.notify_auto_accept_complete,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct FfiPairedDevice {
    pub device_id: String,
    pub display_name: String,
    pub device_role: FfiDeviceRole,
    pub static_x25519_pubkey: String,
    pub last_ble_address: String,
    pub last_lan_address: String,
    pub last_endpoint_id: String,
    pub capability_flags: u32,
    pub auto_accept_files: bool,
    pub paired_at: String,
}

impl From<PairedDevice> for FfiPairedDevice {
    fn from(value: PairedDevice) -> Self {
        Self {
            device_id: value.device_id,
            display_name: value.display_name,
            device_role: FfiDeviceRole::from(value.device_role),
            static_x25519_pubkey: value.static_x25519_pubkey,
            last_ble_address: value.last_ble_address,
            last_lan_address: value.last_lan_address,
            last_endpoint_id: value.last_endpoint_id,
            capability_flags: value.capability_flags,
            auto_accept_files: value.auto_accept_files,
            paired_at: value.paired_at,
        }
    }
}

impl From<FfiPairedDevice> for PairedDevice {
    fn from(value: FfiPairedDevice) -> Self {
        Self {
            device_id: value.device_id,
            display_name: value.display_name,
            device_role: DeviceRole::from(value.device_role),
            static_x25519_pubkey: value.static_x25519_pubkey,
            last_ble_address: value.last_ble_address,
            last_lan_address: value.last_lan_address,
            last_endpoint_id: value.last_endpoint_id,
            capability_flags: value.capability_flags,
            auto_accept_files: value.auto_accept_files,
            paired_at: value.paired_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct FfiLanEndpoint {
    pub device_id: String,
    pub endpoint_id: Vec<u8>,
    pub socket_addrs: Vec<String>,
    pub capability_flags: u32,
    pub issued_at_ms: i64,
}

impl From<v1::LanEndpoint> for FfiLanEndpoint {
    fn from(value: v1::LanEndpoint) -> Self {
        Self {
            device_id: value.device_id,
            endpoint_id: value.endpoint_id,
            socket_addrs: value.socket_addrs,
            capability_flags: value.capability_flags,
            issued_at_ms: value.issued_at_ms,
        }
    }
}

impl From<FfiLanEndpoint> for v1::LanEndpoint {
    fn from(value: FfiLanEndpoint) -> Self {
        Self {
            device_id: value.device_id,
            endpoint_id: value.endpoint_id,
            socket_addrs: value.socket_addrs,
            capability_flags: value.capability_flags,
            issued_at_ms: value.issued_at_ms,
        }
    }
}
