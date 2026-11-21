use std::sync::{Arc, Mutex};

use keynear_core::DaemonConfig;
use tokio::sync::{mpsc, watch};
use tracing::{info, warn};
#[cfg(test)]
use wayland_client::protocol::wl_registry;
use wayland_client::Connection;
#[cfg(test)]
use wayland_client::{Dispatch, QueueHandle};
use x11rb::protocol::xfixes::ConnectionExt as XFixesConnectionExt;
use x11rb::rust_connection::RustConnection;

use super::tracker::SelfWriteTracker;
use super::wayland::{run_wayland_loop, WaylandAppState};
use super::x11::{run_x11_loop, X11LoopParams};
use super::{ClipboardBackendKind, ClipboardChange, ClipboardMonitorHandle};

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

fn try_wayland_backend(
    config_rx: &watch::Receiver<DaemonConfig>,
    event_tx: &mpsc::UnboundedSender<ClipboardChange>,
    self_writes: &Arc<Mutex<SelfWriteTracker>>,
) -> Option<ClipboardMonitorHandle> {
    let Ok(conn) = Connection::connect_to_env() else {
        return None;
    };
    let mut queue: wayland_client::EventQueue<WaylandAppState> = conn.new_event_queue();
    let qh = queue.handle();
    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
    let mut state = WaylandAppState::new(
        config_rx.clone(),
        event_tx.clone(),
        Arc::clone(self_writes),
        cmd_tx.clone(),
    );
    let _registry = conn.display().get_registry(&qh, ());
    if queue.roundtrip(&mut state).is_err() {
        return None;
    }
    let kind = state.bind_preferred_device(&qh)?;
    if queue.roundtrip(&mut state).is_err() {
        return None;
    }
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
    Some(ClipboardMonitorHandle {
        backend: kind,
        cmd_tx: Some(cmd_tx),
        self_writes: Arc::clone(self_writes),
    })
}

fn try_x11_backend(
    config_rx: &watch::Receiver<DaemonConfig>,
    event_tx: &mpsc::UnboundedSender<ClipboardChange>,
    self_writes: &Arc<Mutex<SelfWriteTracker>>,
) -> Option<ClipboardMonitorHandle> {
    let Ok((x11_conn, screen_num)) = RustConnection::connect(None) else {
        return None;
    };
    x11_conn.xfixes_query_version(5, 0).ok()?.reply().ok()?;
    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
    let x11_cmd_tx = cmd_tx.clone();
    let sw = Arc::clone(self_writes);
    let cfg_rx = config_rx.clone();
    let ev_tx = event_tx.clone();
    info!(
        event = "clipboard_backend_selected",
        backend = ClipboardBackendKind::X11.as_str()
    );
    std::thread::Builder::new()
        .name("keynear-x11-clipboard".to_string())
        .spawn(move || {
            if let Err(err) = run_x11_loop(X11LoopParams {
                conn: x11_conn,
                screen_num,
                config_rx: cfg_rx,
                event_tx: ev_tx,
                self_writes: sw,
                cmd_tx: x11_cmd_tx,
                cmd_rx,
            }) {
                warn!(event = "x11_clipboard_stopped", error = %err);
            }
        })
        .ok();
    Some(ClipboardMonitorHandle {
        backend: ClipboardBackendKind::X11,
        cmd_tx: Some(cmd_tx),
        self_writes: Arc::clone(self_writes),
    })
}

pub fn start_clipboard_monitor(
    config_rx: watch::Receiver<DaemonConfig>,
    event_tx: mpsc::UnboundedSender<ClipboardChange>,
) -> ClipboardMonitorHandle {
    let self_writes = Arc::new(Mutex::new(SelfWriteTracker::default()));

    if let Some(handle) = try_wayland_backend(&config_rx, &event_tx, &self_writes) {
        return handle;
    }
    if let Some(handle) = try_x11_backend(&config_rx, &event_tx, &self_writes) {
        return handle;
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

pub(crate) fn read_pipe_bounded(
    pipe: std::io::PipeReader,
) -> Result<Vec<u8>, keynear_core::KeynearError> {
    use std::io::Read;

    let mut limited = pipe.take((keynear_core::MAX_CLIPBOARD_IMAGE_BYTES + 1) as u64);
    let mut buf = Vec::new();
    limited.read_to_end(&mut buf)?;
    if buf.len() > keynear_core::MAX_CLIPBOARD_IMAGE_BYTES {
        return Err(keynear_core::KeynearError::FrameTooLarge {
            size: buf.len(),
            max: keynear_core::MAX_CLIPBOARD_IMAGE_BYTES,
        });
    }
    Ok(buf)
}
