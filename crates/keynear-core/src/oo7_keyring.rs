use std::fs::{self, OpenOptions};
use std::future::Future;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc};

use oo7::file::UnlockedKeyring;
use oo7::{Keyring, Secret};
use rand::Rng;
use tokio::runtime::{Builder, Runtime};
use tracing::warn;
use zeroize::Zeroizing;

use crate::error::KeynearError;
use crate::keyring::{PeerSecrets, SecretStore, StaticKeypairRecord};
use crate::model::{default_config_dir, sanitize_device_id};

const APP_ATTR_KEY: &str = "app";
const APP_ATTR_VAL: &str = "keynear";
const KIND_ATTR_KEY: &str = "kind";
const DEVICE_ATTR_KEY: &str = "device_id";

const KIND_STATIC_KEYPAIR: &str = "static_x25519";
const KIND_PEER_SECRETS: &str = "peer_secrets";
const KIND_PRESENCE_COUNTER: &str = "presence_counter";

#[derive(Clone)]
enum Backend {
    Host(Arc<Keyring>),
    File(Arc<UnlockedKeyring>),
}

pub struct Oo7SecretStore {
    runtime: Runtime,
    backend: Backend,
    is_file_fallback: bool,
}

impl std::fmt::Debug for Oo7SecretStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Oo7SecretStore")
            .field("is_file_fallback", &self.is_file_fallback)
            .finish()
    }
}

impl Oo7SecretStore {
    pub fn open_default() -> Result<Self, KeynearError> {
        Self::open_in(&default_config_dir())
    }

    pub fn open_in(config_dir: &Path) -> Result<Self, KeynearError> {
        let runtime = build_runtime()?;
        let config_dir = config_dir.to_path_buf();
        let (backend, is_file_fallback) = run_on_runtime(&runtime, async move {
            match Keyring::host().await {
                Ok(keyring) => Ok((Backend::Host(Arc::new(keyring)), false)),
                Err(host_err) => {
                    warn!(
                        event = "secret_service_unavailable_using_file_fallback",
                        error = %host_err
                    );
                    let file_keyring = open_file_keyring(&config_dir).await?;
                    Ok((Backend::File(Arc::new(file_keyring)), true))
                }
            }
        })?;
        Ok(Self {
            runtime,
            backend,
            is_file_fallback,
        })
    }

    pub fn open_file_only(config_dir: &Path) -> Result<Self, KeynearError> {
        let runtime = build_runtime()?;
        let config_dir = config_dir.to_path_buf();
        let file_keyring =
            run_on_runtime(
                &runtime,
                async move { open_file_keyring(&config_dir).await },
            )?;
        Ok(Self {
            runtime,
            backend: Backend::File(Arc::new(file_keyring)),
            is_file_fallback: true,
        })
    }

    pub fn is_file_fallback(&self) -> bool {
        self.is_file_fallback
    }

    fn exec<F, T>(&self, fut: F) -> Result<T, KeynearError>
    where
        F: Future<Output = Result<T, KeynearError>> + Send + 'static,
        T: Send + 'static,
    {
        run_on_runtime(&self.runtime, fut)
    }
}

fn build_runtime() -> Result<Runtime, KeynearError> {
    Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .map_err(|err| KeynearError::Keyring(format!("failed to create keyring runtime: {err}")))
}

fn run_on_runtime<F, T>(runtime: &Runtime, fut: F) -> Result<T, KeynearError>
where
    F: Future<Output = Result<T, KeynearError>> + Send + 'static,
    T: Send + 'static,
{
    if tokio::runtime::Handle::try_current().is_ok() {
        let (tx, rx) = mpsc::sync_channel(1);
        runtime.spawn(async move {
            let _ = tx.send(fut.await);
        });
        rx.recv()
            .map_err(|_| KeynearError::Keyring("keyring worker task cancelled".to_string()))?
    } else {
        runtime.block_on(fut)
    }
}

async fn open_file_keyring(config_dir: &Path) -> Result<UnlockedKeyring, KeynearError> {
    fs::create_dir_all(config_dir)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(config_dir, fs::Permissions::from_mode(0o700));
    }

    let keyfile_path = config_dir.join("keyring.key");
    let keyring_path = config_dir.join("keyring.oo7");
    let secret = load_or_create_keyfile(&keyfile_path)?;

    UnlockedKeyring::load(&keyring_path, Some(secret))
        .await
        .map_err(|err| KeynearError::Keyring(format!("failed to open oo7 file keyring: {err}")))
}

