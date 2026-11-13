use std::collections::BTreeMap;
use std::fmt;
use std::sync::Mutex;

use x25519_dalek::{PublicKey, StaticSecret};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::error::KeynearError;
use crate::model::{sanitize_device_id, PairedDevice, PairedDeviceStore};

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
    fn get_peer_secrets(&self, device_id: &str) -> Result<Option<PeerSecrets>, KeynearError>;
    fn put_peer_secrets(&self, device_id: &str, secrets: &PeerSecrets) -> Result<(), KeynearError>;
    fn delete_peer_secrets(&self, device_id: &str) -> Result<(), KeynearError>;
    fn next_presence_counter(&self) -> Result<u32, KeynearError>;
    fn list_peer_ids(&self) -> Result<Vec<String>, KeynearError>;
}

#[derive(Default)]
struct InMemoryState {
    static_keypair: Option<StaticKeypairRecord>,
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

pub fn commit_paired_device(
    store: &PairedDeviceStore,
    secrets: &dyn SecretStore,
    device: PairedDevice,
    peer_secrets: &PeerSecrets,
) -> Result<Vec<PairedDevice>, KeynearError> {
    let clean_id = sanitize_device_id(&device.device_id)?;
    let previous_secrets = secrets.get_peer_secrets(&clean_id)?;
    secrets.put_peer_secrets(&clean_id, peer_secrets)?;

    match store.upsert(device) {
        Ok(updated) => Ok(updated),
        Err(err) => {
            let rollback = match previous_secrets {
                Some(ref prev) => secrets.put_peer_secrets(&clean_id, prev),
                None => secrets.delete_peer_secrets(&clean_id),
            };
            if let Err(rb_err) = rollback {
                tracing::error!(event = "pairing_commit_rollback_failed", error = %rb_err);
            }
            Err(err)
        }
    }
}

pub fn remove_paired_device(
    store: &PairedDeviceStore,
    secrets: &dyn SecretStore,
    device_id: &str,
) -> Result<bool, KeynearError> {
    let clean_id = sanitize_device_id(device_id)?;
    let previous_device = store.get(&clean_id)?;
    let removed = store.remove(&clean_id)?;
    if let Err(err) = secrets.delete_peer_secrets(&clean_id) {
        if let Some(prev) = previous_device {
            if let Err(rb_err) = store.upsert(prev) {
                tracing::error!(event = "unpair_rollback_failed", error = %rb_err);
            }
        }
        return Err(err);
    }
    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::DeviceRole;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(label: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("keynear-keyring-{label}-{nanos}"))
    }

    #[test]
    fn atomic_devices_store_rollback_and_monotonic_counter() {
        let dir = temp_dir("rollback");
        fs::create_dir_all(&dir).expect("create temp dir");
        let blocking_path = dir.join("not-a-dir");
        fs::write(&blocking_path, "block").expect("create blocking file");

        let bad_store = PairedDeviceStore::new(blocking_path.join("devices.toml"));
        let secrets = InMemorySecretStore::new();

        let dev = PairedDevice {
            device_id: "pixel-9".to_string(),
            display_name: "Pixel 9".to_string(),
            device_role: DeviceRole::Mobile,
            static_x25519_pubkey: "abcd".to_string(),
            last_ble_address: "11:22:33:44:55:66".to_string(),
            last_lan_address: String::new(),
            capability_flags: 1,
            auto_accept_files: false,
            paired_at: "2026-11-13T00:00:00Z".to_string(),
        };
        let peer_secrets = PeerSecrets {
            irk: [0x11; 32],
            reconnect_secret: [0x22; 32],
        };

        assert!(commit_paired_device(&bad_store, &secrets, dev.clone(), &peer_secrets).is_err());
        assert!(secrets
            .get_peer_secrets("pixel-9")
            .expect("query after rollback")
            .is_none());

        let good_store = PairedDeviceStore::new(dir.join("devices.toml"));
        let list = commit_paired_device(&good_store, &secrets, dev, &peer_secrets)
            .expect("commit succeeds");
        assert_eq!(list.len(), 1);
        assert_eq!(
            secrets.get_peer_secrets("pixel-9").unwrap(),
            Some(peer_secrets)
        );

        let c1 = secrets.next_presence_counter().expect("c1");
        let c2 = secrets.next_presence_counter().expect("c2");
        assert!(c2 > c1);

        assert!(remove_paired_device(&good_store, &secrets, "pixel-9").expect("remove succeeds"));
        assert!(good_store.load().unwrap().is_empty());
        assert!(secrets.get_peer_secrets("pixel-9").unwrap().is_none());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn secret_types_redact_debug_output() {
        let kp = StaticKeypairRecord::generate();
        let kp_dbg = format!("{kp:?}");
        assert!(!kp_dbg.contains(&hex::encode(kp.private_key)));
        assert!(kp_dbg.contains("[REDACTED]"));

        let ps = PeerSecrets {
            irk: [0xAA; 32],
            reconnect_secret: [0xBB; 32],
        };
        let ps_dbg = format!("{ps:?}");
        assert!(!ps_dbg.contains(&hex::encode(ps.irk)));
        assert!(!ps_dbg.contains(&hex::encode(ps.reconnect_secret)));
        assert!(ps_dbg.contains("[REDACTED]"));
    }
}
