use super::*;

pub struct TransferEngine {
    pub(crate) registry: Arc<TransferRegistry>,
    pub(crate) incoming_directory: Mutex<PathBuf>,
    pub(crate) auto_accept_from_paired: AtomicBool,
    pub(crate) peer_policies: Mutex<HashMap<String, PeerTransferPolicy>>,
    pub(crate) require_explicit_fd_for_single_file: AtomicBool,
}

impl TransferEngine {
    pub fn new(incoming_directory: PathBuf, auto_accept_from_paired: bool) -> Self {
        Self {
            registry: Arc::new(TransferRegistry::new()),
            incoming_directory: Mutex::new(expand_tilde(&incoming_directory)),
            auto_accept_from_paired: AtomicBool::new(auto_accept_from_paired),
            peer_policies: Mutex::new(HashMap::new()),
            require_explicit_fd_for_single_file: AtomicBool::new(false),
        }
    }

    pub fn set_require_explicit_fd_for_single_file(&self, required: bool) {
        self.require_explicit_fd_for_single_file
            .store(required, Ordering::Release);
    }

    pub fn set_incoming_directory(&self, dir: PathBuf) -> Result<(), KeynearError> {
        if dir.as_os_str().is_empty() {
            return Err(KeynearError::Validation {
                field: "incomingDirectory",
                message: "incoming directory must not be empty".to_string(),
            });
        }
        let mut guard = self
            .incoming_directory
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        *guard = expand_tilde(&dir);
        Ok(())
    }

    pub fn incoming_directory(&self) -> PathBuf {
        self.incoming_directory
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clone()
    }

    pub fn set_auto_accept_from_paired(&self, enabled: bool) {
        self.auto_accept_from_paired
            .store(enabled, Ordering::Release);
    }

    pub fn set_peer_auto_accept(&self, device_id: &str, enabled: bool) -> Result<(), KeynearError> {
        let clean_id = sanitize_device_id(device_id)?;
        let mut guard = self.peer_policies.lock().unwrap_or_else(|p| p.into_inner());
        guard.insert(
            clean_id,
            PeerTransferPolicy {
                auto_accept_files: enabled,
            },
        );
        Ok(())
    }

    pub fn remove_peer_policy(&self, device_id: &str) {
        if let Ok(clean_id) = sanitize_device_id(device_id) {
            let mut guard = self.peer_policies.lock().unwrap_or_else(|p| p.into_inner());
            guard.remove(&clean_id);
        }
    }

    pub fn is_auto_accept_allowed(&self, device_id: &str) -> bool {
        if self.auto_accept_from_paired.load(Ordering::Acquire) {
            return true;
        }
        let Ok(clean_id) = sanitize_device_id(device_id) else {
            return false;
        };
        let guard = self.peer_policies.lock().unwrap_or_else(|p| p.into_inner());
        guard
            .get(&clean_id)
            .map(|p| p.auto_accept_files)
            .unwrap_or(false)
    }

    pub fn registry(&self) -> &Arc<TransferRegistry> {
        &self.registry
    }

    pub fn list_transfers(&self) -> Vec<TransferSession> {
        self.registry.list_sessions()
    }

    pub fn respond_consent(
        &self,
        transfer_id: &str,
        accepted: bool,
    ) -> Result<TransferSession, KeynearError> {
        self.registry.respond_consent(transfer_id, accepted)
    }

    pub fn cancel_transfer(&self, transfer_id: &str) -> Result<TransferSession, KeynearError> {
        self.registry.cancel_transfer(transfer_id)
    }

    pub fn provide_receive_fd(&self, transfer_id: &str, file: File) -> Result<(), KeynearError> {
        self.registry.provide_receive_fd(transfer_id, file)
    }
}
