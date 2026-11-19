use std::collections::{HashMap, VecDeque};
use std::io::Read;
use std::os::fd::AsFd;
use std::sync::mpsc as std_mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use keynear_core::{
    is_image_mime, normalize_mime_type, should_ignore_mime_offers, ClipboardPolicy, DaemonConfig,
    DelayedRenderCallback, KeynearError, MAX_CLIPBOARD_IMAGE_BYTES,
};
use sha2::{Digest, Sha256};
use tokio::io::unix::AsyncFd;
use tokio::sync::{mpsc, oneshot, watch};
use tracing::{info, warn};
use wayland_client::backend::ObjectId;
use wayland_client::protocol::{wl_registry, wl_seat};
use wayland_client::{Connection, Dispatch, EventQueue, Proxy, QueueHandle};
use wayland_protocols::ext::data_control::v1::client::{
    ext_data_control_device_v1, ext_data_control_manager_v1, ext_data_control_offer_v1,
    ext_data_control_source_v1,
};
use wayland_protocols_wlr::data_control::v1::client::{
    zwlr_data_control_device_v1, zwlr_data_control_manager_v1, zwlr_data_control_offer_v1,
    zwlr_data_control_source_v1,
};
use x11rb::connection::Connection as X11Connection;
use x11rb::protocol::xfixes::{self, ConnectionExt as XFixesConnectionExt};
use x11rb::protocol::xproto::{self, AtomEnum, ConnectionExt as XProtoConnectionExt};
use x11rb::protocol::Event as X11Event;
use x11rb::rust_connection::RustConnection;

pub const CLIPBOARD_ECHO_WINDOW_SECS: u64 = 5;
const MAX_SELF_WRITE_HISTORY: usize = 16;
const CLIPBOARD_READ_TIMEOUT: Duration = Duration::from_secs(5);
const X11_POLL_INTERVAL: Duration = Duration::from_millis(25);
const X11_MAX_TARGET_ATOMS: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipboardBackendKind {
    ExtDataControlV1,
    ZwlrDataControlV1,
    X11,
    HeadlessUnavailable,
}

impl ClipboardBackendKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ExtDataControlV1 => "ext-data-control-v1",
            Self::ZwlrDataControlV1 => "zwlr-data-control-v1",
            Self::X11 => "x11",
            Self::HeadlessUnavailable => "headless-unavailable",
        }
    }
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ProbeCapabilities {
    pub has_ext_data_control: bool,
    pub has_zwlr_data_control: bool,
    pub has_x11: bool,
}

#[cfg(test)]
pub fn select_backend_from_caps(caps: ProbeCapabilities) -> ClipboardBackendKind {
    if caps.has_ext_data_control {
        ClipboardBackendKind::ExtDataControlV1
    } else if caps.has_zwlr_data_control {
        ClipboardBackendKind::ZwlrDataControlV1
    } else if caps.has_x11 {
        ClipboardBackendKind::X11
    } else {
        ClipboardBackendKind::HeadlessUnavailable
    }
}

#[derive(Clone)]
pub enum ClipboardChange {
    Text {
        mime_type: String,
        text: String,
    },
    ImageOffer {
        offered_mimes: Vec<String>,
        render: DelayedRenderCallback,
    },
}

#[derive(Debug, PartialEq, Eq)]
pub enum OfferAction {
    SkipIgnored,
    OfferImage(Vec<String>),
    ReadText(String),
    SkipUnsupported,
}

pub fn evaluate_offered_mimes(offered_mimes: &[String], policy: &ClipboardPolicy) -> OfferAction {
    if offered_mimes.is_empty() || !policy.clipboard_sync_enabled {
        return OfferAction::SkipIgnored;
    }
    if should_ignore_mime_offers(offered_mimes, &policy.ignored_mime_types) {
        return OfferAction::SkipIgnored;
    }

    let image_mimes: Vec<String> = offered_mimes
        .iter()
        .filter(|m| is_image_mime(m))
        .cloned()
        .collect();
    if !image_mimes.is_empty() {
        if policy.clipboard_images_enabled {
            return OfferAction::OfferImage(image_mimes);
        }
        return OfferAction::SkipUnsupported;
    }

    if let Some(text_mime) = select_preferred_text_mime(offered_mimes) {
        return OfferAction::ReadText(text_mime);
    }

    OfferAction::SkipUnsupported
}

fn select_preferred_text_mime(mimes: &[String]) -> Option<String> {
    const PREFERRED: [&str; 5] = [
        "text/plain;charset=utf-8",
        "utf8_string",
        "text/plain",
        "utf8-string",
        "string",
    ];
    for candidate in PREFERRED {
        if let Some(found) = mimes
            .iter()
            .find(|m| m.trim().eq_ignore_ascii_case(candidate))
        {
            return Some(found.clone());
        }
    }
    mimes
        .iter()
        .find(|m| normalize_mime_type(m).starts_with("text/"))
        .cloned()
}

#[derive(Debug, Default)]
pub struct SelfWriteTracker {
    entries: VecDeque<([u8; 32], Instant)>,
    last_image_write: Option<Instant>,
}

impl SelfWriteTracker {
    pub fn record_write(&mut self, data: &[u8], now: Instant) {
        self.prune(now);
        if self.entries.len() >= MAX_SELF_WRITE_HISTORY {
            self.entries.pop_front();
        }
        let digest: [u8; 32] = Sha256::digest(data).into();
        self.entries.push_back((digest, now));
    }

    pub fn record_image_write(&mut self, now: Instant) {
        self.prune(now);
        self.last_image_write = Some(now);
    }

    pub fn is_recent_image_write(&mut self, now: Instant) -> bool {
        self.prune(now);
        self.last_image_write.is_some()
    }

    pub fn is_recent_echo(&mut self, data: &[u8], now: Instant) -> bool {
        self.prune(now);
        let digest: [u8; 32] = Sha256::digest(data).into();
        self.entries.iter().any(|(d, _)| *d == digest)
    }

