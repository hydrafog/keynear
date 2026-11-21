use super::*;

#[derive(Clone)]
pub struct OrionNodeConfig {
    pub device_id: String,
    pub display_name: String,
    pub capability_flags: u32,
    pub quic_secret_key: Zeroizing<[u8; 32]>,
}

impl fmt::Debug for OrionNodeConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OrionNodeConfig")
            .field("device_id", &self.device_id)
            .field("display_name", &self.display_name)
            .field("capability_flags", &self.capability_flags)
            .field("quic_secret_key", &"[REDACTED]")
            .finish()
    }
}

#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub(crate) struct PeerAuthEntry {
    #[zeroize(skip)]
    pub(crate) device_id: String,
    #[zeroize(skip)]
    pub(crate) endpoint_id: EndpointId,
    #[zeroize(skip)]
    pub(crate) socket_addrs: Vec<SocketAddr>,
    pub(crate) reconnect_secret: [u8; 32],
}

#[derive(Debug, Clone)]
pub(crate) struct PairedPeerHooks {
    pub(crate) allowlist: Arc<RwLock<HashMap<EndpointId, String>>>,
}

impl EndpointHooks for PairedPeerHooks {
    async fn after_handshake(&self, conn: &Connection) -> AfterHandshakeOutcome {
        let remote_id = conn.remote_id();
        let allowed = self
            .allowlist
            .read()
            .map(|guard| guard.contains_key(&remote_id))
            .unwrap_or(false);
        if allowed {
            AfterHandshakeOutcome::Accept
        } else {
            warn!(
                event = "quic_unpaired_peer_rejected",
                remote_id = %remote_id
            );
            AfterHandshakeOutcome::Reject {
                error_code: QUIC_UNAUTHORIZED_CODE.into(),
                reason: b"unpaired_peer".to_vec(),
            }
        }
    }
}
