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

#[cfg(test)]
#[allow(clippy::cognitive_complexity)]
mod tests {
    use super::*;

    fn engine() -> TransferEngine {
        TransferEngine::new(PathBuf::from("/tmp/keynear-incoming"), false)
    }

    fn offer(transfer_id: &str) -> v1::TransferOffer {
        v1::TransferOffer {
            transfer_id: transfer_id.to_string(),
            items: vec![v1::TransferItemMeta {
                name: "a.txt".to_string(),
                mime_type: "text/plain".to_string(),
                size_bytes: 4,
                is_directory: false,
            }],
            total_bytes: 4,
        }
    }

    #[test]
    fn incoming_directory_rejects_empty_and_expands_tilde() {
        let engine = engine();
        assert_eq!(
            engine.incoming_directory(),
            PathBuf::from("/tmp/keynear-incoming")
        );
        let err = engine
            .set_incoming_directory(PathBuf::new())
            .expect_err("empty incoming directory");
        assert!(
            matches!(err, KeynearError::Validation { field, .. } if field == "incomingDirectory")
        );

        engine
            .set_incoming_directory(PathBuf::from("~/Downloads"))
            .expect("set tilde directory");
        let expected = match std::env::var("HOME") {
            Ok(home) => PathBuf::from(home).join("Downloads"),
            Err(_) => PathBuf::from("~/Downloads"),
        };
        assert_eq!(engine.incoming_directory(), expected);
    }

    #[test]
    fn explicit_fd_flag_toggles() {
        let engine = engine();
        assert!(!engine
            .require_explicit_fd_for_single_file
            .load(Ordering::Acquire));
        engine.set_require_explicit_fd_for_single_file(true);
        assert!(engine
            .require_explicit_fd_for_single_file
            .load(Ordering::Acquire));
        engine.set_require_explicit_fd_for_single_file(false);
        assert!(!engine
            .require_explicit_fd_for_single_file
            .load(Ordering::Acquire));
    }

    #[test]
    fn auto_accept_policy_combines_global_flag_and_peer_overrides() {
        let engine = engine();
        assert!(!engine.is_auto_accept_allowed("phone-1"));
        engine
            .set_peer_auto_accept("phone-1", true)
            .expect("set peer policy");
        assert!(engine.is_auto_accept_allowed("phone-1"));
        engine.remove_peer_policy("phone-1");
        assert!(!engine.is_auto_accept_allowed("phone-1"));
        engine.set_auto_accept_from_paired(true);
        assert!(engine.is_auto_accept_allowed("phone-1"));
        assert!(engine.is_auto_accept_allowed("../bad"));
        assert!(engine.set_peer_auto_accept("../bad", true).is_err());
    }

    #[test]
    fn registry_accessor_and_list_transfers_delegate_to_the_registry() {
        let engine = engine();
        assert!(engine.registry().list_sessions().is_empty());
        engine
            .registry()
            .register_outbound("tx-1".to_string(), "phone-1".to_string(), Vec::new(), 0)
            .expect("register outbound");
        let list = engine.list_transfers();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].transfer_id, "tx-1");
    }

    #[test]
    fn consent_cancel_and_fd_delegation_reject_unknown_or_terminal_transfers() {
        let engine = engine();
        assert!(matches!(
            engine.respond_consent("missing", true),
            Err(KeynearError::Transfer(message)) if message.contains("not found")
        ));
        assert!(matches!(
            engine.cancel_transfer("missing"),
            Err(KeynearError::Transfer(message)) if message.contains("not found")
        ));
        let path = std::env::temp_dir().join("keynear-manager-fd-test.txt");
        let err = engine
            .provide_receive_fd("missing", File::create(&path).expect("create temp file"))
            .expect_err("missing fd target");
        assert!(matches!(err, KeynearError::Transfer(message) if message.contains("not found")));
        let _ = std::fs::remove_file(&path);

        engine
            .registry()
            .register_inbound("phone-1".to_string(), &offer("tx-2"), false, false)
            .expect("register inbound");
        let session = engine.respond_consent("tx-2", true).expect("accept");
        assert_eq!(session.state, TransferState::Active);
        let cancelled = engine.cancel_transfer("tx-2").expect("cancel");
        assert_eq!(cancelled.state, TransferState::Cancelled);
    }
}
