use std::fmt;
use std::sync::Mutex;

use keynear_core::{
    derive_presence_token, EveryoneNearbyBeacon, LanEndpointCodec, NoisePairingSession,
    PairedBeacon, PairedBeaconState, PairingCompleted, PairingPeerMetadata, PairingRole,
    PairingState,
};
use keynear_protocol::{v1, DeviceRole};

use crate::callbacks::{to_key_32, FfiDeviceRole, FfiError, FfiLanEndpoint};

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
    inner: Mutex<NoisePairingSession>,
}

impl fmt::Debug for FfiNoisePairingSession {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let guard = self.inner.lock().unwrap_or_else(|p| p.into_inner());
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
        let mut guard = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        let outbound = guard.start_handshake()?;
        Ok(FfiPairingStep {
            outbound_frame: Some(outbound),
            state: FfiPairingState::from(guard.state()),
            verification_code: guard.formatted_verification_code().ok(),
            remaining_secs: guard.remaining_secs(),
        })
    }

    pub fn step(&self, inbound: Vec<u8>) -> Result<FfiPairingStep, FfiError> {
        let mut guard = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        let outbound = guard.step(&inbound)?;
        Ok(FfiPairingStep {
            outbound_frame: outbound,
            state: FfiPairingState::from(guard.state()),
            verification_code: guard.formatted_verification_code().ok(),
            remaining_secs: guard.remaining_secs(),
        })
    }

    pub fn state(&self) -> FfiPairingState {
        let guard = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        FfiPairingState::from(guard.state())
    }

    pub fn remaining_secs(&self) -> u64 {
        let guard = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        guard.remaining_secs()
    }

    pub fn formatted_verification_code(&self) -> Result<String, FfiError> {
        let guard = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        guard.formatted_verification_code().map_err(FfiError::from)
    }

    pub fn approve_and_build_export(&self) -> Result<Vec<u8>, FfiError> {
        let mut guard = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        guard.approve_and_build_export().map_err(FfiError::from)
    }

    pub fn reject(&self) -> Result<(), FfiError> {
        let mut guard = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        guard.reject().map_err(FfiError::from)
    }

    pub fn complete_with_peer_export(
        &self,
        peer_ciphertext: Vec<u8>,
    ) -> Result<FfiPairingCompleted, FfiError> {
        let mut guard = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        let completed = guard.complete_with_peer_export(&peer_ciphertext)?;
        Ok(FfiPairingCompleted::from(completed))
    }
}

pub(crate) fn encode_everyone_nearby_beacon_impl(
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

pub(crate) fn decode_everyone_nearby_beacon_impl(
    service_data: Vec<u8>,
) -> Result<FfiEveryoneNearbyBeacon, FfiError> {
    let decoded = EveryoneNearbyBeacon::decode(&service_data)?;
    Ok(FfiEveryoneNearbyBeacon::from(decoded))
}

pub(crate) fn encode_paired_beacon_impl(
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

pub(crate) fn decode_paired_beacon_impl(
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

pub(crate) fn derive_presence_token_bytes_impl(
    device_id: String,
    counter: u32,
    reconnect_key: Vec<u8>,
) -> Result<Vec<u8>, FfiError> {
    let key = to_key_32(&reconnect_key, "reconnectKey")?;
    let token = derive_presence_token(&device_id, counter, &key)?;
    Ok(token.to_vec())
}

pub(crate) fn encode_lan_endpoint_impl(
    endpoint: FfiLanEndpoint,
    reconnect_key: Vec<u8>,
) -> Result<Vec<u8>, FfiError> {
    let key = to_key_32(&reconnect_key, "reconnectKey")?;
    let proto_ep = v1::LanEndpoint::from(endpoint);
    LanEndpointCodec::encode(&proto_ep, &key).map_err(FfiError::from)
}

pub(crate) fn decode_lan_endpoint_impl(
    encoded: Vec<u8>,
    expected_device_id: String,
    reconnect_key: Vec<u8>,
) -> Result<FfiLanEndpoint, FfiError> {
    let key = to_key_32(&reconnect_key, "reconnectKey")?;
    let decoded = LanEndpointCodec::decode(&encoded, &expected_device_id, &key)?;
    Ok(FfiLanEndpoint::from(decoded))
}
