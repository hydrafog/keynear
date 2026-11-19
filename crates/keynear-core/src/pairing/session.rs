use std::fmt;
use std::time::{Duration, Instant};

use rand::Rng;
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

use keynear_protocol::{sanitize_device_id, DeviceRole, KeynearError, PAIRING_TIMEOUT_SECS};

use super::crypto::{
    derive_reconnect_secret, verification_code_from_hash, SecretBytes32, EXPORT_CONTRIBUTION_LEN,
    MAX_DISPLAY_NAME_BYTES, MAX_GATT_PAIRING_FRAME_BYTES, METADATA_FORMAT_VERSION,
    PAIRING_PROTOCOL,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairingRole {
    Initiator,
    Responder,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairingState {
    HandshakeM1,
    HandshakeM2,
    HandshakeM3,
    AwaitingConfirmation,
    Approved,
    Expired,
    Rejected,
    Completed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairingPeerMetadata {
    pub device_id: String,
    pub display_name: String,
    pub device_role: DeviceRole,
    pub capability_flags: u32,
}

impl PairingPeerMetadata {
    pub fn validate(&self) -> Result<Self, KeynearError> {
        let device_id = sanitize_device_id(&self.device_id)?;
        let display_name = self.display_name.trim().to_string();
        if display_name.is_empty() || display_name.len() > MAX_DISPLAY_NAME_BYTES {
            return Err(KeynearError::Validation {
                field: "displayName",
                message: "display_name must be between 1 and 64 bytes".to_string(),
            });
        }
        Ok(Self {
            device_id,
            display_name,
            device_role: self.device_role,
            capability_flags: self.capability_flags,
        })
    }

    pub fn encode(&self) -> Result<Vec<u8>, KeynearError> {
        let valid = self.validate()?;
        let id_bytes = valid.device_id.as_bytes();
        let name_bytes = valid.display_name.as_bytes();
        let role_byte = match valid.device_role {
            DeviceRole::Desktop => 0x01,
            DeviceRole::Mobile => 0x02,
        };
        let mut out = Vec::with_capacity(8 + id_bytes.len() + name_bytes.len());
        out.push(METADATA_FORMAT_VERSION);
        out.push(role_byte);
        out.extend_from_slice(&valid.capability_flags.to_le_bytes());
        out.push(id_bytes.len() as u8);
        out.extend_from_slice(id_bytes);
        out.push(name_bytes.len() as u8);
        out.extend_from_slice(name_bytes);
        Ok(out)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, KeynearError> {
        if bytes.len() < 8 {
            return Err(KeynearError::Protocol(
                "pairing metadata payload too short".to_string(),
            ));
        }
        if bytes[0] != METADATA_FORMAT_VERSION {
            return Err(KeynearError::Protocol(format!(
                "unsupported pairing metadata version {}",
                bytes[0]
            )));
        }
        let device_role = match bytes[1] {
            0x01 => DeviceRole::Desktop,
            0x02 => DeviceRole::Mobile,
            other => {
                return Err(KeynearError::Protocol(format!(
                    "unknown device role byte {other}"
                )));
            }
        };
        let mut cap_bytes = [0u8; 4];
        cap_bytes.copy_from_slice(&bytes[2..6]);
        let capability_flags = u32::from_le_bytes(cap_bytes);

        let id_len = bytes[6] as usize;
        let id_end = 7 + id_len;
        if id_len == 0 || id_end >= bytes.len() {
            return Err(KeynearError::Protocol(
                "invalid device_id length in pairing metadata".to_string(),
            ));
        }
        let device_id = std::str::from_utf8(&bytes[7..id_end])
            .map_err(|_| KeynearError::Protocol("invalid utf-8 device_id".to_string()))?
            .to_string();

        let name_len = bytes[id_end] as usize;
        let name_start = id_end + 1;
        let name_end = name_start + name_len;
        if name_len == 0 || name_end != bytes.len() {
            return Err(KeynearError::Protocol(
                "invalid display_name length in pairing metadata".to_string(),
            ));
        }
        let display_name = std::str::from_utf8(&bytes[name_start..name_end])
            .map_err(|_| KeynearError::Protocol("invalid utf-8 display_name".to_string()))?
            .to_string();

        Self {
            device_id,
            display_name,
            device_role,
            capability_flags,
        }
        .validate()
    }
}

#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct PairingCompleted {
    #[zeroize(skip)]
    pub peer_metadata: PairingPeerMetadata,
    #[zeroize(skip)]
    pub peer_static_pubkey: [u8; 32],
    pub reconnect_secret: [u8; 32],
}

impl fmt::Debug for PairingCompleted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PairingCompleted")
            .field("peer_metadata", &self.peer_metadata)
            .field("peer_static_pubkey", &hex::encode(self.peer_static_pubkey))
            .field("reconnect_secret", &"[REDACTED]")
            .finish()
    }
}

pub struct NoisePairingSession {
    role: PairingRole,
    state: PairingState,
    deadline: Instant,
    local_metadata: PairingPeerMetadata,
    handshake: Option<snow::HandshakeState>,
    transport: Option<snow::TransportState>,
    pub(crate) handshake_hash: Option<SecretBytes32>,
    peer_static_pubkey: Option<[u8; 32]>,
    verification_code: Option<String>,
    pub(crate) local_contribution: Option<SecretBytes32>,
}

impl fmt::Debug for NoisePairingSession {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NoisePairingSession")
            .field("role", &self.role)
            .field("state", &self.state)
            .field("local_device_id", &self.local_metadata.device_id)
            .field("verification_code", &"[REDACTED]")
            .field("handshake_hash", &"[REDACTED]")
            .field("local_contribution", &"[REDACTED]")
            .finish()
    }
}

