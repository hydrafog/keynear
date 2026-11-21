use std::os::fd::AsFd;
use std::sync::mpsc as std_mpsc;
use std::sync::Arc;
use std::time::Instant;

use keynear_core::{
    normalize_mime_type, should_ignore_mime_offers, DelayedRenderCallback, KeynearError,
};
use wayland_client::backend::ObjectId;
use wayland_client::{Connection, Proxy, QueueHandle};

use super::super::backend::read_pipe_bounded;
use super::super::config::CLIPBOARD_READ_TIMEOUT;
use super::super::handle::BackendCmd;
use super::super::mime::{evaluate_offered_mimes, OfferAction};
use super::super::ClipboardChange;
use super::handles::{ActiveOfferHandle, ActiveSourceHandle};
use super::state::WaylandAppState;

impl WaylandAppState {
    fn is_self_image_write(&self) -> bool {
        if self.active_sources.is_empty() {
            return false;
        }
        let mut tracker = self
            .self_writes
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        tracker.is_recent_image_write(Instant::now())
    }

    fn on_image_offer(&mut self, handle: ActiveOfferHandle, image_mimes: Vec<String>) {
        if self.is_self_image_write() {
            handle.destroy();
            return;
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
                .map_err(|_| KeynearError::Transport("clipboard backend closed".to_string()))?;
            rx.recv_timeout(CLIPBOARD_READ_TIMEOUT).map_err(|_| {
                KeynearError::Transport("timed out rendering clipboard image".to_string())
            })?
        });
        let _ = self.event_tx.send(ClipboardChange::ImageOffer {
            offered_mimes: image_mimes,
            render,
        });
    }

    fn on_text_offer(&mut self, conn: &Connection, handle: ActiveOfferHandle, text_mime: String) {
        let Ok((read_pipe, write_pipe)) = std::io::pipe() else {
            handle.destroy();
            return;
        };
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
    }

    pub(super) fn on_selection(
        &mut self,
        conn: &Connection,
        offer: Option<(ObjectId, ActiveOfferHandle)>,
    ) {
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
            OfferAction::OfferImage(image_mimes) => self.on_image_offer(handle, image_mimes),
            OfferAction::ReadText(text_mime) => self.on_text_offer(conn, handle, text_mime),
        }
    }

    pub(crate) fn handle_render_offer(&self, params: RenderOfferParams) {
        let RenderOfferParams {
            conn,
            offer_seq,
            mime_type,
            reply,
        } = params;
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

    pub(crate) fn handle_write_clipboard(
        &mut self,
        params: WriteClipboardParams,
    ) -> Result<(), KeynearError> {
        let WriteClipboardParams {
            conn,
            qh,
            mime_type,
            data,
        } = params;
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

    pub(super) fn on_source_send(&self, source_id: &ObjectId, fd: std::os::fd::OwnedFd) {
        if let Some((_, payload)) = self.active_sources.get(source_id) {
            let data = Arc::clone(payload);
            std::thread::spawn(move || {
                use std::io::Write;
                let mut file = std::fs::File::from(fd);
                let _ = file.write_all(&data);
            });
        }
    }

    pub(super) fn on_source_cancelled(&mut self, source_id: &ObjectId) {
        if let Some((handle, _)) = self.active_sources.remove(source_id) {
            handle.destroy();
        }
    }
}

pub(crate) struct RenderOfferParams<'a> {
    pub conn: &'a Connection,
    pub offer_seq: u64,
    pub mime_type: String,
    pub reply: std_mpsc::Sender<Result<Vec<u8>, KeynearError>>,
}

pub(crate) struct WriteClipboardParams<'a> {
    pub conn: &'a Connection,
    pub qh: &'a QueueHandle<super::state::WaylandAppState>,
    pub mime_type: String,
    pub data: Vec<u8>,
}
