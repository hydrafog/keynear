use std::os::fd::AsFd;

use tokio::io::unix::AsyncFd;
use tokio::sync::mpsc;
use wayland_client::{Connection, EventQueue, QueueHandle};

use super::super::handle::BackendCmd;
use super::selection::{RenderOfferParams, WriteClipboardParams};
use super::state::WaylandAppState;

struct WaylandConnFd(Connection);

impl std::os::fd::AsRawFd for WaylandConnFd {
    fn as_raw_fd(&self) -> std::os::fd::RawFd {
        self.0.as_fd().as_raw_fd()
    }
}

fn dispatch_backend_cmd(
    conn: &Connection,
    qh: &QueueHandle<WaylandAppState>,
    state: &mut WaylandAppState,
    cmd: Option<BackendCmd>,
) -> bool {
    match cmd {
        Some(BackendCmd::WriteClipboard {
            mime_type,
            data,
            reply,
        }) => {
            let res = state.handle_write_clipboard(WriteClipboardParams {
                conn,
                qh,
                mime_type,
                data,
            });
            let _ = reply.send(res);
            true
        }
        Some(BackendCmd::RenderOffer {
            offer_seq,
            mime_type,
            reply,
        }) => {
            state.handle_render_offer(RenderOfferParams {
                conn,
                offer_seq,
                mime_type,
                reply,
            });
            true
        }
        Some(BackendCmd::Shutdown) | None => false,
    }
}

pub(crate) fn run_wayland_loop(
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
                    if !dispatch_backend_cmd(&conn, &qh, &mut state, cmd) {
                        break;
                    }
                }
            }
        }
    });
}
