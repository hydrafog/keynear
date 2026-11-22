use rand::Rng;
use zeroize::Zeroizing;

use keynear_core::keyring::{PeerSecrets, SecretStore, StaticKeypairRecord};
use keynear_protocol::error::KeynearError;
use keynear_protocol::model::sanitize_device_id;

use super::backend::{delete_secret, get_secret_bytes, put_secret_bytes};
use super::{
    Oo7SecretStore, APP_ATTR_KEY, APP_ATTR_VAL, DEVICE_ATTR_KEY, KIND_ATTR_KEY, KIND_PEER_SECRETS,
    KIND_PRESENCE_COUNTER, KIND_QUIC_SECRET_KEY, KIND_STATIC_KEYPAIR,
};

impl SecretStore for Oo7SecretStore {
    fn get_or_create_static_keypair(&self) -> Result<StaticKeypairRecord, KeynearError> {
        let backend = self.backend.clone();
        self.exec(async move {
            let attrs = [
                (APP_ATTR_KEY, APP_ATTR_VAL),
                (KIND_ATTR_KEY, KIND_STATIC_KEYPAIR),
            ];
            if let Some(bytes) = get_secret_bytes(&backend, &attrs).await? {
                if bytes.len() == 32 {
                    let mut priv_arr = [0u8; 32];
                    priv_arr.copy_from_slice(&bytes);
                    return Ok(StaticKeypairRecord::from_private_bytes(priv_arr));
                }
                return Err(KeynearError::Keyring(
                    "stored static X25519 key has invalid length".to_string(),
                ));
            }
            let created = StaticKeypairRecord::generate();
            put_secret_bytes(
                &backend,
                "Keynear Static Identity",
                &attrs,
                &created.private_key,
            )
            .await?;
            Ok(created)
        })
    }

    fn get_or_create_quic_secret_key(&self) -> Result<Zeroizing<[u8; 32]>, KeynearError> {
        let backend = self.backend.clone();
        self.exec(async move {
            let attrs = [
                (APP_ATTR_KEY, APP_ATTR_VAL),
                (KIND_ATTR_KEY, KIND_QUIC_SECRET_KEY),
            ];
            if let Some(bytes) = get_secret_bytes(&backend, &attrs).await? {
                if bytes.len() == 32 {
                    let mut key = Zeroizing::new([0u8; 32]);
                    key.copy_from_slice(&bytes);
                    return Ok(key);
                }
                return Err(KeynearError::Keyring(
                    "stored QUIC Ed25519 key has invalid length".to_string(),
                ));
            }
            let mut created = Zeroizing::new([0u8; 32]);
            rand::rng().fill_bytes(&mut created[..]);
            put_secret_bytes(&backend, "Keynear QUIC Identity", &attrs, &created[..]).await?;
            Ok(created)
        })
    }

    fn get_peer_secrets(&self, device_id: &str) -> Result<Option<PeerSecrets>, KeynearError> {
        let clean_id = sanitize_device_id(device_id)?;
        let backend = self.backend.clone();
        self.exec(async move {
            let attrs = [
                (APP_ATTR_KEY, APP_ATTR_VAL),
                (KIND_ATTR_KEY, KIND_PEER_SECRETS),
                (DEVICE_ATTR_KEY, clean_id.as_str()),
            ];
            let Some(bytes) = get_secret_bytes(&backend, &attrs).await? else {
                return Ok(None);
            };
            if bytes.len() != 64 {
                return Err(KeynearError::Keyring(
                    "stored peer secrets have invalid length".to_string(),
                ));
            }
            let mut irk = [0u8; 32];
            let mut reconnect_secret = [0u8; 32];
            irk.copy_from_slice(&bytes[..32]);
            reconnect_secret.copy_from_slice(&bytes[32..64]);
            Ok(Some(PeerSecrets {
                irk,
                reconnect_secret,
            }))
        })
    }

    fn put_peer_secrets(&self, device_id: &str, secrets: &PeerSecrets) -> Result<(), KeynearError> {
        let clean_id = sanitize_device_id(device_id)?;
        let backend = self.backend.clone();
        let mut combined = Zeroizing::new([0u8; 64]);
        combined[..32].copy_from_slice(&secrets.irk);
        combined[32..64].copy_from_slice(&secrets.reconnect_secret);
        self.exec(async move {
            let attrs = [
                (APP_ATTR_KEY, APP_ATTR_VAL),
                (KIND_ATTR_KEY, KIND_PEER_SECRETS),
                (DEVICE_ATTR_KEY, clean_id.as_str()),
            ];
            let label = format!("Keynear Peer {clean_id}");
            put_secret_bytes(&backend, &label, &attrs, &combined[..]).await
        })
    }

    fn delete_peer_secrets(&self, device_id: &str) -> Result<(), KeynearError> {
        let clean_id = sanitize_device_id(device_id)?;
        let backend = self.backend.clone();
        self.exec(async move {
            let attrs = [
                (APP_ATTR_KEY, APP_ATTR_VAL),
                (KIND_ATTR_KEY, KIND_PEER_SECRETS),
                (DEVICE_ATTR_KEY, clean_id.as_str()),
            ];
            delete_secret(&backend, &attrs).await
        })
    }

    fn next_presence_counter(&self) -> Result<u32, KeynearError> {
        let backend = self.backend.clone();
        self.exec(async move {
            let attrs = [
                (APP_ATTR_KEY, APP_ATTR_VAL),
                (KIND_ATTR_KEY, KIND_PRESENCE_COUNTER),
            ];
            let current = match get_secret_bytes(&backend, &attrs).await? {
                Some(bytes) if bytes.len() == 4 => {
                    u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
                }
                Some(_) => {
                    return Err(KeynearError::Keyring(
                        "stored presence counter has invalid length".to_string(),
                    ));
                }
                None => 0,
            };
            let next = current.checked_add(1).ok_or_else(|| {
                KeynearError::Crypto("presence counter exhausted u32 space".to_string())
            })?;
            put_secret_bytes(
                &backend,
                "Keynear Presence Counter",
                &attrs,
                &next.to_be_bytes(),
            )
            .await?;
            Ok(next)
        })
    }

    fn list_peer_ids(&self) -> Result<Vec<String>, KeynearError> {
        let backend = self.backend.clone();
        self.exec(async move {
            let attrs = [
                (APP_ATTR_KEY, APP_ATTR_VAL),
                (KIND_ATTR_KEY, KIND_PEER_SECRETS),
            ];
            let mut ids = Vec::new();
            match &backend {
                super::Backend::Host(keyring) => {
                    let items = keyring
                        .search_items(&attrs)
                        .await
                        .map_err(|err| KeynearError::Keyring(err.to_string()))?;
                    for item in items {
                        let item_attrs = item
                            .attributes()
                            .await
                            .map_err(|err| KeynearError::Keyring(err.to_string()))?;
                        if let Some(id) = item_attrs.get(DEVICE_ATTR_KEY) {
                            ids.push(id.clone());
                        }
                    }
                }
                super::Backend::File(keyring) => {
                    let items = keyring
                        .search_items(&attrs)
                        .await
                        .map_err(|err| KeynearError::Keyring(err.to_string()))?;
                    for item in items {
                        if let Some(id) = item.attributes().get(DEVICE_ATTR_KEY) {
                            ids.push(id.to_string());
                        }
                    }
                }
            }
            ids.sort();
            ids.dedup();
            Ok(ids)
        })
    }
}
