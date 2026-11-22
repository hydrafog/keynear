use oo7::Secret;
use zeroize::Zeroizing;

use keynear_protocol::error::KeynearError;

use super::Backend;

pub(crate) async fn get_secret_bytes(
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

pub(crate) async fn put_secret_bytes(
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

pub(crate) async fn delete_secret(
    backend: &Backend,
    attrs: &[(&str, &str)],
) -> Result<(), KeynearError> {
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
