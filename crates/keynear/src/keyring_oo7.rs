mod backend;
mod runtime;
mod store;

#[cfg(test)]
mod tests;

use std::future::Future;
use std::path::Path;
use std::sync::Arc;

use oo7::file::UnlockedKeyring;
use oo7::Keyring;
use tokio::runtime::Runtime;

use keynear_protocol::error::KeynearError;
use keynear_protocol::model::default_config_dir;

pub(crate) const APP_ATTR_KEY: &str = "app";
pub(crate) const APP_ATTR_VAL: &str = "keynear";
pub(crate) const KIND_ATTR_KEY: &str = "kind";
pub(crate) const DEVICE_ATTR_KEY: &str = "device_id";

pub(crate) const KIND_STATIC_KEYPAIR: &str = "static_x25519";
pub(crate) const KIND_QUIC_SECRET_KEY: &str = "quic_ed25519";
pub(crate) const KIND_PEER_SECRETS: &str = "peer_secrets";
pub(crate) const KIND_PRESENCE_COUNTER: &str = "presence_counter";

#[derive(Clone)]
pub(crate) enum Backend {
    Host(Arc<Keyring>),
    File(Arc<UnlockedKeyring>),
}

pub struct Oo7SecretStore {
    runtime: Option<Runtime>,
    backend: Backend,
    is_file_fallback: bool,
}

impl Drop for Oo7SecretStore {
    fn drop(&mut self) {
        if let Some(rt) = self.runtime.take() {
            rt.shutdown_background();
        }
    }
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
        let runtime = runtime::build_runtime()?;
        let config_dir = config_dir.to_path_buf();
        let backend = runtime::run_on_runtime(&runtime, async move {
            runtime::open_host_or_file_keyring(&config_dir).await
        })?;
        let is_file_fallback = matches!(backend, Backend::File(_));
        Ok(Self {
            runtime: Some(runtime),
            backend,
            is_file_fallback,
        })
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn open_file_only(config_dir: &Path) -> Result<Self, KeynearError> {
        let runtime = runtime::build_runtime()?;
        let config_dir = config_dir.to_path_buf();
        let file_keyring = runtime::run_on_runtime(&runtime, async move {
            runtime::open_file_keyring(&config_dir).await
        })?;
        Ok(Self {
            runtime: Some(runtime),
            backend: Backend::File(Arc::new(file_keyring)),
            is_file_fallback: true,
        })
    }

    pub fn is_file_fallback(&self) -> bool {
        self.is_file_fallback
    }

    fn runtime(&self) -> &Runtime {
        self.runtime.as_ref().expect("keyring runtime active")
    }

    fn exec<F, T>(&self, fut: F) -> Result<T, KeynearError>
    where
        F: Future<Output = Result<T, KeynearError>> + Send + 'static,
        T: Send + 'static,
    {
        runtime::run_on_runtime(self.runtime(), fut)
    }
}
