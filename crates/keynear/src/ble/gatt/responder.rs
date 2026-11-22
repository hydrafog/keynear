use std::sync::Arc;

use keynear_core::{
    commit_paired_device, NoisePairingSession, PairedDeviceStore, PairingCompleted,
    PairingPeerMetadata, PairingState, PeerSecrets, SecretStore, MAX_GATT_PAIRING_FRAME_BYTES,
};
use keynear_protocol::{KeynearError, PairedDevice};
use zeroize::Zeroizing;

use super::GattInboundResult;

pub struct GattPairingResponder {
    local_metadata: PairingPeerMetadata,
    secret_store: Arc<dyn SecretStore>,
    device_store: PairedDeviceStore,
    session: Option<NoisePairingSession>,
    local_export: Option<Vec<u8>>,
    pending_peer_export: Option<(Vec<u8>, Option<String>)>,
    peer_ble_address: Option<String>,
}

impl GattPairingResponder {
    pub fn new(
        local_metadata: PairingPeerMetadata,
        secret_store: Arc<dyn SecretStore>,
        device_store: PairedDeviceStore,
    ) -> Self {
        Self {
            local_metadata,
            secret_store,
            device_store,
            session: None,
            local_export: None,
            pending_peer_export: None,
            peer_ble_address: None,
        }
    }

    fn clear_session(&mut self) {
        self.session = None;
        self.local_export = None;
        self.pending_peer_export = None;
    }

    pub fn verification_code(&self) -> Option<String> {
        self.session
            .as_ref()
            .and_then(|s| s.formatted_verification_code().ok())
    }

    pub fn begin_session(&mut self, peer_address: Option<String>) -> Result<(), KeynearError> {
        let kp = self.secret_store.get_or_create_static_keypair()?;
        let priv_bytes = Zeroizing::new(kp.private_key);
        let session = NoisePairingSession::new_responder(&priv_bytes, self.local_metadata.clone())?;
        self.session = Some(session);
        self.local_export = None;
        self.pending_peer_export = None;
        self.peer_ble_address = peer_address;
        Ok(())
    }

    fn handle_handshake_frame(&mut self, frame: &[u8]) -> Result<GattInboundResult, KeynearError> {
        let fallback = self.verification_code();
        let session = self
            .session
            .as_mut()
            .ok_or_else(|| KeynearError::Protocol("missing pairing session".to_string()))?;
        let outbound = session.step(frame)?;
        if let Some(m2) = outbound {
            return Ok(GattInboundResult::Outbound(m2));
        }
        let code = session.formatted_verification_code()?;
        Ok(GattInboundResult::AwaitingApproval {
            verification_code: fallback.unwrap_or(code),
        })
    }

    fn build_paired_device(
        &self,
        completed: &PairingCompleted,
        peer_address: Option<String>,
    ) -> PairedDevice {
        PairedDevice {
            device_id: completed.peer_metadata.device_id.clone(),
            display_name: completed.peer_metadata.display_name.clone(),
            device_role: completed.peer_metadata.device_role,
            static_x25519_pubkey: hex::encode(completed.peer_static_pubkey),
            last_ble_address: self
                .peer_ble_address
                .clone()
                .or(peer_address)
                .unwrap_or_default(),
            last_lan_address: String::new(),
            last_endpoint_id: String::new(),
            capability_flags: completed.peer_metadata.capability_flags,
            auto_accept_files: false,
            paired_at: format!(
                "{}",
                std::time::UNIX_EPOCH
                    .elapsed()
                    .map(|d| d.as_secs())
                    .unwrap_or(0)
            ),
        }
    }

    fn handle_approved_frame(
        &mut self,
        frame: &[u8],
        peer_address: Option<String>,
    ) -> Result<GattInboundResult, KeynearError> {
        let completed = {
            let session = self
                .session
                .as_mut()
                .ok_or_else(|| KeynearError::Protocol("missing pairing session".to_string()))?;
            session.complete_with_peer_export(frame)?
        };
        let device = self.build_paired_device(&completed, peer_address);
        let secrets = PeerSecrets {
            irk: completed.reconnect_secret,
            reconnect_secret: completed.reconnect_secret,
        };
        commit_paired_device(
            &self.device_store,
            self.secret_store.as_ref(),
            device,
            &secrets,
        )?;
        self.clear_session();
        Ok(GattInboundResult::Completed(completed))
    }

    pub fn handle_tx_write(
        &mut self,
        frame: &[u8],
        peer_address: Option<String>,
    ) -> Result<GattInboundResult, KeynearError> {
        if frame.is_empty() || frame.len() > MAX_GATT_PAIRING_FRAME_BYTES {
            return Err(KeynearError::Validation {
                field: "gattFrame",
                message: format!(
                    "pairing frame length {} must be within 1..={}",
                    frame.len(),
                    MAX_GATT_PAIRING_FRAME_BYTES
                ),
            });
        }

        if self.session.is_none() {
            self.begin_session(peer_address.clone())?;
        }

        let state = self
            .session
            .as_ref()
            .ok_or_else(|| KeynearError::Protocol("missing pairing session".to_string()))?
            .state();

        match state {
            PairingState::HandshakeM1 | PairingState::HandshakeM3 => {
                self.handle_handshake_frame(frame)
            }
            PairingState::AwaitingConfirmation => {
                self.pending_peer_export = Some((frame.to_vec(), peer_address));
                Ok(GattInboundResult::PeerExportBuffered)
            }
            PairingState::Approved => self.handle_approved_frame(frame, peer_address),
            PairingState::Expired => {
                self.clear_session();
                Err(KeynearError::PairingExpired)
            }
            PairingState::Rejected => {
                self.clear_session();
                Err(KeynearError::PairingRejected)
            }
            _ => Err(KeynearError::Validation {
                field: "pairingState",
                message: "unexpected GATT pairing frame for current state".to_string(),
            }),
        }
    }

    pub fn approve(&mut self) -> Result<Vec<u8>, KeynearError> {
        let session = self
            .session
            .as_mut()
            .ok_or_else(|| KeynearError::Validation {
                field: "pairingState",
                message: "no pending pairing session to approve".to_string(),
            })?;
        let export = session.approve_and_build_export()?;
        self.local_export = Some(export.clone());
        Ok(export)
    }

    pub fn complete_pending_export(&mut self) -> Result<Option<PairingCompleted>, KeynearError> {
        let Some((frame, peer_address)) = self.pending_peer_export.take() else {
            return Ok(None);
        };
        match self.handle_approved_frame(&frame, peer_address)? {
            GattInboundResult::Completed(completed) => Ok(Some(completed)),
            _ => Ok(None),
        }
    }

    pub fn reject(&mut self) -> Result<(), KeynearError> {
        self.pending_peer_export = None;
        if let Some(mut session) = self.session.take() {
            let res = session.reject();
            self.local_export = None;
            return res;
        }
        Ok(())
    }
}
