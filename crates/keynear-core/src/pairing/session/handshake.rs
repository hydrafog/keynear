use super::*;

impl NoisePairingSession {
    pub fn step(&mut self, inbound: &[u8]) -> Result<Option<Vec<u8>>, KeynearError> {
        self.step_at(inbound, Instant::now())
    }

    pub fn step_at(
        &mut self,
        inbound: &[u8],
        now: Instant,
    ) -> Result<Option<Vec<u8>>, KeynearError> {
        self.ensure_not_expired_at(now)?;
        validate_pairing_frame(inbound)?;

        let hs = require_handshake(&mut self.handshake, self.state)?;

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
}

fn require_handshake(
    handshake: &mut Option<snow::HandshakeState>,
    state: PairingState,
) -> Result<&mut snow::HandshakeState, KeynearError> {
    handshake.as_mut().ok_or_else(|| match state {
        PairingState::Expired => KeynearError::PairingExpired,
        PairingState::Rejected => KeynearError::PairingRejected,
        _ => KeynearError::Validation {
            field: "pairingState",
            message: "handshake already finished or terminated".to_string(),
        },
    })
}

fn validate_pairing_frame(inbound: &[u8]) -> Result<(), KeynearError> {
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
    Ok(())
}
