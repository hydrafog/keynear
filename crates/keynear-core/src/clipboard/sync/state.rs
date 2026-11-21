use super::*;

pub struct ClipboardEngine {
    pub(super) policy: RwLock<ClipboardPolicy>,
    pub(super) outgoing_offers: Mutex<HashMap<String, OutgoingOfferEntry>>,
    pub(super) incoming_offers: Mutex<HashMap<String, IncomingOfferEntry>>,
    pub(super) incoming_queue: Mutex<VecDeque<ClipboardItem>>,
    pub(super) incoming_tx: Mutex<Option<mpsc::UnboundedSender<ClipboardItem>>>,
    pub(super) sequence: AtomicU64,
}

impl ClipboardEngine {
    pub fn new(policy: ClipboardPolicy) -> Self {
        Self {
            policy: RwLock::new(policy),
            outgoing_offers: Mutex::new(HashMap::new()),
            incoming_offers: Mutex::new(HashMap::new()),
            incoming_queue: Mutex::new(VecDeque::with_capacity(MAX_CLIPBOARD_QUEUE)),
            incoming_tx: Mutex::new(None),
            sequence: AtomicU64::new(1),
        }
    }

    pub fn with_incoming_sender(
        policy: ClipboardPolicy,
        tx: mpsc::UnboundedSender<ClipboardItem>,
    ) -> Self {
        let engine = Self::new(policy);
        if let Ok(mut guard) = engine.incoming_tx.lock() {
            *guard = Some(tx);
        }
        engine
    }

    pub fn set_policy(&self, policy: ClipboardPolicy) {
        let mut guard = self.policy.write().unwrap_or_else(|p| p.into_inner());
        *guard = policy;
    }

    pub fn policy(&self) -> ClipboardPolicy {
        self.policy
            .read()
            .unwrap_or_else(|p| p.into_inner())
            .clone()
    }

    pub fn poll_incoming(&self) -> Vec<ClipboardItem> {
        let mut guard = self
            .incoming_queue
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        guard.drain(..).collect()
    }

    pub fn can_accept_mimes(&self, mimes: &[String], is_image: bool, is_otp: bool) -> bool {
        let policy = self.policy();
        check_policy(&policy, mimes, is_image, is_otp).is_ok()
    }
}
