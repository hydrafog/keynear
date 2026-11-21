use super::*;

pub const MAX_ACTIVE_TRANSFERS: usize = 32;
pub(super) const MAX_HISTORY_TRANSFERS: usize = 128;

#[derive(Debug, Clone)]
pub struct PeerTransferPolicy {
    pub auto_accept_files: bool,
}

pub(super) struct TransferEntry {
    pub(super) session: TransferSession,
    pub(super) consent_tx: Option<oneshot::Sender<bool>>,
    pub(super) fd_tx: Option<oneshot::Sender<File>>,
    pub(super) cancel_flag: Arc<AtomicBool>,
}

pub struct InboundRegistration {
    pub state: TransferState,
    pub consent_rx: Option<oneshot::Receiver<bool>>,
    pub fd_rx: Option<oneshot::Receiver<File>>,
    pub cancel_flag: Arc<AtomicBool>,
}

#[derive(Default)]
pub struct TransferRegistry {
    pub(super) entries: Mutex<HashMap<String, TransferEntry>>,
}
