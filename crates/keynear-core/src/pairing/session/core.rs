use super::*;

pub struct NoisePairingSession {
    pub(crate) role: PairingRole,
    pub(crate) state: PairingState,
    pub(crate) deadline: Instant,
    pub(crate) local_metadata: PairingPeerMetadata,
    pub(crate) handshake: Option<snow::HandshakeState>,
    pub(crate) transport: Option<snow::TransportState>,
    pub(crate) handshake_hash: Option<SecretBytes32>,
    pub(crate) peer_static_pubkey: Option<[u8; 32]>,
    pub(crate) verification_code: Option<String>,
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
}
