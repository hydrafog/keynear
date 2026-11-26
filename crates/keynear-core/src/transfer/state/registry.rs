use super::types::TransferEntry;
use super::util::{ensure_capacity, now_timestamp_string};
use super::*;

impl TransferRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn list_sessions(&self) -> Vec<TransferSession> {
        let guard = self.entries.lock().unwrap_or_else(|p| p.into_inner());
        let mut list: Vec<TransferSession> = guard.values().map(|e| e.session.clone()).collect();
        list.sort_by(|a, b| {
            b.started_at
                .cmp(&a.started_at)
                .then_with(|| a.transfer_id.cmp(&b.transfer_id))
        });
        list
    }

    pub fn get_session(&self, transfer_id: &str) -> Option<TransferSession> {
        let guard = self.entries.lock().unwrap_or_else(|p| p.into_inner());
        guard.get(transfer_id).map(|e| e.session.clone())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn register_outbound(
        &self,
        transfer_id: String,
        peer_device_id: String,
        items: Vec<TransferItem>,
        total_bytes: u64,
    ) -> Result<Arc<AtomicBool>, KeynearError> {
        let clean_peer = sanitize_device_id(&peer_device_id)?;
        let mut guard = self.entries.lock().unwrap_or_else(|p| p.into_inner());
        ensure_capacity(&mut guard)?;
        let cancel_flag = Arc::new(AtomicBool::new(false));
        let session = TransferSession {
            transfer_id: transfer_id.clone(),
            peer_device_id: clean_peer,
            direction: TransferDirection::Send,
            items,
            total_bytes,
            transferred_bytes: 0,
            state: TransferState::Active,
            started_at: now_timestamp_string(),
        };
        guard.insert(
            transfer_id,
            TransferEntry {
                session,
                consent_tx: None,
                fd_tx: None,
                cancel_flag: Arc::clone(&cancel_flag),
            },
        );
        Ok(cancel_flag)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn register_inbound(
        &self,
        peer_device_id: String,
        offer: &v1::TransferOffer,
        auto_accept: bool,
        require_explicit_fd: bool,
    ) -> Result<InboundRegistration, KeynearError> {
        let clean_peer = sanitize_device_id(&peer_device_id)?;
        validate_offer(offer)?;

        let items = build_inbound_items(offer);

        let mut guard = self.entries.lock().unwrap_or_else(|p| p.into_inner());
        ensure_capacity(&mut guard)?;
        if guard.contains_key(&offer.transfer_id) {
            return Err(KeynearError::Transfer(format!(
                "duplicate transfer_id '{}'",
                offer.transfer_id
            )));
        }

        let cancel_flag = Arc::new(AtomicBool::new(false));
        let (consent_tx, consent_rx) = consent_channels(auto_accept);
        let (fd_tx, fd_rx) = fd_channels(require_explicit_fd);

        let initial_state = if auto_accept {
            TransferState::Active
        } else {
            TransferState::PendingConsent
        };

        let session = TransferSession {
            transfer_id: offer.transfer_id.clone(),
            peer_device_id: clean_peer,
            direction: TransferDirection::Receive,
            items,
            total_bytes: offer.total_bytes,
            transferred_bytes: 0,
            state: initial_state,
            started_at: now_timestamp_string(),
        };

        guard.insert(
            offer.transfer_id.clone(),
            TransferEntry {
                session,
                consent_tx,
                fd_tx,
                cancel_flag: Arc::clone(&cancel_flag),
            },
        );

        Ok(InboundRegistration {
            state: initial_state,
            consent_rx,
            fd_rx,
            cancel_flag,
        })
    }

    pub fn respond_consent(
        &self,
        transfer_id: &str,
        accepted: bool,
    ) -> Result<TransferSession, KeynearError> {
        let mut guard = self.entries.lock().unwrap_or_else(|p| p.into_inner());
        let entry = guard
            .get_mut(transfer_id)
            .ok_or_else(|| KeynearError::Transfer(format!("transfer '{transfer_id}' not found")))?;
        if entry.session.state != TransferState::PendingConsent {
            return Err(KeynearError::Transfer(format!(
                "transfer '{transfer_id}' is not awaiting consent"
            )));
        }
        if accepted {
            entry.session.state = TransferState::Active;
        } else {
            entry.session.state = TransferState::Cancelled;
            entry.cancel_flag.store(true, Ordering::Release);
            entry.fd_tx.take();
        }
        if let Some(tx) = entry.consent_tx.take() {
            let _ = tx.send(accepted);
        }
        Ok(entry.session.clone())
    }

    pub fn provide_receive_fd(&self, transfer_id: &str, file: File) -> Result<(), KeynearError> {
        let mut guard = self.entries.lock().unwrap_or_else(|p| p.into_inner());
        let entry = guard
            .get_mut(transfer_id)
            .ok_or_else(|| KeynearError::Transfer(format!("transfer '{transfer_id}' not found")))?;
        if matches!(
            entry.session.state,
            TransferState::Cancelled | TransferState::Failed | TransferState::Completed
        ) {
            return Err(KeynearError::TransferCancelled(format!(
                "transfer '{transfer_id}' is already terminated"
            )));
        }
        let tx = entry.fd_tx.take().ok_or_else(|| {
            KeynearError::Transfer(format!(
                "transfer '{transfer_id}' is not awaiting an output file descriptor"
            ))
        })?;
        tx.send(file).map_err(|_| {
            KeynearError::Transfer(format!(
                "transfer '{transfer_id}' receiver dropped before accepting fd"
            ))
        })
    }

    pub fn cancel_transfer(&self, transfer_id: &str) -> Result<TransferSession, KeynearError> {
        let mut guard = self.entries.lock().unwrap_or_else(|p| p.into_inner());
        let entry = guard
            .get_mut(transfer_id)
            .ok_or_else(|| KeynearError::Transfer(format!("transfer '{transfer_id}' not found")))?;
        entry.cancel_flag.store(true, Ordering::Release);
        if matches!(
            entry.session.state,
            TransferState::PendingConsent | TransferState::Active
        ) {
            entry.session.state = TransferState::Cancelled;
        }
        if let Some(tx) = entry.consent_tx.take() {
            let _ = tx.send(false);
        }
        entry.fd_tx.take();
        Ok(entry.session.clone())
    }

    pub fn update_progress(&self, transfer_id: &str, transferred_bytes: u64) {
        let mut guard = self.entries.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(entry) = guard.get_mut(transfer_id) {
            entry.session.transferred_bytes = transferred_bytes;
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn finish_transfer(
        &self,
        transfer_id: &str,
        state: TransferState,
        transferred_bytes: u64,
        target_paths: Option<&[String]>,
        checksum_hex: Option<&str>,
    ) {
        let mut guard = self.entries.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(entry) = guard.get_mut(transfer_id) {
            entry.session.state = state;
            entry.session.transferred_bytes = transferred_bytes;
            entry.consent_tx.take();
            entry.fd_tx.take();
            if let Some(paths) = target_paths {
                for (item, path) in entry.session.items.iter_mut().zip(paths.iter()) {
                    item.target_path = Some(path.clone());
                }
            }
            if let Some(digest) = checksum_hex {
                for item in &mut entry.session.items {
                    item.checksum = Some(digest.to_string());
                }
            }
        }
    }
}

fn build_inbound_items(offer: &v1::TransferOffer) -> Vec<TransferItem> {
    offer
        .items
        .iter()
        .map(|meta| TransferItem {
            name: meta.name.clone(),
            mime_type: meta.mime_type.clone(),
            size_bytes: meta.size_bytes,
            is_directory: meta.is_directory,
            target_path: None,
            checksum: None,
        })
        .collect()
}

fn consent_channels(
    auto_accept: bool,
) -> (
    Option<oneshot::Sender<bool>>,
    Option<oneshot::Receiver<bool>>,
) {
    if auto_accept {
        (None, None)
    } else {
        let (tx, rx) = oneshot::channel();
        (Some(tx), Some(rx))
    }
}

fn fd_channels(
    require_explicit_fd: bool,
) -> (
    Option<oneshot::Sender<File>>,
    Option<oneshot::Receiver<File>>,
) {
    if require_explicit_fd {
        let (tx, rx) = oneshot::channel();
        (Some(tx), Some(rx))
    } else {
        (None, None)
    }
}

#[cfg(test)]
#[path = "registry/tests.rs"]
mod tests;