    fn prune(&mut self, now: Instant) {
        let window = Duration::from_secs(CLIPBOARD_ECHO_WINDOW_SECS);
        while let Some((_, ts)) = self.entries.front() {
            if now.saturating_duration_since(*ts) >= window {
                self.entries.pop_front();
            } else {
                break;
            }
        }
        if let Some(ts) = self.last_image_write {
            if now.saturating_duration_since(ts) >= window {
                self.last_image_write = None;
            }
        }
    }
}

enum BackendCmd {
    WriteClipboard {
        mime_type: String,
        data: Vec<u8>,
        reply: oneshot::Sender<Result<(), KeynearError>>,
    },
    RenderOffer {
        offer_seq: u64,
        mime_type: String,
        reply: std_mpsc::Sender<Result<Vec<u8>, KeynearError>>,
    },
    Shutdown,
}

#[derive(Clone)]
pub struct ClipboardMonitorHandle {
    backend: ClipboardBackendKind,
    cmd_tx: Option<mpsc::UnboundedSender<BackendCmd>>,
    self_writes: Arc<Mutex<SelfWriteTracker>>,
}

impl ClipboardMonitorHandle {
    pub fn headless() -> Self {
        Self {
            backend: ClipboardBackendKind::HeadlessUnavailable,
            cmd_tx: None,
            self_writes: Arc::new(Mutex::new(SelfWriteTracker::default())),
        }
    }

    pub fn backend(&self) -> ClipboardBackendKind {
        self.backend
    }

    #[cfg(test)]
    pub fn write_clipboard(&self, mime_type: &str, data: &[u8]) -> Result<(), KeynearError> {
        let clean_mime = mime_type.trim();
        if clean_mime.is_empty() {
            return Err(KeynearError::Validation {
                field: "mimeType",
                message: "clipboard MIME type must not be empty".to_string(),
            });
        }
        if data.len() > MAX_CLIPBOARD_IMAGE_BYTES {
            return Err(KeynearError::FrameTooLarge {
                size: data.len(),
                max: MAX_CLIPBOARD_IMAGE_BYTES,
            });
        }
        {
            let mut tracker = self
                .self_writes
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let now = Instant::now();
            tracker.record_write(data, now);
            if is_image_mime(clean_mime) {
                tracker.record_image_write(now);
            }
        }
        let Some(ref tx) = self.cmd_tx else {
            return Ok(());
        };
        let (reply_tx, reply_rx) = oneshot::channel();
        tx.send(BackendCmd::WriteClipboard {
            mime_type: clean_mime.to_string(),
            data: data.to_vec(),
            reply: reply_tx,
        })
        .map_err(|_| KeynearError::Transport("clipboard monitor stopped".to_string()))?;
        let _ = reply_rx.blocking_recv();
        Ok(())
    }

    pub async fn write_clipboard_async(
        &self,
        mime_type: &str,
        data: &[u8],
    ) -> Result<(), KeynearError> {
        let clean_mime = mime_type.trim();
        if clean_mime.is_empty() {
            return Err(KeynearError::Validation {
                field: "mimeType",
                message: "clipboard MIME type must not be empty".to_string(),
            });
        }
        if data.len() > MAX_CLIPBOARD_IMAGE_BYTES {
            return Err(KeynearError::FrameTooLarge {
                size: data.len(),
                max: MAX_CLIPBOARD_IMAGE_BYTES,
            });
        }
        {
            let mut tracker = self
                .self_writes
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let now = Instant::now();
            tracker.record_write(data, now);
            if is_image_mime(clean_mime) {
                tracker.record_image_write(now);
            }
        }
        let Some(ref tx) = self.cmd_tx else {
            return Ok(());
        };
        let (reply_tx, reply_rx) = oneshot::channel();
        tx.send(BackendCmd::WriteClipboard {
            mime_type: clean_mime.to_string(),
            data: data.to_vec(),
            reply: reply_tx,
        })
        .map_err(|_| KeynearError::Transport("clipboard monitor stopped".to_string()))?;
        reply_rx
            .await
            .map_err(|_| KeynearError::Transport("clipboard write reply dropped".to_string()))?
    }

    pub fn shutdown(&self) {
        if let Some(ref tx) = self.cmd_tx {
            let _ = tx.send(BackendCmd::Shutdown);
        }
    }
}

#[cfg(test)]
pub fn probe_clipboard_backend() -> ClipboardBackendKind {
    if let Ok(conn) = Connection::connect_to_env() {
        let mut queue = conn.new_event_queue();
        let qh = queue.handle();
        let mut probe = WaylandProbeState::default();
        let _registry = conn.display().get_registry(&qh, ());
        if queue.roundtrip(&mut probe).is_ok() && probe.has_seat {
            let kind = select_backend_from_caps(ProbeCapabilities {
                has_ext_data_control: probe.has_ext_manager,
                has_zwlr_data_control: probe.has_zwlr_manager,
                has_x11: false,
            });
            if kind != ClipboardBackendKind::HeadlessUnavailable {
                return kind;
            }
        }
    }

    if let Ok((x11_conn, _)) = RustConnection::connect(None) {
        if x11_conn
            .xfixes_query_version(5, 0)
            .ok()
            .and_then(|c| c.reply().ok())
            .is_some()
        {
            return ClipboardBackendKind::X11;
        }
    }

    ClipboardBackendKind::HeadlessUnavailable
}

