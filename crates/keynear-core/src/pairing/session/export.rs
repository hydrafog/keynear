use super::*;

impl NoisePairingSession {
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
        validate_peer_export_frame(peer_ciphertext)?;

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

        let ikm = build_reconnect_ikm(self.role, local_contrib, peer_contrib);
        let reconnect_secret = derive_reconnect_secret(&handshake_hash.0, &ikm)?;

        self.state = PairingState::Completed;
        self.clear_secrets();

        Ok(PairingCompleted {
            peer_metadata,
            peer_static_pubkey,
            reconnect_secret,
        })
    }

    pub(crate) fn ensure_not_expired_at(&mut self, now: Instant) -> Result<(), KeynearError> {
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

    pub(crate) fn clear_secrets(&mut self) {
        self.handshake = None;
        self.transport = None;
        self.handshake_hash = None;
        self.local_contribution = None;
        if let Some(mut code) = self.verification_code.take() {
            code.zeroize();
        }
    }
}

fn validate_peer_export_frame(peer_ciphertext: &[u8]) -> Result<(), KeynearError> {
    if peer_ciphertext.is_empty() || peer_ciphertext.len() > MAX_GATT_PAIRING_FRAME_BYTES {
        return Err(KeynearError::Validation {
            field: "gattFrame",
            message: "peer export frame size out of bounds".to_string(),
        });
    }
    Ok(())
}

fn build_reconnect_ikm(
    role: PairingRole,
    local_contrib: &SecretBytes32,
    peer_contrib: &[u8],
) -> Zeroizing<[u8; 64]> {
    let mut ikm = Zeroizing::new([0u8; 64]);
    match role {
        PairingRole::Initiator => {
            ikm[..32].copy_from_slice(&local_contrib.0);
            ikm[32..].copy_from_slice(peer_contrib);
        }
        PairingRole::Responder => {
            ikm[..32].copy_from_slice(peer_contrib);
            ikm[32..].copy_from_slice(&local_contrib.0);
        }
    }
    ikm
}