impl Drop for NoisePairingSession {
    fn drop(&mut self) {
        self.clear_secrets();
    }
}

impl NoisePairingSession {
    pub fn new_initiator(
        local_static_private: &[u8; 32],
        local_metadata: PairingPeerMetadata,
    ) -> Result<Self, KeynearError> {
        Self::new_with_timeout(
            PairingRole::Initiator,
            local_static_private,
            local_metadata,
            Duration::from_secs(PAIRING_TIMEOUT_SECS),
        )
    }

    pub fn new_responder(
        local_static_private: &[u8; 32],
        local_metadata: PairingPeerMetadata,
    ) -> Result<Self, KeynearError> {
        Self::new_with_timeout(
            PairingRole::Responder,
            local_static_private,
            local_metadata,
            Duration::from_secs(PAIRING_TIMEOUT_SECS),
        )
    }

    pub fn new_with_timeout(
        role: PairingRole,
        local_static_private: &[u8; 32],
        local_metadata: PairingPeerMetadata,
        timeout: Duration,
    ) -> Result<Self, KeynearError> {
        let local_metadata = local_metadata.validate()?;
        let params: snow::params::NoiseParams = PAIRING_PROTOCOL
            .parse()
            .map_err(|err| KeynearError::Crypto(format!("invalid noise params: {err}")))?;
        let builder = snow::Builder::new(params)
            .local_private_key(local_static_private)
            .map_err(|err| KeynearError::Crypto(format!("invalid noise private key: {err}")))?;
        let handshake = match role {
            PairingRole::Initiator => builder.build_initiator(),
            PairingRole::Responder => builder.build_responder(),
        }
        .map_err(|err| KeynearError::Crypto(format!("noise build failed: {err}")))?;

        Ok(Self {
            role,
            state: PairingState::HandshakeM1,
            deadline: Instant::now() + timeout,
            local_metadata,
            handshake: Some(handshake),
            transport: None,
            handshake_hash: None,
            peer_static_pubkey: None,
            verification_code: None,
            local_contribution: None,
        })
    }

    pub fn role(&self) -> PairingRole {
        self.role
    }

    pub fn state(&self) -> PairingState {
        self.state
    }

    pub fn remaining_secs(&self) -> u64 {
        self.remaining_secs_at(Instant::now())
    }

