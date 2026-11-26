use crate::pairing::*;

#[uniffi::export]
impl FfiNoisePairingSession {
    #[uniffi::constructor]
    #[allow(clippy::too_many_arguments)]
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
    #[allow(clippy::too_many_arguments)]
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

#[cfg(test)]
mod tests {
    use super::*;

    fn initiator() -> FfiNoisePairingSession {
        FfiNoisePairingSession::new_initiator(
            vec![0x11; 32],
            "phone-1".to_string(),
            "Pixel 9".to_string(),
            FfiDeviceRole::Mobile,
            0x03,
        )
        .expect("initiator session")
    }

    #[test]
    fn state_starts_at_handshake_m1_for_both_roles() {
        assert_eq!(initiator().state(), FfiPairingState::HandshakeM1);
        let responder = FfiNoisePairingSession::new_responder(
            vec![0x22; 32],
            "linux-1".to_string(),
            "Workstation".to_string(),
            FfiDeviceRole::Desktop,
            0x07,
        )
        .expect("responder session");
        assert_eq!(responder.state(), FfiPairingState::HandshakeM1);
    }

    #[test]
    fn reject_moves_to_rejected_and_second_reject_fails() {
        let session = initiator();
        session.reject().expect("first reject succeeds");
        assert_eq!(session.state(), FfiPairingState::Rejected);
        assert!(matches!(session.reject(), Err(FfiError::PairingRejected)));
    }

    #[test]
    fn new_initiator_rejects_invalid_static_private_key() {
        assert!(matches!(
            FfiNoisePairingSession::new_initiator(
                vec![0u8; 5],
                "phone-1".to_string(),
                "Pixel 9".to_string(),
                FfiDeviceRole::Mobile,
                0x03,
            ),
            Err(FfiError::Validation { .. })
        ));
    }
}
