use std::fmt;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::error::KeynearError;
use crate::keyring::StaticKeypairRecord;
use crate::model::{
    DaemonConfig, DeviceRole, PairedDevice, PairedDeviceStore, VisibilityMode,
    EVERYONE_NEARBY_TIMEOUT_SECS, PAIRING_TIMEOUT_SECS,
};
use crate::pairing::{
    NoisePairingSession, PairingCompleted, PairingPeerMetadata, PairingRole, PairingState,
};
use crate::presence::{
    derive_presence_token, EveryoneNearbyBeacon, PairedBeacon, PairedBeaconState,
    KEYNEAR_BLE_SERVICE_UUID, KEYNEAR_BLE_SERVICE_UUID16,
};
use crate::proto::{control_frame_to_envelope, decode_control_frame, MAX_CONTROL_FRAME_BYTES};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum FfiPairingRole {
    Initiator,
    Responder,
}

impl From<PairingRole> for FfiPairingRole {
    fn from(value: PairingRole) -> Self {
        match value {
            PairingRole::Initiator => Self::Initiator,
            PairingRole::Responder => Self::Responder,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum FfiPairingState {
    HandshakeM1,
    HandshakeM2,
    HandshakeM3,
    AwaitingConfirmation,
    Approved,
    Expired,
    Rejected,
    Completed,
}

impl From<PairingState> for FfiPairingState {
    fn from(value: PairingState) -> Self {
        match value {
            PairingState::HandshakeM1 => Self::HandshakeM1,
            PairingState::HandshakeM2 => Self::HandshakeM2,
            PairingState::HandshakeM3 => Self::HandshakeM3,
            PairingState::AwaitingConfirmation => Self::AwaitingConfirmation,
            PairingState::Approved => Self::Approved,
            PairingState::Expired => Self::Expired,
            PairingState::Rejected => Self::Rejected,
            PairingState::Completed => Self::Completed,
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
pub struct FfiPairedDevice {
    pub device_id: String,
    pub display_name: String,
    pub device_role: FfiDeviceRole,
    pub static_x25519_pubkey: String,
    pub last_ble_address: String,
    pub last_lan_address: String,
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
            capability_flags: value.capability_flags,
            auto_accept_files: value.auto_accept_files,
            paired_at: value.paired_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct FfiPairedBeaconState {
    pub counter: u32,
    pub available: bool,
    pub battery_level: Option<u8>,
}

impl From<PairedBeaconState> for FfiPairedBeaconState {
    fn from(value: PairedBeaconState) -> Self {
        Self {
            counter: value.counter,
            available: value.available,
            battery_level: value.battery_level,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct FfiEveryoneNearbyBeacon {
    pub role: FfiDeviceRole,
    pub capability_flags: u32,
    pub display_name: String,
}

impl From<EveryoneNearbyBeacon> for FfiEveryoneNearbyBeacon {
    fn from(value: EveryoneNearbyBeacon) -> Self {
        Self {
            role: FfiDeviceRole::from(value.role),
            capability_flags: value.capability_flags,
            display_name: value.display_name,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct FfiPairingStep {
    pub outbound_frame: Option<Vec<u8>>,
    pub state: FfiPairingState,
    pub verification_code: Option<String>,
    pub remaining_secs: u64,
}

#[derive(Clone, PartialEq, Eq, uniffi::Record)]
pub struct FfiStaticKeypair {
    pub private_key: Vec<u8>,
    pub public_key_hex: String,
}

impl fmt::Debug for FfiStaticKeypair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FfiStaticKeypair")
            .field("private_key", &"[REDACTED]")
            .field("public_key_hex", &self.public_key_hex)
            .finish()
    }
}

#[derive(Clone, PartialEq, Eq, uniffi::Record)]
pub struct FfiPairingCompleted {
    pub peer_device_id: String,
    pub peer_display_name: String,
    pub peer_role: FfiDeviceRole,
    pub peer_capability_flags: u32,
    pub peer_static_pubkey_hex: String,
    pub reconnect_secret: Vec<u8>,
}

impl fmt::Debug for FfiPairingCompleted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FfiPairingCompleted")
            .field("peer_device_id", &self.peer_device_id)
            .field("peer_display_name", &self.peer_display_name)
            .field("peer_role", &self.peer_role)
            .field("peer_capability_flags", &self.peer_capability_flags)
            .field("peer_static_pubkey_hex", &self.peer_static_pubkey_hex)
            .field("reconnect_secret", &"[REDACTED]")
            .finish()
    }
}

impl From<PairingCompleted> for FfiPairingCompleted {
    fn from(value: PairingCompleted) -> Self {
        Self {
            peer_device_id: value.peer_metadata.device_id.clone(),
            peer_display_name: value.peer_metadata.display_name.clone(),
            peer_role: FfiDeviceRole::from(value.peer_metadata.device_role),
            peer_capability_flags: value.peer_metadata.capability_flags,
            peer_static_pubkey_hex: hex::encode(value.peer_static_pubkey),
            reconnect_secret: value.reconnect_secret.to_vec(),
        }
    }
}

fn to_key_32(bytes: &[u8], field: &'static str) -> Result<[u8; 32], KeynearError> {
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

#[derive(uniffi::Object)]
pub struct FfiNoisePairingSession {
    inner: Mutex<NoisePairingSession>,
}

impl fmt::Debug for FfiNoisePairingSession {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let guard = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.fmt(f)
    }
}

#[uniffi::export]
impl FfiNoisePairingSession {
    #[uniffi::constructor]
    pub fn new_initiator(
        local_static_private: Vec<u8>,
        local_device_id: String,
        local_display_name: String,
        local_role: FfiDeviceRole,
        capability_flags: u32,
    ) -> Result<Self, FfiError> {
        let key = to_key_32(&local_static_private, "localStaticPrivate")?;
        let metadata = PairingPeerMetadata {
            device_id: local_device_id,
            display_name: local_display_name,
            device_role: DeviceRole::from(local_role),
            capability_flags,
        };
        let session = NoisePairingSession::new_initiator(&key, metadata)?;
        Ok(Self {
            inner: Mutex::new(session),
        })
    }

    #[uniffi::constructor]
    pub fn new_responder(
        local_static_private: Vec<u8>,
        local_device_id: String,
        local_display_name: String,
        local_role: FfiDeviceRole,
        capability_flags: u32,
    ) -> Result<Self, FfiError> {
        let key = to_key_32(&local_static_private, "localStaticPrivate")?;
        let metadata = PairingPeerMetadata {
            device_id: local_device_id,
            display_name: local_display_name,
            device_role: DeviceRole::from(local_role),
            capability_flags,
        };
        let session = NoisePairingSession::new_responder(&key, metadata)?;
        Ok(Self {
            inner: Mutex::new(session),
        })
    }

    pub fn start_handshake(&self) -> Result<FfiPairingStep, FfiError> {
        let mut guard = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let outbound = guard.start_handshake()?;
        Ok(FfiPairingStep {
            outbound_frame: Some(outbound),
            state: FfiPairingState::from(guard.state()),
            verification_code: guard.formatted_verification_code().ok(),
            remaining_secs: guard.remaining_secs(),
        })
    }

    pub fn step(&self, inbound: Vec<u8>) -> Result<FfiPairingStep, FfiError> {
        let mut guard = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let outbound = guard.step(&inbound)?;
        Ok(FfiPairingStep {
            outbound_frame: outbound,
            state: FfiPairingState::from(guard.state()),
            verification_code: guard.formatted_verification_code().ok(),
            remaining_secs: guard.remaining_secs(),
        })
    }

    pub fn state(&self) -> FfiPairingState {
        let guard = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        FfiPairingState::from(guard.state())
    }

    pub fn remaining_secs(&self) -> u64 {
        let guard = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.remaining_secs()
    }

    pub fn formatted_verification_code(&self) -> Result<String, FfiError> {
        let guard = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.formatted_verification_code().map_err(FfiError::from)
    }

    pub fn approve_and_build_export(&self) -> Result<Vec<u8>, FfiError> {
        let mut guard = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.approve_and_build_export().map_err(FfiError::from)
    }

    pub fn reject(&self) -> Result<(), FfiError> {
        let mut guard = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.reject().map_err(FfiError::from)
    }

    pub fn complete_with_peer_export(
        &self,
        peer_ciphertext: Vec<u8>,
    ) -> Result<FfiPairingCompleted, FfiError> {
        let mut guard = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let completed = guard.complete_with_peer_export(&peer_ciphertext)?;
        Ok(FfiPairingCompleted::from(completed))
    }
}

#[derive(Debug, Default)]
struct VisibilityStateInner {
    mode: VisibilityMode,
    deadline: Option<Instant>,
}

impl VisibilityStateInner {
    fn refresh(&mut self, now: Instant) {
        if self.mode == VisibilityMode::EveryoneNearby {
            if let Some(deadline) = self.deadline {
                if now >= deadline {
                    self.mode = VisibilityMode::PairedOnly;
                    self.deadline = None;
                }
            }
        }
    }

    fn set_mode(&mut self, mode: VisibilityMode, now: Instant) -> VisibilityMode {
        self.mode = mode;
        self.deadline = if mode == VisibilityMode::EveryoneNearby {
            Some(now + Duration::from_secs(EVERYONE_NEARBY_TIMEOUT_SECS))
        } else {
            None
        };
        mode
    }

    fn remaining_secs(&mut self, now: Instant) -> u64 {
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

#[derive(Debug, Default, uniffi::Object)]
pub struct CoreHandle {
    visibility: Mutex<VisibilityStateInner>,
}

#[uniffi::export]
impl CoreHandle {
    #[uniffi::constructor]
    pub fn new() -> Self {
        Self {
            visibility: Mutex::new(VisibilityStateInner::default()),
        }
    }

    pub fn status(&self) -> CoreStatus {
        get_core_status()
    }

    pub fn visibility_mode(&self) -> FfiVisibilityMode {
        let mut guard = self
            .visibility
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.refresh(Instant::now());
        FfiVisibilityMode::from(guard.mode)
    }

    pub fn set_visibility_mode(&self, mode: FfiVisibilityMode) -> FfiVisibilityMode {
        let mut guard = self
            .visibility
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let updated = guard.set_mode(VisibilityMode::from(mode), Instant::now());
        FfiVisibilityMode::from(updated)
    }

    pub fn cycle_visibility_mode(&self) -> FfiVisibilityMode {
        let mut guard = self
            .visibility
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
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

    pub fn everyone_nearby_remaining_secs(&self) -> u64 {
        let mut guard = self
            .visibility
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.remaining_secs(Instant::now())
    }

    pub fn validate_config_toml(&self, raw_toml: String) -> Result<FfiVisibilityMode, FfiError> {
        let parsed: DaemonConfig =
            toml::from_str(&raw_toml).map_err(|err| KeynearError::Config(err.to_string()))?;
        parsed.validate()?;
        Ok(FfiVisibilityMode::from(parsed.visibility_mode))
    }

    pub fn validate_control_frame(&self, bytes: Vec<u8>) -> Result<String, FfiError> {
        let frame = decode_control_frame(&bytes)?;
        control_frame_to_envelope(&frame)?;
        Ok(frame.kind)
    }

    pub fn generate_static_keypair(&self) -> FfiStaticKeypair {
        let kp = StaticKeypairRecord::generate();
        FfiStaticKeypair {
            private_key: kp.private_key.to_vec(),
            public_key_hex: hex::encode(kp.public_key),
        }
    }

    pub fn encode_everyone_nearby_beacon(
        &self,
        role: FfiDeviceRole,
        capability_flags: u32,
        display_name: String,
    ) -> Result<Vec<u8>, FfiError> {
        let beacon = EveryoneNearbyBeacon {
            role: DeviceRole::from(role),
            capability_flags,
            display_name,
        };
        beacon.encode().map_err(FfiError::from)
    }

    pub fn decode_everyone_nearby_beacon(
        &self,
        service_data: Vec<u8>,
    ) -> Result<FfiEveryoneNearbyBeacon, FfiError> {
        let decoded = EveryoneNearbyBeacon::decode(&service_data)?;
        Ok(FfiEveryoneNearbyBeacon::from(decoded))
    }

    pub fn encode_paired_beacon(
        &self,
        device_id: String,
        counter: u32,
        reconnect_key: Vec<u8>,
        available: bool,
        battery_level: Option<u8>,
    ) -> Result<Vec<u8>, FfiError> {
        let key = to_key_32(&reconnect_key, "reconnectKey")?;
        let encoded = PairedBeacon::encode(&device_id, counter, &key, available, battery_level)?;
        Ok(encoded.to_vec())
    }

    pub fn decode_paired_beacon(
        &self,
        service_data: Vec<u8>,
        device_id: String,
        expected_counter: u32,
        last_accepted_counter: Option<u32>,
        reconnect_key: Vec<u8>,
    ) -> Result<FfiPairedBeaconState, FfiError> {
        let key = to_key_32(&reconnect_key, "reconnectKey")?;
        let decoded = PairedBeacon::decode_and_verify(
            &service_data,
            &device_id,
            expected_counter,
            last_accepted_counter,
            &key,
        )?;
        Ok(FfiPairedBeaconState::from(decoded))
    }

    pub fn derive_presence_token_bytes(
        &self,
        device_id: String,
        counter: u32,
        reconnect_key: Vec<u8>,
    ) -> Result<Vec<u8>, FfiError> {
        let key = to_key_32(&reconnect_key, "reconnectKey")?;
        let token = derive_presence_token(&device_id, counter, &key)?;
        Ok(token.to_vec())
    }

    pub fn load_paired_devices(
        &self,
        devices_toml_path: String,
    ) -> Result<Vec<FfiPairedDevice>, FfiError> {
        let store = PairedDeviceStore::new(PathBuf::from(devices_toml_path));
        let devices = store.load()?;
        Ok(devices.into_iter().map(FfiPairedDevice::from).collect())
    }

    pub fn save_paired_device(
        &self,
        devices_toml_path: String,
        device: FfiPairedDevice,
    ) -> Result<(), FfiError> {
        let store = PairedDeviceStore::new(PathBuf::from(devices_toml_path));
        store.upsert(PairedDevice::from(device))?;
        Ok(())
    }

    pub fn remove_paired_device(
        &self,
        devices_toml_path: String,
        device_id: String,
    ) -> Result<bool, FfiError> {
        let store = PairedDeviceStore::new(PathBuf::from(devices_toml_path));
        store.remove(&device_id).map_err(FfiError::from)
    }
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
pub fn everyone_nearby_timeout_secs() -> u64 {
    EVERYONE_NEARBY_TIMEOUT_SECS
}

#[uniffi::export]
pub fn pairing_timeout_secs() -> u64 {
    PAIRING_TIMEOUT_SECS
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::Arc;
    use std::thread;
    use std::time::{SystemTime, UNIX_EPOCH};

    use prost::Message;

    use super::*;
    use crate::proto::{encode_control_frame, envelope_to_control_frame, v1, ControlFrame};

    #[test]
    fn core_status_reports_ready() {
        let handle = CoreHandle::new();
        let status = handle.status();
        assert!(status.ready);
        assert_eq!(
            status.max_control_frame_bytes,
            MAX_CONTROL_FRAME_BYTES as u32
        );
        assert_eq!(handle.visibility_mode(), FfiVisibilityMode::PairedOnly);
        assert_eq!(ble_service_uuid16(), 0xFE6D);
        assert_eq!(everyone_nearby_timeout_secs(), 600);
        assert_eq!(pairing_timeout_secs(), 120);
    }

    #[test]
    fn core_handle_cycles_visibility_thread_safely() {
        let handle = Arc::new(CoreHandle::new());
        let mut workers = Vec::new();
        for _ in 0..6 {
            let shared = Arc::clone(&handle);
            workers.push(thread::spawn(move || {
                shared.cycle_visibility_mode();
            }));
        }
        for worker in workers {
            worker.join().expect("thread completes");
        }
        assert_eq!(handle.visibility_mode(), FfiVisibilityMode::PairedOnly);
    }

    #[test]
    fn everyone_nearby_deadline_auto_reverts_to_paired_only() {
        let now = Instant::now();
        let mut inner = VisibilityStateInner::default();
        inner.set_mode(VisibilityMode::EveryoneNearby, now);
        assert_eq!(inner.remaining_secs(now + Duration::from_secs(1)), 599);

        let expired_remaining = inner.remaining_secs(now + Duration::from_secs(600));
        assert_eq!(expired_remaining, 0);
        assert_eq!(inner.mode, VisibilityMode::PairedOnly);
    }

    #[test]
    fn ffi_beacons_and_noise_pairing_roundtrip() {
        let handle = CoreHandle::new();
        let beacon_bytes = handle
            .encode_everyone_nearby_beacon(FfiDeviceRole::Mobile, 0x03, "Pixel 9".to_string())
            .expect("encode everyone nearby");
        let decoded_beacon = handle
            .decode_everyone_nearby_beacon(beacon_bytes)
            .expect("decode everyone nearby");
        assert_eq!(decoded_beacon.role, FfiDeviceRole::Mobile);
        assert_eq!(decoded_beacon.display_name, "Pixel 9");

        let init_kp = handle.generate_static_keypair();
        let resp_kp = handle.generate_static_keypair();

        let initiator = FfiNoisePairingSession::new_initiator(
            init_kp.private_key,
            "phone-1".to_string(),
            "Pixel 9".to_string(),
            FfiDeviceRole::Mobile,
            0x03,
        )
        .expect("init session");
        let responder = FfiNoisePairingSession::new_responder(
            resp_kp.private_key,
            "linux-1".to_string(),
            "Workstation".to_string(),
            FfiDeviceRole::Desktop,
            0x07,
        )
        .expect("resp session");

        let m1 = initiator.start_handshake().expect("m1");
        let m2 = responder
            .step(m1.outbound_frame.expect("m1 bytes"))
            .expect("m2");
        let m3 = initiator
            .step(m2.outbound_frame.expect("m2 bytes"))
            .expect("m3");
        let fin = responder
            .step(m3.outbound_frame.expect("m3 bytes"))
            .expect("m3 read");
        assert_eq!(m3.state, FfiPairingState::AwaitingConfirmation);
        assert_eq!(fin.state, FfiPairingState::AwaitingConfirmation);
        assert_eq!(
            initiator.formatted_verification_code().expect("init code"),
            responder.formatted_verification_code().expect("resp code")
        );

        let init_export = initiator.approve_and_build_export().expect("init export");
        let resp_export = responder.approve_and_build_export().expect("resp export");
        let init_done = initiator
            .complete_with_peer_export(resp_export)
            .expect("init done");
        let resp_done = responder
            .complete_with_peer_export(init_export)
            .expect("resp done");
        assert_eq!(init_done.reconnect_secret, resp_done.reconnect_secret);

        let key_debug = format!("{init_done:?}");
        assert!(key_debug.contains("[REDACTED]"));
        assert!(!key_debug.contains(&hex::encode(&init_done.reconnect_secret)));

        let paired_bytes = handle
            .encode_paired_beacon(
                "phone-1".to_string(),
                5,
                init_done.reconnect_secret.clone(),
                true,
                Some(88),
            )
            .expect("encode paired beacon");
        let decoded_paired = handle
            .decode_paired_beacon(
                paired_bytes,
                "phone-1".to_string(),
                5,
                None,
                init_done.reconnect_secret,
            )
            .expect("decode paired beacon");
        assert_eq!(decoded_paired.counter, 5);
        assert!(decoded_paired.available);
        assert_eq!(decoded_paired.battery_level, Some(88));
    }

    #[test]
    fn ffi_paired_device_store_roundtrip() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("keynear-uniffi-test-{nonce}"));
        fs::create_dir_all(&dir).expect("tempdir");
        let path = dir.join("devices.toml").display().to_string();
        let handle = CoreHandle::new();

        handle
            .save_paired_device(
                path.clone(),
                FfiPairedDevice {
                    device_id: "linux-1".to_string(),
                    display_name: "Workstation".to_string(),
                    device_role: FfiDeviceRole::Desktop,
                    static_x25519_pubkey: "ab".repeat(32),
                    last_ble_address: "AA:BB:CC:DD:EE:FF".to_string(),
                    last_lan_address: String::new(),
                    capability_flags: 0x03,
                    auto_accept_files: false,
                    paired_at: "2026-11-13T20:00:00Z".to_string(),
                },
            )
            .expect("save device");

        let loaded = handle
            .load_paired_devices(path.clone())
            .expect("load devices");
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].device_id, "linux-1");
        assert!(handle
            .remove_paired_device(path.clone(), "linux-1".to_string())
            .expect("remove"));
        assert!(handle
            .load_paired_devices(path)
            .expect("empty after remove")
            .is_empty());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn validate_config_toml_checks_schema_and_validation() {
        let handle = CoreHandle::new();
        let mode = handle
            .validate_config_toml("visibilityMode = \"everyoneNearby\"\n".to_string())
            .expect("valid toml succeeds");
        assert_eq!(mode, FfiVisibilityMode::EveryoneNearby);

        let err = handle
            .validate_config_toml("incomingDirectory = \"\"\n".to_string())
            .expect_err("empty incoming directory fails");
        assert!(matches!(err, FfiError::Validation { .. }));
    }

    #[test]
    fn validate_control_frame_checks_envelope_and_size() {
        let handle = CoreHandle::new();
        let env = v1::Envelope {
            version: 1,
            sequence: 11,
            timestamp_ms: 1_763_000_000_000,
            payload: Some(v1::envelope::Payload::ControlAck(v1::ControlAck {
                acked_sequence: 10,
                accepted: true,
                reason: String::new(),
            })),
        };
        let frame = envelope_to_control_frame(&env).expect("envelope to frame");
        let bytes = encode_control_frame(&frame).expect("encode frame");
        let kind = handle
            .validate_control_frame(bytes)
            .expect("valid control frame accepted");
        assert_eq!(kind, "control_ack");

        let oversized = vec![0u8; MAX_CONTROL_FRAME_BYTES + 1];
        assert!(matches!(
            handle.validate_control_frame(oversized),
            Err(FfiError::FrameTooLarge { .. })
        ));

        let empty_payload = v1::ControlAck {
            acked_sequence: 1,
            accepted: true,
            reason: String::new(),
        }
        .encode_to_vec();
        let bad_kind = ControlFrame {
            version: 1,
            sequence: 1,
            timestamp_ms: 1,
            kind: "invalid".to_string(),
            payload: empty_payload,
        };
        let bad_bytes = encode_control_frame(&bad_kind).expect("encode bad kind frame");
        assert!(matches!(
            handle.validate_control_frame(bad_bytes),
            Err(FfiError::Protocol { .. })
        ));
    }
}