    pub fn remaining_secs_at(&self, now: Instant) -> u64 {
        self.deadline
            .checked_duration_since(now)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    pub fn peer_static_pubkey(&self) -> Option<[u8; 32]> {
        self.peer_static_pubkey
    }

    pub fn verification_code(&self) -> Result<&str, KeynearError> {
        if self.state == PairingState::Expired {
            return Err(KeynearError::PairingExpired);
        }
        if self.state == PairingState::Rejected {
            return Err(KeynearError::PairingRejected);
        }
        self.verification_code
            .as_deref()
            .ok_or_else(|| KeynearError::Validation {
                field: "pairingState",
                message: "verification code is not available before handshake finishes".to_string(),
            })
    }

    pub fn formatted_verification_code(&self) -> Result<String, KeynearError> {
        let code = self.verification_code()?;
        Ok(format!("{} {}", &code[..3], &code[3..6]))
    }

    pub fn start_handshake(&mut self) -> Result<Vec<u8>, KeynearError> {
        self.start_handshake_at(Instant::now())
    }

    pub fn start_handshake_at(&mut self, now: Instant) -> Result<Vec<u8>, KeynearError> {
        self.ensure_not_expired_at(now)?;
        if self.role != PairingRole::Initiator || self.state != PairingState::HandshakeM1 {
            return Err(KeynearError::Validation {
                field: "pairingState",
                message: "start_handshake requires initiator in HandshakeM1".to_string(),
            });
        }
        let hs = self
            .handshake
            .as_mut()
            .ok_or_else(|| KeynearError::Protocol("missing noise handshake state".to_string()))?;
        let mut buf = vec![0u8; MAX_GATT_PAIRING_FRAME_BYTES];
        let written = hs
            .write_message(&[], &mut buf)
            .map_err(|err| KeynearError::Crypto(format!("noise write m1 failed: {err}")))?;
        buf.truncate(written);
        self.state = PairingState::HandshakeM2;
        Ok(buf)
    }

    pub fn step(&mut self, inbound: &[u8]) -> Result<Option<Vec<u8>>, KeynearError> {
        self.step_at(inbound, Instant::now())
    }

    pub fn step_at(
        &mut self,
        inbound: &[u8],
        now: Instant,
    ) -> Result<Option<Vec<u8>>, KeynearError> {
        self.ensure_not_expired_at(now)?;
        if inbound.is_empty() || inbound.len() > MAX_GATT_PAIRING_FRAME_BYTES {
            return Err(KeynearError::Validation {
                field: "gattFrame",
                message: format!(
                    "pairing frame size {} must be within 1..={}",
                    inbound.len(),
                    MAX_GATT_PAIRING_FRAME_BYTES
                ),
            });
        }

        let hs = self.handshake.as_mut().ok_or_else(|| match self.state {
            PairingState::Expired => KeynearError::PairingExpired,
            PairingState::Rejected => KeynearError::PairingRejected,
            _ => KeynearError::Validation {
                field: "pairingState",
                message: "handshake already finished or terminated".to_string(),
            },
        })?;

        let mut scratch = vec![0u8; MAX_GATT_PAIRING_FRAME_BYTES];
        match (self.role, self.state) {
            (PairingRole::Responder, PairingState::HandshakeM1) => {
                hs.read_message(inbound, &mut scratch)
                    .map_err(|err| KeynearError::Crypto(format!("noise read m1 failed: {err}")))?;
                let mut out = vec![0u8; MAX_GATT_PAIRING_FRAME_BYTES];
                let written = hs
                    .write_message(&[], &mut out)
                    .map_err(|err| KeynearError::Crypto(format!("noise write m2 failed: {err}")))?;
                out.truncate(written);
                self.state = PairingState::HandshakeM3;
                Ok(Some(out))
            }
            (PairingRole::Initiator, PairingState::HandshakeM2) => {
                hs.read_message(inbound, &mut scratch)
                    .map_err(|err| KeynearError::Crypto(format!("noise read m2 failed: {err}")))?;
                let mut out = vec![0u8; MAX_GATT_PAIRING_FRAME_BYTES];
                let written = hs
                    .write_message(&[], &mut out)
                    .map_err(|err| KeynearError::Crypto(format!("noise write m3 failed: {err}")))?;
                out.truncate(written);
                self.finish_handshake()?;
                Ok(Some(out))
            }
            (PairingRole::Responder, PairingState::HandshakeM3) => {
                hs.read_message(inbound, &mut scratch)
                    .map_err(|err| KeynearError::Crypto(format!("noise read m3 failed: {err}")))?;
                self.finish_handshake()?;
                Ok(None)
            }
            _ => Err(KeynearError::Validation {
                field: "pairingState",
                message: "unexpected handshake frame for current role and state".to_string(),
            }),
        }
    }

    fn finish_handshake(&mut self) -> Result<(), KeynearError> {
        let hs = self
            .handshake
            .as_ref()
            .ok_or_else(|| KeynearError::Protocol("missing noise handshake state".to_string()))?;
        if !hs.is_handshake_finished() {
            return Err(KeynearError::Crypto(
                "noise handshake did not finish after M3".to_string(),
            ));
        }

        let raw_hash = hs.get_handshake_hash();
        if raw_hash.len() < 32 {
            return Err(KeynearError::Crypto(
                "unexpected handshake hash length".to_string(),
            ));
        }
        let mut hash_arr = [0u8; 32];
        hash_arr.copy_from_slice(&raw_hash[..32]);
        let code = verification_code_from_hash(&hash_arr);

        let remote_static = hs
            .get_remote_static()
            .ok_or_else(|| KeynearError::Crypto("missing remote static key".to_string()))?;
        if remote_static.len() != 32 {
            return Err(KeynearError::Crypto(
                "invalid remote static key length".to_string(),
            ));
        }
        let mut remote_arr = [0u8; 32];
        remote_arr.copy_from_slice(remote_static);

        self.handshake_hash = Some(SecretBytes32(hash_arr));
        self.peer_static_pubkey = Some(remote_arr);
        self.verification_code = Some(code);
        self.state = PairingState::AwaitingConfirmation;
        Ok(())
    }

    pub fn approve_and_build_export(&mut self) -> Result<Vec<u8>, KeynearError> {
        self.approve_and_build_export_at(Instant::now())
    }

    pub fn approve_and_build_export_at(&mut self, now: Instant) -> Result<Vec<u8>, KeynearError> {
        self.ensure_not_expired_at(now)?;
        if self.state != PairingState::AwaitingConfirmation {
            return Err(KeynearError::Validation {
                field: "pairingState",
                message: "approve requires AwaitingConfirmation state".to_string(),
            });
        }

        let hs = self
            .handshake
            .take()
            .ok_or_else(|| KeynearError::Protocol("missing handshake state".to_string()))?;
        let mut transport = hs
            .into_transport_mode()
            .map_err(|err| KeynearError::Crypto(format!("noise transport switch failed: {err}")))?;

        let mut contrib = [0u8; EXPORT_CONTRIBUTION_LEN];
        rand::rng().fill_bytes(&mut contrib);

        let meta_bytes = self.local_metadata.encode()?;
        let mut plaintext = Zeroizing::new(Vec::with_capacity(
            EXPORT_CONTRIBUTION_LEN + meta_bytes.len(),
        ));
        plaintext.extend_from_slice(&contrib);
        plaintext.extend_from_slice(&meta_bytes);

        let mut out = vec![0u8; MAX_GATT_PAIRING_FRAME_BYTES];
        let written = transport
            .write_message(&plaintext, &mut out)
            .map_err(|err| KeynearError::Crypto(format!("export encryption failed: {err}")))?;
        out.truncate(written);

        self.local_contribution = Some(SecretBytes32(contrib));
        contrib.zeroize();
        self.transport = Some(transport);
        self.state = PairingState::Approved;
        Ok(out)
    }

    pub fn reject(&mut self) -> Result<(), KeynearError> {
        match self.state {
            PairingState::Expired => Err(KeynearError::PairingExpired),
            PairingState::Rejected => Err(KeynearError::PairingRejected),
            PairingState::Completed => Err(KeynearError::Validation {
                field: "pairingState",
                message: "cannot reject completed pairing".to_string(),
            }),
            _ => {
                self.state = PairingState::Rejected;
                self.clear_secrets();
                Ok(())
            }
        }
    }

    pub fn complete_with_peer_export(
        &mut self,
        peer_ciphertext: &[u8],
    ) -> Result<PairingCompleted, KeynearError> {
        self.complete_with_peer_export_at(peer_ciphertext, Instant::now())
    }

    pub fn complete_with_peer_export_at(
        &mut self,
        peer_ciphertext: &[u8],
        now: Instant,
    ) -> Result<PairingCompleted, KeynearError> {
        self.ensure_not_expired_at(now)?;
        if self.state != PairingState::Approved {
            return Err(KeynearError::Validation {
                field: "pairingState",
                message: "complete requires Approved state".to_string(),
            });
        }
        if peer_ciphertext.is_empty() || peer_ciphertext.len() > MAX_GATT_PAIRING_FRAME_BYTES {
            return Err(KeynearError::Validation {
                field: "gattFrame",
                message: "peer export frame size out of bounds".to_string(),
            });
        }

        let transport = self
            .transport
            .as_mut()
            .ok_or_else(|| KeynearError::Protocol("missing noise transport state".to_string()))?;
        let mut pt_buf = Zeroizing::new(vec![0u8; MAX_GATT_PAIRING_FRAME_BYTES]);
        let read_len = transport
            .read_message(peer_ciphertext, &mut pt_buf)
            .map_err(|err| KeynearError::Crypto(format!("peer export decryption failed: {err}")))?;
        if read_len <= EXPORT_CONTRIBUTION_LEN {
            return Err(KeynearError::Protocol(
                "peer export payload missing metadata".to_string(),
            ));
        }

        let peer_contrib = &pt_buf[..EXPORT_CONTRIBUTION_LEN];
        let peer_metadata =
            PairingPeerMetadata::decode(&pt_buf[EXPORT_CONTRIBUTION_LEN..read_len])?;

        let local_contrib = self
            .local_contribution
            .as_ref()
            .ok_or_else(|| KeynearError::Crypto("missing local export contribution".to_string()))?;
        let handshake_hash = self
            .handshake_hash
            .as_ref()
            .ok_or_else(|| KeynearError::Crypto("missing handshake hash".to_string()))?;
        let peer_static_pubkey = self
            .peer_static_pubkey
            .ok_or_else(|| KeynearError::Crypto("missing peer static pubkey".to_string()))?;

        let mut ikm = Zeroizing::new([0u8; 64]);
        match self.role {
            PairingRole::Initiator => {
                ikm[..32].copy_from_slice(&local_contrib.0);
                ikm[32..].copy_from_slice(peer_contrib);
            }
            PairingRole::Responder => {
                ikm[..32].copy_from_slice(peer_contrib);
                ikm[32..].copy_from_slice(&local_contrib.0);
            }
        }

        let reconnect_secret = derive_reconnect_secret(&handshake_hash.0, &ikm)?;

        self.state = PairingState::Completed;
        self.clear_secrets();

        Ok(PairingCompleted {
            peer_metadata,
            peer_static_pubkey,
            reconnect_secret,
        })
    }

    fn ensure_not_expired_at(&mut self, now: Instant) -> Result<(), KeynearError> {
        if self.state == PairingState::Expired {
            return Err(KeynearError::PairingExpired);
        }
        if self.state == PairingState::Rejected {
            return Err(KeynearError::PairingRejected);
        }
        if now >= self.deadline && self.state != PairingState::Completed {
            self.state = PairingState::Expired;
            self.clear_secrets();
            return Err(KeynearError::PairingExpired);
        }
        Ok(())
    }

    fn clear_secrets(&mut self) {
        self.handshake = None;
        self.transport = None;
        self.handshake_hash = None;
        self.local_contribution = None;
        if let Some(mut code) = self.verification_code.take() {
            code.zeroize();
        }
    }
}
