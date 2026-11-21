use super::*;

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
            rollback_peer_secrets(secrets, &clean_id, previous_secrets);
            Err(err)
        }
    }
}

fn rollback_peer_secrets(
    secrets: &dyn SecretStore,
    clean_id: &str,
    previous_secrets: Option<PeerSecrets>,
) {
    let rollback = match previous_secrets {
        Some(ref prev) => secrets.put_peer_secrets(clean_id, prev),
        None => secrets.delete_peer_secrets(clean_id),
    };
    if let Err(rb_err) = rollback {
        tracing::error!(event = "pairing_commit_rollback_failed", error = %rb_err);
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