pub fn start_clipboard_monitor(
    config_rx: watch::Receiver<DaemonConfig>,
    event_tx: mpsc::UnboundedSender<ClipboardChange>,
) -> ClipboardMonitorHandle {
    let self_writes = Arc::new(Mutex::new(SelfWriteTracker::default()));

    if let Ok(conn) = Connection::connect_to_env() {
        let mut queue: EventQueue<WaylandAppState> = conn.new_event_queue();
        let qh = queue.handle();
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        let mut state = WaylandAppState::new(
            config_rx.clone(),
            event_tx.clone(),
            Arc::clone(&self_writes),
            cmd_tx.clone(),
        );
        let _registry = conn.display().get_registry(&qh, ());
        if queue.roundtrip(&mut state).is_ok() {
            if let Some(kind) = state.bind_preferred_device(&qh) {
                if queue.roundtrip(&mut state).is_ok() {
                    info!(
                        event = "clipboard_backend_selected",
                        backend = kind.as_str()
                    );
                    std::thread::Builder::new()
                        .name("keynear-wayland-clipboard".to_string())
                        .spawn(move || {
                            run_wayland_loop(conn, queue, state, cmd_rx);
                        })
                        .ok();
                    return ClipboardMonitorHandle {
                        backend: kind,
                        cmd_tx: Some(cmd_tx),
                        self_writes,
                    };
                }
            }
        }
    }

    if let Ok((x11_conn, screen_num)) = RustConnection::connect(None) {
        if x11_conn
            .xfixes_query_version(5, 0)
            .ok()
            .and_then(|c| c.reply().ok())
            .is_some()
        {
            let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
            let x11_cmd_tx = cmd_tx.clone();
            let sw = Arc::clone(&self_writes);
            let cfg_rx = config_rx.clone();
            let ev_tx = event_tx.clone();
            info!(
                event = "clipboard_backend_selected",
                backend = ClipboardBackendKind::X11.as_str()
            );
            std::thread::Builder::new()
                .name("keynear-x11-clipboard".to_string())
                .spawn(move || {
                    if let Err(err) =
                        run_x11_loop(x11_conn, screen_num, cfg_rx, ev_tx, sw, x11_cmd_tx, cmd_rx)
                    {
                        warn!(event = "x11_clipboard_stopped", error = %err);
                    }
                })
                .ok();
            return ClipboardMonitorHandle {
                backend: ClipboardBackendKind::X11,
                cmd_tx: Some(cmd_tx),
                self_writes,
            };
        }
    }

    info!(
        event = "clipboard_backend_selected",
        backend = ClipboardBackendKind::HeadlessUnavailable.as_str()
    );
    ClipboardMonitorHandle {
        backend: ClipboardBackendKind::HeadlessUnavailable,
        cmd_tx: None,
        self_writes,
    }
}

#[cfg(test)]
#[derive(Default)]
struct WaylandProbeState {
    has_seat: bool,
    has_ext_manager: bool,
    has_zwlr_manager: bool,
}

#[cfg(test)]
impl Dispatch<wl_registry::WlRegistry, ()> for WaylandProbeState {
    fn event(
        state: &mut Self,
        _: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global { interface, .. } = event {
            match interface.as_str() {
                "wl_seat" => state.has_seat = true,
                "ext_data_control_manager_v1" => state.has_ext_manager = true,
                "zwlr_data_control_manager_v1" => state.has_zwlr_manager = true,
                _ => {}
            }
        }
    }
}

enum ActiveOfferHandle {
    Ext(ext_data_control_offer_v1::ExtDataControlOfferV1),
    Zwlr(zwlr_data_control_offer_v1::ZwlrDataControlOfferV1),
}

impl ActiveOfferHandle {
    fn receive(&self, mime: String, fd: std::os::fd::BorrowedFd<'_>) {
        match self {
            Self::Ext(o) => o.receive(mime, fd),
            Self::Zwlr(o) => o.receive(mime, fd),
        }
    }

    fn destroy(&self) {
        match self {
            Self::Ext(o) => o.destroy(),
            Self::Zwlr(o) => o.destroy(),
        }
    }
}

enum ActiveSourceHandle {
    Ext(ext_data_control_source_v1::ExtDataControlSourceV1),
    Zwlr(zwlr_data_control_source_v1::ZwlrDataControlSourceV1),
}

impl ActiveSourceHandle {
    fn destroy(&self) {
        match self {
            Self::Ext(s) => s.destroy(),
            Self::Zwlr(s) => s.destroy(),
        }
    }
}

struct WaylandAppState {
    seat: Option<wl_seat::WlSeat>,
    ext_manager: Option<ext_data_control_manager_v1::ExtDataControlManagerV1>,
    zwlr_manager: Option<zwlr_data_control_manager_v1::ZwlrDataControlManagerV1>,
    ext_device: Option<ext_data_control_device_v1::ExtDataControlDeviceV1>,
    zwlr_device: Option<zwlr_data_control_device_v1::ZwlrDataControlDeviceV1>,
    backend_kind: Option<ClipboardBackendKind>,
    pending_offers: HashMap<ObjectId, Vec<String>>,
    current_offer: Option<(u64, ActiveOfferHandle)>,
    next_offer_seq: u64,
    active_sources: HashMap<ObjectId, (ActiveSourceHandle, Arc<Vec<u8>>)>,
    config_rx: watch::Receiver<DaemonConfig>,
    event_tx: mpsc::UnboundedSender<ClipboardChange>,
    self_writes: Arc<Mutex<SelfWriteTracker>>,
    cmd_tx: mpsc::UnboundedSender<BackendCmd>,
}

impl WaylandAppState {
    fn new(
        config_rx: watch::Receiver<DaemonConfig>,
        event_tx: mpsc::UnboundedSender<ClipboardChange>,
        self_writes: Arc<Mutex<SelfWriteTracker>>,
        cmd_tx: mpsc::UnboundedSender<BackendCmd>,
    ) -> Self {
        Self {
            seat: None,
            ext_manager: None,
            zwlr_manager: None,
            ext_device: None,
            zwlr_device: None,
            backend_kind: None,
            pending_offers: HashMap::new(),
            current_offer: None,
            next_offer_seq: 1,
            active_sources: HashMap::new(),
            config_rx,
            event_tx,
            self_writes,
            cmd_tx,
        }
    }

    fn bind_preferred_device(&mut self, qh: &QueueHandle<Self>) -> Option<ClipboardBackendKind> {
        let seat = self.seat.as_ref()?;
        if let Some(ref mgr) = self.ext_manager {
            let dev = mgr.get_data_device(seat, qh, ());
            self.ext_device = Some(dev);
            self.backend_kind = Some(ClipboardBackendKind::ExtDataControlV1);
            return Some(ClipboardBackendKind::ExtDataControlV1);
        }
        if let Some(ref mgr) = self.zwlr_manager {
            let dev = mgr.get_data_device(seat, qh, ());
            self.zwlr_device = Some(dev);
            self.backend_kind = Some(ClipboardBackendKind::ZwlrDataControlV1);
            return Some(ClipboardBackendKind::ZwlrDataControlV1);
        }
        None
    }

