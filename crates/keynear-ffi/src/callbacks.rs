use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use keynear_core::{
    StaticKeypairRecord, KEYNEAR_BLE_SERVICE_UUID, KEYNEAR_BLE_SERVICE_UUID16,
    LAN_ENDPOINT_CHAR_UUID, LAN_ENDPOINT_CHAR_UUID16,
};
use keynear_protocol::{
    control_frame_to_envelope, decode_control_frame, v1, DaemonConfig, DeviceRole, KeynearError,
    PairedDevice, PairedDeviceStore, VisibilityMode, EVERYONE_NEARBY_TIMEOUT_SECS,
    MAX_CONTROL_FRAME_BYTES, PAIRING_TIMEOUT_SECS,
};

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

pub(crate) fn to_key_32(bytes: &[u8], field: &'static str) -> Result<[u8; 32], KeynearError> {
    if bytes.len() != 32 {
        return Err(KeynearError::Validation {
            field,
            message: format!("expected 32 bytes, got {}", bytes.len()),
        });
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(bytes);
    Ok(out)
}

#[derive(Debug, Default)]
pub(crate) struct VisibilityStateInner {
    pub(crate) mode: VisibilityMode,
    deadline: Option<Instant>,
}

impl VisibilityStateInner {
    pub(crate) fn refresh(&mut self, now: Instant) {
        if self.mode == VisibilityMode::EveryoneNearby {
            if let Some(deadline) = self.deadline {
                if now >= deadline {
                    self.mode = VisibilityMode::PairedOnly;
                    self.deadline = None;
                }
            }
        }
    }

    pub(crate) fn set_mode(&mut self, mode: VisibilityMode, now: Instant) -> VisibilityMode {
        self.mode = mode;
        self.deadline = if mode == VisibilityMode::EveryoneNearby {
            Some(now + Duration::from_secs(EVERYONE_NEARBY_TIMEOUT_SECS))
        } else {
            None
        };
        mode
    }

    pub(crate) fn remaining_secs(&mut self, now: Instant) -> u64 {
        self.refresh(now);
        if self.mode != VisibilityMode::EveryoneNearby {
            return 0;
        }
        self.deadline
            .and_then(|deadline| deadline.checked_duration_since(now))
            .map(|duration| duration.as_secs())
            .unwrap_or(0)
    }
}

pub(crate) fn visibility_mode_impl(lock: &Mutex<VisibilityStateInner>) -> FfiVisibilityMode {
    let mut guard = lock.lock().unwrap_or_else(|p| p.into_inner());
    guard.refresh(Instant::now());
    FfiVisibilityMode::from(guard.mode)
}

pub(crate) fn set_visibility_mode_impl(
    lock: &Mutex<VisibilityStateInner>,
    mode: FfiVisibilityMode,
) -> FfiVisibilityMode {
    let mut guard = lock.lock().unwrap_or_else(|p| p.into_inner());
    let updated = guard.set_mode(VisibilityMode::from(mode), Instant::now());
    FfiVisibilityMode::from(updated)
}

pub(crate) fn cycle_visibility_mode_impl(lock: &Mutex<VisibilityStateInner>) -> FfiVisibilityMode {
    let mut guard = lock.lock().unwrap_or_else(|p| p.into_inner());
    let now = Instant::now();
    guard.refresh(now);
    let next = match guard.mode {
        VisibilityMode::Hidden => VisibilityMode::PairedOnly,
        VisibilityMode::PairedOnly => VisibilityMode::EveryoneNearby,
        VisibilityMode::EveryoneNearby => VisibilityMode::Hidden,
    };
    guard.set_mode(next, now);
    FfiVisibilityMode::from(next)
}

pub(crate) fn remaining_secs_impl(lock: &Mutex<VisibilityStateInner>) -> u64 {
    let mut guard = lock.lock().unwrap_or_else(|p| p.into_inner());
    guard.remaining_secs(Instant::now())
}

pub(crate) fn validate_config_toml_impl(raw_toml: String) -> Result<FfiVisibilityMode, FfiError> {
    let parsed: DaemonConfig =
        toml::from_str(&raw_toml).map_err(|err| KeynearError::Config(err.to_string()))?;
    parsed.validate()?;
    Ok(FfiVisibilityMode::from(parsed.visibility_mode))
}

pub(crate) fn load_daemon_config_impl(
    lock: &Mutex<VisibilityStateInner>,
    config_toml_path: String,
) -> Result<FfiDaemonConfig, FfiError> {
    let cfg = DaemonConfig::load_or_default(&PathBuf::from(config_toml_path))?;
    let ffi_cfg = FfiDaemonConfig::from(cfg);
    set_visibility_mode_impl(lock, ffi_cfg.visibility_mode);
    Ok(ffi_cfg)
}

pub(crate) fn save_daemon_config_impl(
    lock: &Mutex<VisibilityStateInner>,
    config_toml_path: String,
    config: FfiDaemonConfig,
) -> Result<FfiDaemonConfig, FfiError> {
    let daemon_cfg = DaemonConfig::from(config);
    daemon_cfg.save_to_path(&PathBuf::from(config_toml_path))?;
    let saved = FfiDaemonConfig::from(daemon_cfg);
    set_visibility_mode_impl(lock, saved.visibility_mode);
    Ok(saved)
}

pub(crate) fn validate_control_frame_impl(bytes: Vec<u8>) -> Result<String, FfiError> {
    let frame = decode_control_frame(&bytes)?;
    control_frame_to_envelope(&frame)?;
    Ok(frame.kind)
}

pub(crate) fn generate_static_keypair_impl() -> (Vec<u8>, String) {
    let kp = StaticKeypairRecord::generate();
    (kp.private_key.to_vec(), hex::encode(kp.public_key))
}

pub(crate) fn load_paired_devices_impl(
    devices_toml_path: String,
) -> Result<Vec<FfiPairedDevice>, FfiError> {
    let store = PairedDeviceStore::new(PathBuf::from(devices_toml_path));
    let devices = store.load()?;
    Ok(devices.into_iter().map(FfiPairedDevice::from).collect())
}

pub(crate) fn save_paired_device_impl(
    devices_toml_path: String,
    device: FfiPairedDevice,
) -> Result<(), FfiError> {
    let store = PairedDeviceStore::new(PathBuf::from(devices_toml_path));
    store.upsert(PairedDevice::from(device))?;
    Ok(())
}

pub(crate) fn remove_paired_device_impl(
    devices_toml_path: String,
    device_id: String,
) -> Result<bool, FfiError> {
    let store = PairedDeviceStore::new(PathBuf::from(devices_toml_path));
    store.remove(&device_id).map_err(FfiError::from)
}

pub(crate) fn set_auto_accept_files_impl(
    devices_toml_path: String,
    device_id: String,
    enabled: bool,
) -> Result<FfiPairedDevice, FfiError> {
    let store = PairedDeviceStore::new(PathBuf::from(devices_toml_path));
    let updated = store.set_auto_accept_files(&device_id, enabled)?;
    Ok(FfiPairedDevice::from(updated))
}

pub(crate) fn rename_paired_device_impl(
    devices_toml_path: String,
    device_id: String,
    new_display_name: String,
) -> Result<FfiPairedDevice, FfiError> {
    let store = PairedDeviceStore::new(PathBuf::from(devices_toml_path));
    let updated = store.rename(&device_id, &new_display_name)?;
    Ok(FfiPairedDevice::from(updated))
}

#[uniffi::export]
pub fn get_core_status() -> CoreStatus {
    CoreStatus {
        version: env!("CARGO_PKG_VERSION").to_string(),
        max_control_frame_bytes: MAX_CONTROL_FRAME_BYTES as u32,
        ready: true,
    }
}

#[uniffi::export]
pub fn default_config_summary() -> String {
    let config = DaemonConfig::default();
    format!(
        "visibility={:?},incoming_dir={}",
        config.visibility_mode, config.incoming_directory
    )
}

#[uniffi::export]
pub fn ble_service_uuid_string() -> String {
    KEYNEAR_BLE_SERVICE_UUID.to_string()
}

#[uniffi::export]
pub fn ble_service_uuid16() -> u16 {
    KEYNEAR_BLE_SERVICE_UUID16
}

#[uniffi::export]
pub fn lan_endpoint_char_uuid_string() -> String {
    LAN_ENDPOINT_CHAR_UUID.to_string()
}

#[uniffi::export]
pub fn lan_endpoint_char_uuid16() -> u16 {
    LAN_ENDPOINT_CHAR_UUID16
}

#[uniffi::export]
pub fn everyone_nearby_timeout_secs() -> u64 {
    EVERYONE_NEARBY_TIMEOUT_SECS
}

#[uniffi::export]
pub fn pairing_timeout_secs() -> u64 {
    PAIRING_TIMEOUT_SECS
}