fn load_or_create_keyfile(path: &PathBuf) -> Result<Secret, KeynearError> {
    if path.exists() {
        let bytes = Zeroizing::new(fs::read(path)?);
        if bytes.len() < 32 {
            return Err(KeynearError::Keyring(
                "local keyring.key is truncated or invalid".to_string(),
            ));
        }
        return Ok(Secret::blob(bytes.as_slice()));
    }

    let mut key = Zeroizing::new([0u8; 32]);
    rand::rng().fill_bytes(&mut key[..]);
    let mut opts = OpenOptions::new();
    opts.create_new(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut file = opts.open(path)?;
    file.write_all(&key[..])?;
    file.sync_all()?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    }
    Ok(Secret::blob(&key[..]))
}

async fn get_secret_bytes(
    backend: &Backend,
    attrs: &[(&str, &str)],
) -> Result<Option<Zeroizing<Vec<u8>>>, KeynearError> {
    match backend {
        Backend::Host(keyring) => {
            let items = keyring
                .search_items(&attrs)
                .await
                .map_err(|err| KeynearError::Keyring(err.to_string()))?;
            if let Some(item) = items.into_iter().next() {
                let secret = item
                    .secret()
                    .await
                    .map_err(|err| KeynearError::Keyring(err.to_string()))?;
                Ok(Some(Zeroizing::new(secret.as_bytes().to_vec())))
            } else {
                Ok(None)
            }
        }
        Backend::File(keyring) => {
            let item = keyring
                .lookup_item(&attrs)
                .await
                .map_err(|err| KeynearError::Keyring(err.to_string()))?;
            Ok(item.map(|it| Zeroizing::new(it.secret().as_bytes().to_vec())))
        }
    }
}

async fn put_secret_bytes(
    backend: &Backend,
    label: &str,
    attrs: &[(&str, &str)],
    bytes: &[u8],
) -> Result<(), KeynearError> {
    let secret = Secret::blob(bytes);
    match backend {
        Backend::Host(keyring) => keyring
            .create_item(label, &attrs, secret, true)
            .await
            .map_err(|err| KeynearError::Keyring(err.to_string())),
        Backend::File(keyring) => keyring
            .create_item(label, &attrs, secret, true)
            .await
            .map(|_| ())
            .map_err(|err| KeynearError::Keyring(err.to_string())),
    }
}

async fn delete_secret(backend: &Backend, attrs: &[(&str, &str)]) -> Result<(), KeynearError> {
    match backend {
        Backend::Host(keyring) => keyring
            .delete(&attrs)
            .await
            .map_err(|err| KeynearError::Keyring(err.to_string())),
        Backend::File(keyring) => keyring
            .delete(&attrs)
            .await
            .map_err(|err| KeynearError::Keyring(err.to_string())),
    }
}

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
                Backend::Host(keyring) => {
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
                Backend::File(keyring) => {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_config_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("keynear-oo7-{label}-{nanos}"))
    }

    #[test]
    fn falls_back_to_file_keyring_when_dbus_absent() {
        let dir = temp_config_dir("fallback");
        let store = Oo7SecretStore::open_file_only(&dir).expect("open file keyring");
        assert!(store.is_file_fallback());

        let kp1 = store
            .get_or_create_static_keypair()
            .expect("create keypair");
        let kp2 = store
            .get_or_create_static_keypair()
            .expect("reload keypair");
        assert_eq!(kp1.public_key, kp2.public_key);
        assert_eq!(kp1.private_key, kp2.private_key);

        let secrets = PeerSecrets {
            irk: [0x33; 32],
            reconnect_secret: [0x44; 32],
        };
        store
            .put_peer_secrets("pixel-9", &secrets)
            .expect("put peer secrets");
        assert_eq!(
            store.get_peer_secrets("pixel-9").expect("get peer secrets"),
            Some(secrets.clone())
        );
        assert_eq!(store.list_peer_ids().expect("list peers"), vec!["pixel-9"]);

        let c1 = store.next_presence_counter().expect("c1");
        let c2 = store.next_presence_counter().expect("c2");
        assert_eq!(c1, 1);
        assert_eq!(c2, 2);

        drop(store);

        let reopened = Oo7SecretStore::open_file_only(&dir).expect("reopen file keyring");
        assert_eq!(
            reopened.get_peer_secrets("pixel-9").expect("persisted"),
            Some(secrets)
        );
        assert_eq!(reopened.next_presence_counter().expect("c3"), 3);

        reopened
            .delete_peer_secrets("pixel-9")
            .expect("delete peer secrets");
        assert!(reopened.get_peer_secrets("pixel-9").unwrap().is_none());

        let _ = fs::remove_dir_all(&dir);
    }
}
