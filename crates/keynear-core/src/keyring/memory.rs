use super::*;

#[derive(Default)]
struct InMemoryState {
    static_keypair: Option<StaticKeypairRecord>,
    quic_secret_key: Option<Zeroizing<[u8; 32]>>,
    peers: BTreeMap<String, PeerSecrets>,
    counter: u32,
}

#[derive(Default)]
pub struct InMemorySecretStore {
    state: Mutex<InMemoryState>,
}

impl InMemorySecretStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl SecretStore for InMemorySecretStore {
    fn get_or_create_static_keypair(&self) -> Result<StaticKeypairRecord, KeynearError> {
        let mut guard = self
            .state
            .lock()
            .map_err(|_| KeynearError::Keyring("in-memory keyring mutex poisoned".to_string()))?;
        if let Some(ref existing) = guard.static_keypair {
            return Ok(existing.clone());
        }
        let created = StaticKeypairRecord::generate();
        guard.static_keypair = Some(created.clone());
        Ok(created)
    }

    fn get_or_create_quic_secret_key(&self) -> Result<Zeroizing<[u8; 32]>, KeynearError> {
        let mut guard = self
            .state
            .lock()
            .map_err(|_| KeynearError::Keyring("in-memory keyring mutex poisoned".to_string()))?;
        if let Some(ref existing) = guard.quic_secret_key {
            return Ok(existing.clone());
        }
        let mut created = Zeroizing::new([0u8; 32]);
        rand::rng().fill_bytes(&mut created[..]);
        guard.quic_secret_key = Some(created.clone());
        Ok(created)
    }

    fn get_peer_secrets(&self, device_id: &str) -> Result<Option<PeerSecrets>, KeynearError> {
        let clean_id = sanitize_device_id(device_id)?;
        let guard = self
            .state
            .lock()
            .map_err(|_| KeynearError::Keyring("in-memory keyring mutex poisoned".to_string()))?;
        Ok(guard.peers.get(&clean_id).cloned())
    }

    fn put_peer_secrets(&self, device_id: &str, secrets: &PeerSecrets) -> Result<(), KeynearError> {
        let clean_id = sanitize_device_id(device_id)?;
        let mut guard = self
            .state
            .lock()
            .map_err(|_| KeynearError::Keyring("in-memory keyring mutex poisoned".to_string()))?;
        guard.peers.insert(clean_id, secrets.clone());
        Ok(())
    }

    fn delete_peer_secrets(&self, device_id: &str) -> Result<(), KeynearError> {
        let clean_id = sanitize_device_id(device_id)?;
        let mut guard = self
            .state
            .lock()
            .map_err(|_| KeynearError::Keyring("in-memory keyring mutex poisoned".to_string()))?;
        guard.peers.remove(&clean_id);
        Ok(())
    }

    fn next_presence_counter(&self) -> Result<u32, KeynearError> {
        let mut guard = self
            .state
            .lock()
            .map_err(|_| KeynearError::Keyring("in-memory keyring mutex poisoned".to_string()))?;
        guard.counter = guard.counter.checked_add(1).ok_or_else(|| {
            KeynearError::Crypto("presence counter exhausted u32 space".to_string())
        })?;
        Ok(guard.counter)
    }

    fn list_peer_ids(&self) -> Result<Vec<String>, KeynearError> {
        let guard = self
            .state
            .lock()
            .map_err(|_| KeynearError::Keyring("in-memory keyring mutex poisoned".to_string()))?;
        Ok(guard.peers.keys().cloned().collect())
    }
}