    fn current_policy(&self) -> ClipboardPolicy {
        let cfg = self.config_rx.borrow();
        ClipboardPolicy::from(&*cfg)
    }

    fn on_data_offer(&mut self, offer_id: ObjectId) {
        self.pending_offers.insert(offer_id, Vec::new());
    }

    fn on_offer_mime(&mut self, offer_id: &ObjectId, mime_type: String) {
        if let Some(list) = self.pending_offers.get_mut(offer_id) {
            list.push(mime_type);
        }
    }

    fn on_selection(&mut self, conn: &Connection, offer: Option<(ObjectId, ActiveOfferHandle)>) {
        if let Some((_, old)) = self.current_offer.take() {
            old.destroy();
        }
        let Some((offer_id, handle)) = offer else {
            return;
        };
        let mimes = self.pending_offers.remove(&offer_id).unwrap_or_default();
        let policy = self.current_policy();

        match evaluate_offered_mimes(&mimes, &policy) {
            OfferAction::SkipIgnored | OfferAction::SkipUnsupported => {
                handle.destroy();
            }
            OfferAction::OfferImage(image_mimes) => {
                if !self.active_sources.is_empty() {
                    let is_self_image = {
                        let mut tracker = self
                            .self_writes
                            .lock()
                            .unwrap_or_else(|poisoned| poisoned.into_inner());
                        tracker.is_recent_image_write(Instant::now())
                    };
                    if is_self_image {
                        handle.destroy();
                        return;
                    }
                }
                let seq = self.next_offer_seq;
                self.next_offer_seq += 1;
                self.current_offer = Some((seq, handle));
                let cmd_tx = self.cmd_tx.clone();
                let render: DelayedRenderCallback = Arc::new(move |mime| {
                    let (tx, rx) = std_mpsc::channel();
                    cmd_tx
                        .send(BackendCmd::RenderOffer {
                            offer_seq: seq,
                            mime_type: mime.to_string(),
                            reply: tx,
                        })
                        .map_err(|_| {
                            KeynearError::Transport("clipboard backend closed".to_string())
                        })?;
                    rx.recv_timeout(CLIPBOARD_READ_TIMEOUT).map_err(|_| {
                        KeynearError::Transport("timed out rendering clipboard image".to_string())
                    })?
                });
                let _ = self.event_tx.send(ClipboardChange::ImageOffer {
                    offered_mimes: image_mimes,
                    render,
                });
            }
            OfferAction::ReadText(text_mime) => {
                if let Ok((read_pipe, write_pipe)) = std::io::pipe() {
                    handle.receive(text_mime.clone(), write_pipe.as_fd());
                    drop(write_pipe);
                    let _ = conn.flush();
                    self.current_offer = Some((0, handle));
                    let event_tx = self.event_tx.clone();
                    let self_writes = Arc::clone(&self.self_writes);
                    std::thread::spawn(move || {
                        if let Ok(bytes) = read_pipe_bounded(read_pipe) {
                            let is_echo = {
                                let mut tracker = self_writes
                                    .lock()
                                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                                tracker.is_recent_echo(&bytes, Instant::now())
                            };
                            if !is_echo {
                                if let Ok(text) = String::from_utf8(bytes) {
                                    if !text.is_empty() {
                                        let _ = event_tx.send(ClipboardChange::Text {
                                            mime_type: text_mime,
                                            text,
                                        });
                                    }
                                }
                            }
                        }
                    });
                } else {
                    handle.destroy();
                }
            }
        }
    }

    fn handle_render_offer(
        &self,
        conn: &Connection,
        offer_seq: u64,
        mime_type: String,
        reply: std_mpsc::Sender<Result<Vec<u8>, KeynearError>>,
    ) {
        let Some((active_seq, ref handle)) = self.current_offer else {
            let _ = reply.send(Err(KeynearError::Transfer(
                "no active clipboard offer".to_string(),
            )));
            return;
        };
        if active_seq != offer_seq {
            let _ = reply.send(Err(KeynearError::Transfer(
                "clipboard selection changed before fetch".to_string(),
            )));
            return;
        }
        let policy = self.current_policy();
        if should_ignore_mime_offers(std::slice::from_ref(&mime_type), &policy.ignored_mime_types) {
            let _ = reply.send(Err(KeynearError::Validation {
                field: "mimeType",
                message: "clipboard MIME type is ignored by policy".to_string(),
            }));
            return;
        }
        let Ok((read_pipe, write_pipe)) = std::io::pipe() else {
            let _ = reply.send(Err(KeynearError::Transport(
                "failed to create clipboard pipe".to_string(),
            )));
            return;
        };
        handle.receive(mime_type, write_pipe.as_fd());
        drop(write_pipe);
        let _ = conn.flush();
        std::thread::spawn(move || {
            let _ = reply.send(read_pipe_bounded(read_pipe));
        });
    }

    fn handle_write_clipboard(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        mime_type: String,
        data: Vec<u8>,
    ) -> Result<(), KeynearError> {
        let payload = Arc::new(data);
        let mut offered = vec![mime_type.clone()];
        if normalize_mime_type(&mime_type) == "text/plain" {
            for extra in ["text/plain;charset=utf-8", "UTF8_STRING", "TEXT", "STRING"] {
                if !offered.iter().any(|m| m == extra) {
                    offered.push(extra.to_string());
                }
            }
        }

        if let (Some(ref mgr), Some(ref dev)) = (&self.ext_manager, &self.ext_device) {
            let source = mgr.create_data_source(qh, ());
            for m in offered {
                source.offer(m);
            }
            dev.set_selection(Some(&source));
            self.active_sources
                .insert(source.id(), (ActiveSourceHandle::Ext(source), payload));
            let _ = conn.flush();
            return Ok(());
        }

        if let (Some(ref mgr), Some(ref dev)) = (&self.zwlr_manager, &self.zwlr_device) {
            let source = mgr.create_data_source(qh, ());
            for m in offered {
                source.offer(m);
            }
            dev.set_selection(Some(&source));
            self.active_sources
                .insert(source.id(), (ActiveSourceHandle::Zwlr(source), payload));
            let _ = conn.flush();
            return Ok(());
        }

        Err(KeynearError::Transport(
            "Wayland data-control device not bound".to_string(),
        ))
    }

