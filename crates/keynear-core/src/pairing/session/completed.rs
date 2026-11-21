use super::*;

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
