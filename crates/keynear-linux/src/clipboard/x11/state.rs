use std::collections::HashMap;
use std::sync::mpsc as std_mpsc;
use std::sync::{Arc, Mutex};

use keynear_core::{DaemonConfig, KeynearError};
use tokio::sync::{mpsc, watch};
use x11rb::rust_connection::RustConnection;

use super::super::handle::BackendCmd;
use super::super::tracker::SelfWriteTracker;
use super::super::ClipboardChange;

#[derive(Clone, Copy)]
pub(super) enum X11SelectionStage {
    InspectTargets,
    ReadText,
    FetchImage(u64),
}

pub(super) struct X11LoopState {
    pub(super) stage: X11SelectionStage,
    pub(super) active_offer_seq: u64,
    pub(super) pending_text_mime: String,
    pub(super) owned_clipboard: Option<(String, Vec<u8>)>,
    pub(super) pending_image_replies: HashMap<u64, std_mpsc::Sender<Result<Vec<u8>, KeynearError>>>,
}

pub(crate) struct X11LoopParams {
    pub conn: RustConnection,
    pub screen_num: usize,
    pub config_rx: watch::Receiver<DaemonConfig>,
    pub event_tx: mpsc::UnboundedSender<ClipboardChange>,
    pub self_writes: Arc<Mutex<SelfWriteTracker>>,
    pub cmd_tx: mpsc::UnboundedSender<BackendCmd>,
    pub cmd_rx: mpsc::UnboundedReceiver<BackendCmd>,
}

pub(super) struct X11LoopContext<'a> {
    pub(super) conn: &'a RustConnection,
    pub(super) window: u32,
    pub(super) atom_clipboard: u32,
    pub(super) atom_targets: u32,
    pub(super) atom_utf8: u32,
    pub(super) atom_prop: u32,
    pub(super) config_rx: &'a watch::Receiver<DaemonConfig>,
    pub(super) event_tx: &'a mpsc::UnboundedSender<ClipboardChange>,
    pub(super) self_writes: &'a Arc<Mutex<SelfWriteTracker>>,
    pub(super) cmd_tx: &'a mpsc::UnboundedSender<BackendCmd>,
}