    fn on_source_send(&self, source_id: &ObjectId, fd: std::os::fd::OwnedFd) {
        if let Some((_, payload)) = self.active_sources.get(source_id) {
            let data = Arc::clone(payload);
            std::thread::spawn(move || {
                use std::io::Write;
                let mut file = std::fs::File::from(fd);
                let _ = file.write_all(&data);
            });
        }
    }

    fn on_source_cancelled(&mut self, source_id: &ObjectId) {
        if let Some((handle, _)) = self.active_sources.remove(source_id) {
            handle.destroy();
        }
    }
}

impl Dispatch<wl_registry::WlRegistry, ()> for WaylandAppState {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            match interface.as_str() {
                "wl_seat" if state.seat.is_none() => {
                    state.seat = Some(registry.bind(name, version.min(7), qh, ()));
                }
                "ext_data_control_manager_v1" if state.ext_manager.is_none() => {
                    state.ext_manager = Some(registry.bind(name, version.min(1), qh, ()));
                }
                "zwlr_data_control_manager_v1" if state.zwlr_manager.is_none() => {
                    state.zwlr_manager = Some(registry.bind(name, version.min(2), qh, ()));
                }
                _ => {}
            }
        }
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for WaylandAppState {
    fn event(
        _: &mut Self,
        _: &wl_seat::WlSeat,
        _: wl_seat::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ext_data_control_manager_v1::ExtDataControlManagerV1, ()> for WaylandAppState {
    fn event(
        _: &mut Self,
        _: &ext_data_control_manager_v1::ExtDataControlManagerV1,
        _: ext_data_control_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ext_data_control_device_v1::ExtDataControlDeviceV1, ()> for WaylandAppState {
    fn event(
        state: &mut Self,
        _: &ext_data_control_device_v1::ExtDataControlDeviceV1,
        event: ext_data_control_device_v1::Event,
        _: &(),
        conn: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            ext_data_control_device_v1::Event::DataOffer { id } => {
                state.on_data_offer(id.id());
            }
            ext_data_control_device_v1::Event::Selection { id } => {
                state.on_selection(conn, id.map(|o| (o.id(), ActiveOfferHandle::Ext(o))));
            }
            ext_data_control_device_v1::Event::PrimarySelection { id: Some(offer) } => {
                state.pending_offers.remove(&offer.id());
                offer.destroy();
            }
            _ => {}
        }
    }

    wayland_client::event_created_child!(
        WaylandAppState,
        ext_data_control_device_v1::ExtDataControlDeviceV1,
        [
            ext_data_control_device_v1::EVT_DATA_OFFER_OPCODE => (ext_data_control_offer_v1::ExtDataControlOfferV1, ()),
        ]
    );
}

impl Dispatch<ext_data_control_offer_v1::ExtDataControlOfferV1, ()> for WaylandAppState {
    fn event(
        state: &mut Self,
        offer: &ext_data_control_offer_v1::ExtDataControlOfferV1,
        event: ext_data_control_offer_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let ext_data_control_offer_v1::Event::Offer { mime_type } = event {
            state.on_offer_mime(&offer.id(), mime_type);
        }
    }
}

impl Dispatch<ext_data_control_source_v1::ExtDataControlSourceV1, ()> for WaylandAppState {
    fn event(
        state: &mut Self,
        source: &ext_data_control_source_v1::ExtDataControlSourceV1,
        event: ext_data_control_source_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            ext_data_control_source_v1::Event::Send { fd, .. } => {
                state.on_source_send(&source.id(), fd);
            }
            ext_data_control_source_v1::Event::Cancelled => {
                state.on_source_cancelled(&source.id());
            }
            _ => {}
        }
    }
}

impl Dispatch<zwlr_data_control_manager_v1::ZwlrDataControlManagerV1, ()> for WaylandAppState {
    fn event(
        _: &mut Self,
        _: &zwlr_data_control_manager_v1::ZwlrDataControlManagerV1,
        _: zwlr_data_control_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<zwlr_data_control_device_v1::ZwlrDataControlDeviceV1, ()> for WaylandAppState {
    fn event(
        state: &mut Self,
        _: &zwlr_data_control_device_v1::ZwlrDataControlDeviceV1,
        event: zwlr_data_control_device_v1::Event,
        _: &(),
        conn: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_data_control_device_v1::Event::DataOffer { id } => {
                state.on_data_offer(id.id());
            }
            zwlr_data_control_device_v1::Event::Selection { id } => {
                state.on_selection(conn, id.map(|o| (o.id(), ActiveOfferHandle::Zwlr(o))));
            }
            zwlr_data_control_device_v1::Event::PrimarySelection { id: Some(offer) } => {
                state.pending_offers.remove(&offer.id());
                offer.destroy();
            }
            _ => {}
        }
    }

    wayland_client::event_created_child!(
        WaylandAppState,
        zwlr_data_control_device_v1::ZwlrDataControlDeviceV1,
        [
            zwlr_data_control_device_v1::EVT_DATA_OFFER_OPCODE => (zwlr_data_control_offer_v1::ZwlrDataControlOfferV1, ()),
        ]
    );
}

impl Dispatch<zwlr_data_control_offer_v1::ZwlrDataControlOfferV1, ()> for WaylandAppState {
    fn event(
        state: &mut Self,
        offer: &zwlr_data_control_offer_v1::ZwlrDataControlOfferV1,
        event: zwlr_data_control_offer_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let zwlr_data_control_offer_v1::Event::Offer { mime_type } = event {
            state.on_offer_mime(&offer.id(), mime_type);
        }
    }
}

impl Dispatch<zwlr_data_control_source_v1::ZwlrDataControlSourceV1, ()> for WaylandAppState {
    fn event(
        state: &mut Self,
        source: &zwlr_data_control_source_v1::ZwlrDataControlSourceV1,
        event: zwlr_data_control_source_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_data_control_source_v1::Event::Send { fd, .. } => {
                state.on_source_send(&source.id(), fd);
            }
            zwlr_data_control_source_v1::Event::Cancelled => {
                state.on_source_cancelled(&source.id());
            }
            _ => {}
        }
    }
}

struct WaylandConnFd(Connection);

impl std::os::fd::AsRawFd for WaylandConnFd {
    fn as_raw_fd(&self) -> std::os::fd::RawFd {
        self.0.as_fd().as_raw_fd()
    }
}

fn run_wayland_loop(
    conn: Connection,
    mut queue: EventQueue<WaylandAppState>,
    mut state: WaylandAppState,
    mut cmd_rx: mpsc::UnboundedReceiver<BackendCmd>,
) {
    let Ok(rt) = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .build()
    else {
        return;
    };
    let qh = queue.handle();
    rt.block_on(async move {
        let Ok(async_fd) = (unsafe {
            AsyncFd::register_with_interest(
                WaylandConnFd(conn.clone()),
                tokio::io::Interest::READABLE,
            )
        }) else {
            return;
        };
        loop {
            if queue.dispatch_pending(&mut state).is_err() {
                break;
            }
            let _ = queue.flush();
            tokio::select! {
                ready = async_fd.readable() => {
                    match ready {
                        Ok(mut guard) => {
                            if let Some(read_guard) = conn.prepare_read() {
                                let _ = read_guard.read();
                            }
                            if queue.dispatch_pending(&mut state).is_err() {
                                break;
                            }
                            guard.clear_ready();
                        }
                        Err(_) => break,
                    }
                }
                cmd = cmd_rx.recv() => {
                    match cmd {
                        Some(BackendCmd::WriteClipboard { mime_type, data, reply }) => {
                            let res = state.handle_write_clipboard(&conn, &qh, mime_type, data);
                            let _ = reply.send(res);
                        }
                        Some(BackendCmd::RenderOffer { offer_seq, mime_type, reply }) => {
                            state.handle_render_offer(&conn, offer_seq, mime_type, reply);
                        }
                        Some(BackendCmd::Shutdown) | None => break,
                    }
                }
            }
        }
    });
}

fn read_pipe_bounded(pipe: std::io::PipeReader) -> Result<Vec<u8>, KeynearError> {
    let mut limited = pipe.take((MAX_CLIPBOARD_IMAGE_BYTES + 1) as u64);
    let mut buf = Vec::new();
    limited.read_to_end(&mut buf)?;
    if buf.len() > MAX_CLIPBOARD_IMAGE_BYTES {
        return Err(KeynearError::FrameTooLarge {
            size: buf.len(),
            max: MAX_CLIPBOARD_IMAGE_BYTES,
        });
    }
    Ok(buf)
}

#[derive(Clone, Copy)]
enum X11SelectionStage {
    InspectTargets,
    ReadText,
    FetchImage(u64),
}

fn run_x11_loop(
    conn: RustConnection,
    screen_num: usize,
    config_rx: watch::Receiver<DaemonConfig>,
    event_tx: mpsc::UnboundedSender<ClipboardChange>,
    self_writes: Arc<Mutex<SelfWriteTracker>>,
    cmd_tx: mpsc::UnboundedSender<BackendCmd>,
    mut cmd_rx: mpsc::UnboundedReceiver<BackendCmd>,
) -> Result<(), KeynearError> {
    let screen = &conn
        .setup()
        .roots
        .get(screen_num)
        .ok_or_else(|| KeynearError::Transport("X11 screen missing".to_string()))?;
    let window = conn
        .generate_id()
        .map_err(|err| KeynearError::Transport(err.to_string()))?;
    conn.create_window(
        x11rb::COPY_DEPTH_FROM_PARENT,
        window,
        screen.root,
        0,
        0,
        1,
        1,
        0,
        xproto::WindowClass::INPUT_OUTPUT,
        0,
        &xproto::CreateWindowAux::new(),
    )
    .map_err(|err| KeynearError::Transport(err.to_string()))?;

    let atom_clipboard = intern_atom(&conn, b"CLIPBOARD")?;
    let atom_targets = intern_atom(&conn, b"TARGETS")?;
    let atom_utf8 = intern_atom(&conn, b"UTF8_STRING")?;
    let atom_prop = intern_atom(&conn, b"KEYNEAR_CLIPBOARD_DATA")?;

    conn.xfixes_select_selection_input(
        window,
        atom_clipboard,
        xfixes::SelectionEventMask::SET_SELECTION_OWNER,
    )
    .map_err(|err| KeynearError::Transport(err.to_string()))?;
    conn.flush()
        .map_err(|err| KeynearError::Transport(err.to_string()))?;

    let mut stage = X11SelectionStage::InspectTargets;
    let mut active_offer_seq: u64 = 0;
    let mut pending_text_mime = String::from("text/plain;charset=utf-8");
    let mut owned_clipboard: Option<(String, Vec<u8>)> = None;
    let mut pending_image_replies: HashMap<u64, std_mpsc::Sender<Result<Vec<u8>, KeynearError>>> =
        HashMap::new();

    loop {
        while let Some(ev) = conn
            .poll_for_event()
            .map_err(|err| KeynearError::Transport(err.to_string()))?
        {
            match ev {
                X11Event::XfixesSelectionNotify(notify) if notify.selection == atom_clipboard => {
                    if notify.owner == window || notify.owner == x11rb::NONE {
                        continue;
                    }
                    active_offer_seq += 1;
                    stage = X11SelectionStage::InspectTargets;
                    let _ = conn.convert_selection(
                        window,
                        atom_clipboard,
                        atom_targets,
                        atom_prop,
                        notify.timestamp,
                    );
                    let _ = conn.flush();
                }
                X11Event::SelectionNotify(notify)
                    if notify.selection == atom_clipboard
                        && notify.property != AtomEnum::NONE.into() =>
                {
                    match stage {
                        X11SelectionStage::InspectTargets => {
                            let mimes = read_x11_targets(&conn, window, notify.property)?;
                            let policy = {
                                let cfg = config_rx.borrow();
                                ClipboardPolicy::from(&*cfg)
                            };
                            match evaluate_offered_mimes(&mimes, &policy) {
                                OfferAction::SkipIgnored | OfferAction::SkipUnsupported => {}
                                OfferAction::OfferImage(image_mimes) => {
                                    let seq = active_offer_seq;
                                    let tx_clone = cmd_tx.clone();
                                    let render: DelayedRenderCallback = Arc::new(move |mime| {
                                        let (tx, rx) = std_mpsc::channel();
                                        tx_clone
                                            .send(BackendCmd::RenderOffer {
                                                offer_seq: seq,
                                                mime_type: mime.to_string(),
                                                reply: tx,
                                            })
                                            .map_err(|_| {
                                                KeynearError::Transport(
                                                    "X11 clipboard stopped".to_string(),
                                                )
                                            })?;
                                        rx.recv_timeout(CLIPBOARD_READ_TIMEOUT).map_err(|_| {
                                            KeynearError::Transport(
                                                "timed out reading X11 image".to_string(),
                                            )
                                        })?
                                    });
                                    let _ = event_tx.send(ClipboardChange::ImageOffer {
                                        offered_mimes: image_mimes,
                                        render,
                                    });
                                }
                                OfferAction::ReadText(text_mime) => {
                                    pending_text_mime = text_mime.clone();
                                    stage = X11SelectionStage::ReadText;
                                    let target_atom = intern_atom(&conn, text_mime.as_bytes())?;
                                    let _ = conn.convert_selection(
                                        window,
                                        atom_clipboard,
                                        target_atom,
                                        atom_prop,
                                        notify.time,
                                    );
                                    let _ = conn.flush();
                                }
                            }
                        }
                        X11SelectionStage::ReadText => {
                            let bytes = read_x11_property_bytes(&conn, window, notify.property)?;
                            let is_echo = {
                                let mut tracker = self_writes
                                    .lock()
                                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                                tracker.is_recent_echo(&bytes, Instant::now())
                            };
                            if !is_echo {
                                if let Ok(text) = String::from_utf8(bytes) {
                                    if !text.is_empty() {
                                        let _ = event_tx.send(ClipboardChange::Text {
                                            mime_type: pending_text_mime.clone(),
                                            text,
                                        });
                                    }
                                }
                            }
                        }
                        X11SelectionStage::FetchImage(seq) => {
                            let bytes = read_x11_property_bytes(&conn, window, notify.property);
                            if let Some(reply) = pending_image_replies.remove(&seq) {
                                let _ = reply.send(bytes);
                            }
                        }
                    }
                }
                X11Event::SelectionRequest(req) if req.selection == atom_clipboard => {
                    serve_x11_selection_request(
                        &conn,
                        &req,
                        atom_targets,
                        atom_utf8,
                        owned_clipboard.as_ref(),
                    )?;
                }
                X11Event::SelectionClear(clr) if clr.selection == atom_clipboard => {
                    owned_clipboard = None;
                }
                _ => {}
            }
        }

        while let Ok(cmd) = cmd_rx.try_recv() {
            match cmd {
                BackendCmd::WriteClipboard {
                    mime_type,
                    data,
                    reply,
                } => {
                    owned_clipboard = Some((mime_type, data));
                    let res = conn
                        .set_selection_owner(window, atom_clipboard, x11rb::CURRENT_TIME)
                        .map_err(|err| KeynearError::Transport(err.to_string()))
                        .and_then(|_| {
                            conn.flush()
                                .map_err(|err| KeynearError::Transport(err.to_string()))
                        });
                    let _ = reply.send(res);
                }
                BackendCmd::RenderOffer {
                    offer_seq,
                    mime_type,
                    reply,
                } => {
                    if offer_seq != active_offer_seq {
                        let _ = reply.send(Err(KeynearError::Transfer(
                            "clipboard selection changed before fetch".to_string(),
                        )));
                        continue;
                    }
                    let target_atom = intern_atom(&conn, mime_type.as_bytes())?;
                    pending_image_replies.insert(offer_seq, reply);
                    stage = X11SelectionStage::FetchImage(offer_seq);
                    let _ = conn.convert_selection(
                        window,
                        atom_clipboard,
                        target_atom,
                        atom_prop,
                        x11rb::CURRENT_TIME,
                    );
                    let _ = conn.flush();
                }
                BackendCmd::Shutdown => return Ok(()),
            }
        }

        std::thread::sleep(X11_POLL_INTERVAL);
    }
}

fn intern_atom(conn: &RustConnection, name: &[u8]) -> Result<u32, KeynearError> {
    Ok(conn
        .intern_atom(false, name)
        .map_err(|err| KeynearError::Transport(err.to_string()))?
        .reply()
        .map_err(|err| KeynearError::Transport(err.to_string()))?
        .atom)
}

fn read_x11_targets(
    conn: &RustConnection,
    window: u32,
    property: u32,
) -> Result<Vec<String>, KeynearError> {
    let reply = conn
        .get_property(
            true,
            window,
            property,
            AtomEnum::ATOM,
            0,
            X11_MAX_TARGET_ATOMS as u32,
        )
        .map_err(|err| KeynearError::Transport(err.to_string()))?
        .reply()
        .map_err(|err| KeynearError::Transport(err.to_string()))?;

    let mut out = Vec::new();
    if let Some(atoms) = reply.value32() {
        for atom in atoms.take(X11_MAX_TARGET_ATOMS) {
            if let Ok(cookie) = conn.get_atom_name(atom) {
                if let Ok(name_reply) = cookie.reply() {
                    if let Ok(name) = String::from_utf8(name_reply.name) {
                        out.push(name);
                    }
                }
            }
        }
    }
    Ok(out)
}

fn read_x11_property_bytes(
    conn: &RustConnection,
    window: u32,
    property: u32,
) -> Result<Vec<u8>, KeynearError> {
    let max_words = (MAX_CLIPBOARD_IMAGE_BYTES / 4) as u32;
    let reply = conn
        .get_property(true, window, property, AtomEnum::ANY, 0, max_words)
        .map_err(|err| KeynearError::Transport(err.to_string()))?
        .reply()
        .map_err(|err| KeynearError::Transport(err.to_string()))?;
    if reply.bytes_after > 0 {
        return Err(KeynearError::FrameTooLarge {
            size: reply.value.len() + reply.bytes_after as usize,
            max: MAX_CLIPBOARD_IMAGE_BYTES,
        });
    }
    Ok(reply.value)
}

fn serve_x11_selection_request(
    conn: &RustConnection,
    req: &xproto::SelectionRequestEvent,
    atom_targets: u32,
    atom_utf8: u32,
    owned: Option<&(String, Vec<u8>)>,
) -> Result<(), KeynearError> {
    let prop = if req.property == AtomEnum::NONE.into() {
        req.target
    } else {
        req.property
    };
    let mut granted = false;

    if let Some((mime, data)) = owned {
        if req.target == atom_targets {
            let mime_atom = intern_atom(conn, mime.as_bytes())?;
            let targets = [atom_targets, mime_atom, atom_utf8];
            let mut bytes = Vec::with_capacity(targets.len() * 4);
            for atom in targets {
                bytes.extend_from_slice(&atom.to_ne_bytes());
            }
            let _ = conn.change_property(
                xproto::PropMode::REPLACE,
                req.requestor,
                prop,
                AtomEnum::ATOM,
                32,
                targets.len() as u32,
                &bytes,
            );
            granted = true;
        } else {
            let mime_atom = intern_atom(conn, mime.as_bytes())?;
            if req.target == mime_atom
                || (normalize_mime_type(mime) == "text/plain" && req.target == atom_utf8)
            {
                let _ = conn.change_property(
                    xproto::PropMode::REPLACE,
                    req.requestor,
                    prop,
                    req.target,
                    8,
                    data.len() as u32,
                    data,
                );
                granted = true;
            }
        }
    }

    let notify = xproto::SelectionNotifyEvent {
        response_type: xproto::SELECTION_NOTIFY_EVENT,
        sequence: 0,
        time: req.time,
        requestor: req.requestor,
        selection: req.selection,
        target: req.target,
        property: if granted { prop } else { AtomEnum::NONE.into() },
    };
    let _ = conn.send_event(false, req.requestor, xproto::EventMask::NO_EVENT, notify);
    let _ = conn.flush();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_fallback_and_headless_degrade_non_fatally() {
        assert_eq!(
            select_backend_from_caps(ProbeCapabilities {
                has_ext_data_control: true,
                has_zwlr_data_control: true,
                has_x11: true,
            }),
            ClipboardBackendKind::ExtDataControlV1
        );
        assert_eq!(
            select_backend_from_caps(ProbeCapabilities {
                has_ext_data_control: false,
                has_zwlr_data_control: true,
                has_x11: true,
            }),
            ClipboardBackendKind::ZwlrDataControlV1
        );
        assert_eq!(
            select_backend_from_caps(ProbeCapabilities {
                has_ext_data_control: false,
                has_zwlr_data_control: false,
                has_x11: true,
            }),
            ClipboardBackendKind::X11
        );
        assert_eq!(
            select_backend_from_caps(ProbeCapabilities::default()),
            ClipboardBackendKind::HeadlessUnavailable
        );

        let _ = probe_clipboard_backend();
        let headless = ClipboardMonitorHandle::headless();
        assert_eq!(
            headless.backend(),
            ClipboardBackendKind::HeadlessUnavailable
        );
        headless
            .write_clipboard("text/plain", b"safe in headless")
            .expect("headless write succeeds non-fatally");
    }

    #[test]
    fn pre_read_mime_filter_skips_before_read() {
        let mut policy = ClipboardPolicy::default();

        assert_eq!(
            evaluate_offered_mimes(
                &[
                    "text/plain;charset=utf-8".to_string(),
                    "x-kde-passwordManagerHint".to_string(),
                ],
                &policy,
            ),
            OfferAction::SkipIgnored
        );
        assert_eq!(
            evaluate_offered_mimes(&["application/x-password".to_string()], &policy),
            OfferAction::SkipIgnored
        );

        policy
            .ignored_mime_types
            .push("application/x-custom-secret".to_string());
        assert_eq!(
            evaluate_offered_mimes(
                &[
                    "text/plain".to_string(),
                    "Application/X-Custom-Secret; version=1".to_string(),
                ],
                &policy,
            ),
            OfferAction::SkipIgnored
        );

        assert_eq!(
            evaluate_offered_mimes(
                &["text/plain".to_string(), "image/png".to_string()],
                &policy,
            ),
            OfferAction::OfferImage(vec!["image/png".to_string()])
        );

        policy.clipboard_images_enabled = false;
        assert_eq!(
            evaluate_offered_mimes(&["image/png".to_string()], &policy),
            OfferAction::SkipUnsupported
        );

        assert_eq!(
            evaluate_offered_mimes(&["text/plain;charset=utf-8".to_string()], &policy),
            OfferAction::ReadText("text/plain;charset=utf-8".to_string())
        );
    }

    #[test]
    fn self_write_echo_suppression_drops_recent_digest() {
        let mut tracker = SelfWriteTracker::default();
        let t0 = Instant::now();
        tracker.record_write(b"847291", t0);

        assert!(tracker.is_recent_echo(b"847291", t0 + Duration::from_secs(1)));
        assert!(!tracker.is_recent_echo(b"different-copy", t0 + Duration::from_secs(1)));
        assert!(!tracker.is_recent_echo(
            b"847291",
            t0 + Duration::from_secs(CLIPBOARD_ECHO_WINDOW_SECS + 1)
        ));
    }
}
