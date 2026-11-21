use crate::pairing::*;

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

#[derive(uniffi::Object)]
pub struct FfiNoisePairingSession {
    pub(crate) inner: Mutex<NoisePairingSession>,
}

impl fmt::Debug for FfiNoisePairingSession {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let guard = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        guard.fmt(f)
    }
}
