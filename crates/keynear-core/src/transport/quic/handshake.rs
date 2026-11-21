use super::*;

use super::config::PeerAuthEntry;
use super::util::now_millis;

impl OrionQuicNode {
    pub async fn authenticate_outbound(
        &self,
        conn: &Connection,
        expected_peer_id: &str,
        reconnect_secret: &[u8; 32],
    ) -> Result<(), KeynearError> {
        let remote_eid = conn.remote_id();
        let (mut send, mut recv) = conn
            .open_bi()
            .await
            .map_err(|err| KeynearError::Transport(format!("open auth stream failed: {err}")))?;

        write_stream_kind(&mut send, QuicStreamKind::Control).await?;
        let local_handshake = self.build_handshake_envelope(&remote_eid, reconnect_secret)?;
        write_control_envelope(&mut send, &local_handshake).await?;
        send.finish()
            .map_err(|err| KeynearError::Transport(format!("finish auth stream failed: {err}")))?;

        let kind = read_stream_kind(&mut recv).await?;
        if kind != QuicStreamKind::Control {
            return Err(KeynearError::Protocol(
                "expected control stream for handshake".to_string(),
            ));
        }
        let reply = read_control_envelope(&mut recv).await?;
        let Some(v1::envelope::Payload::Handshake(peer_hs)) = reply.payload else {
            return Err(KeynearError::Protocol(
                "expected HandshakePayload in auth reply".to_string(),
            ));
        };
        if peer_hs.device_id != expected_peer_id {
            return Err(KeynearError::UnauthorizedPeer(
                "peer device_id mismatch during QUIC handshake".to_string(),
            ));
        }
        verify_handshake_auth_proof(
            reconnect_secret,
            &remote_eid,
            &self.endpoint.id(),
            &peer_hs.device_id,
            &peer_hs.auth_proof,
        )
    }

    pub async fn authenticate_inbound(&self, conn: &Connection) -> Result<String, KeynearError> {
        let remote_eid = conn.remote_id();
        let entry = self.find_paired_peer(&remote_eid)?;

        let (mut send, mut recv) = conn
            .accept_bi()
            .await
            .map_err(|err| KeynearError::Transport(format!("accept auth stream failed: {err}")))?;

        let kind = read_stream_kind(&mut recv).await?;
        if kind != QuicStreamKind::Control {
            return Err(KeynearError::Protocol(
                "expected control stream for inbound handshake".to_string(),
            ));
        }
        let env = read_control_envelope(&mut recv).await?;
        let Some(v1::envelope::Payload::Handshake(peer_hs)) = env.payload else {
            return Err(KeynearError::Protocol(
                "expected HandshakePayload on inbound connection".to_string(),
            ));
        };
        if peer_hs.device_id != entry.device_id {
            return Err(KeynearError::UnauthorizedPeer(
                "inbound handshake device_id mismatch".to_string(),
            ));
        }
        verify_handshake_auth_proof(
            &entry.reconnect_secret,
            &remote_eid,
            &self.endpoint.id(),
            &peer_hs.device_id,
            &peer_hs.auth_proof,
        )?;

        write_stream_kind(&mut send, QuicStreamKind::Control).await?;
        let reply = self.build_handshake_envelope(&remote_eid, &entry.reconnect_secret)?;
        write_control_envelope(&mut send, &reply).await?;
        send.finish()
            .map_err(|err| KeynearError::Transport(format!("finish auth reply failed: {err}")))?;

        Ok(entry.device_id.clone())
    }

    fn build_handshake_envelope(
        &self,
        remote_endpoint_id: &EndpointId,
        reconnect_secret: &[u8; 32],
    ) -> Result<v1::Envelope, KeynearError> {
        let proof = compute_handshake_auth_proof(
            reconnect_secret,
            &self.endpoint.id(),
            remote_endpoint_id,
            &self.config.device_id,
        )?;
        Ok(v1::Envelope {
            version: 1,
            sequence: 1,
            timestamp_ms: now_millis(),
            payload: Some(v1::envelope::Payload::Handshake(v1::HandshakePayload {
                device_id: self.config.device_id.clone(),
                display_name: self.config.display_name.clone(),
                ephemeral_pubkey: self.endpoint.id().as_bytes().to_vec(),
                capability_flags: self.config.capability_flags,
                auth_proof: proof.to_vec(),
            })),
        })
    }

    fn find_paired_peer(&self, remote_eid: &EndpointId) -> Result<PeerAuthEntry, KeynearError> {
        let allowlist = self
            .allowlist
            .read()
            .map_err(|_| KeynearError::Transport("allowlist lock poisoned".to_string()))?;
        let Some(device_id) = allowlist.get(remote_eid) else {
            return Err(KeynearError::UnauthorizedPeer(
                "remote endpoint is not paired".to_string(),
            ));
        };
        let peers = self
            .peers_by_device
            .read()
            .map_err(|_| KeynearError::Transport("peer auth registry lock poisoned".to_string()))?;
        peers
            .get(device_id)
            .cloned()
            .ok_or_else(|| KeynearError::UnauthorizedPeer("paired peer secret missing".to_string()))
    }
}
