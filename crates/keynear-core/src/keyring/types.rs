use super::*;

#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct StaticKeypairRecord {
    pub private_key: [u8; 32],
    #[zeroize(skip)]
    pub public_key: [u8; 32],
}

impl fmt::Debug for StaticKeypairRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StaticKeypairRecord")
            .field("private_key", &"[REDACTED]")
            .field("public_key", &hex::encode(self.public_key))
            .finish()
    }
}

impl StaticKeypairRecord {
    pub fn generate() -> Self {
        let secret = StaticSecret::random();
        let public = PublicKey::from(&secret);
        Self {
            private_key: secret.to_bytes(),
            public_key: public.to_bytes(),
        }
    }

    pub fn from_private_bytes(private_key: [u8; 32]) -> Self {
        let secret = StaticSecret::from(private_key);
        let public = PublicKey::from(&secret);
        Self {
            private_key,
            public_key: public.to_bytes(),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Zeroize, ZeroizeOnDrop)]
pub struct PeerSecrets {
    pub irk: [u8; 32],
    pub reconnect_secret: [u8; 32],
}

impl fmt::Debug for PeerSecrets {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PeerSecrets")
            .field("irk", &"[REDACTED]")
            .field("reconnect_secret", &"[REDACTED]")
            .finish()
    }
}

pub trait SecretStore: Send + Sync {
    fn get_or_create_static_keypair(&self) -> Result<StaticKeypairRecord, KeynearError>;
    fn get_or_create_quic_secret_key(&self) -> Result<Zeroizing<[u8; 32]>, KeynearError>;
    fn get_peer_secrets(&self, device_id: &str) -> Result<Option<PeerSecrets>, KeynearError>;
    fn put_peer_secrets(&self, device_id: &str, secrets: &PeerSecrets) -> Result<(), KeynearError>;
    fn delete_peer_secrets(&self, device_id: &str) -> Result<(), KeynearError>;
    fn next_presence_counter(&self) -> Result<u32, KeynearError>;
    fn list_peer_ids(&self) -> Result<Vec<String>, KeynearError>;
}
