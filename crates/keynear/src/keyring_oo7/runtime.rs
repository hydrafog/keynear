use std::fs::{self, OpenOptions};
use std::future::Future;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::mpsc;

use oo7::file::UnlockedKeyring;
use oo7::{Keyring, Secret};
use rand::Rng;
use tokio::runtime::{Builder, Runtime};
use tracing::warn;
use zeroize::Zeroizing;

use keynear_protocol::error::KeynearError;

use super::Backend;

pub(crate) fn build_runtime() -> Result<Runtime, KeynearError> {
    Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .map_err(|err| KeynearError::Keyring(format!("failed to create keyring runtime: {err}")))
}

pub(crate) fn run_on_runtime<F, T>(runtime: &Runtime, fut: F) -> Result<T, KeynearError>
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

pub(crate) async fn open_host_or_file_keyring(config_dir: &Path) -> Result<Backend, KeynearError> {
    match Keyring::host().await {
        Ok(keyring) => Ok(Backend::Host(std::sync::Arc::new(keyring))),
        Err(host_err) => fallback_file_keyring(config_dir, host_err).await,
    }
}

pub(crate) async fn open_file_keyring(config_dir: &Path) -> Result<UnlockedKeyring, KeynearError> {
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

async fn fallback_file_keyring(
    config_dir: &Path,
    host_err: oo7::Error,
) -> Result<Backend, KeynearError> {
    warn!(
        event = "secret_service_unavailable_using_file_fallback",
        error = %host_err
    );
    let file_keyring = open_file_keyring(config_dir).await?;
    Ok(Backend::File(std::sync::Arc::new(file_keyring)))
}
