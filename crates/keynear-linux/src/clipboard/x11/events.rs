use std::sync::mpsc as std_mpsc;
use std::sync::Arc;

use keynear_core::{ClipboardPolicy, DelayedRenderCallback, KeynearError};
use tokio::sync::mpsc;
use x11rb::connection::Connection as X11Connection;
use x11rb::protocol::xproto::{self, ConnectionExt as XProtoConnectionExt};

use super::super::config::CLIPBOARD_READ_TIMEOUT;
use super::super::handle::BackendCmd;
use super::super::mime::{evaluate_offered_mimes, OfferAction};
use super::super::ClipboardChange;
use super::setup::{intern_atom, read_x11_property_bytes, read_x11_targets};
use super::state::{X11LoopContext, X11LoopState, X11SelectionStage};

fn make_x11_render_callback(
    seq: u64,
    cmd_tx: mpsc::UnboundedSender<BackendCmd>,
) -> DelayedRenderCallback {
    Arc::new(move |mime| {
        let (tx, rx) = std_mpsc::channel();
        cmd_tx
            .send(BackendCmd::RenderOffer {
                offer_seq: seq,
                mime_type: mime.to_string(),
                reply: tx,
            })
            .map_err(|_| KeynearError::Transport("X11 clipboard stopped".to_string()))?;
        rx.recv_timeout(CLIPBOARD_READ_TIMEOUT)
            .map_err(|_| KeynearError::Transport("timed out reading X11 image".to_string()))?
    })
}

fn handle_inspect_targets(
    ctx: &X11LoopContext<'_>,
    notify: &xproto::SelectionNotifyEvent,
    state: &mut X11LoopState,
) -> Result<(), KeynearError> {
    let mimes = read_x11_targets(ctx.conn, ctx.window, notify.property)?;
    let policy = {
        let cfg = ctx.config_rx.borrow();
        ClipboardPolicy::from(&*cfg)
    };
    match evaluate_offered_mimes(&mimes, &policy) {
        OfferAction::SkipIgnored | OfferAction::SkipUnsupported => {}
        OfferAction::OfferImage(image_mimes) => {
            let seq = state.active_offer_seq;
            let render = make_x11_render_callback(seq, ctx.cmd_tx.clone());
            let _ = ctx.event_tx.send(ClipboardChange::ImageOffer {
                offered_mimes: image_mimes,
                render,
            });
        }
        OfferAction::ReadText(text_mime) => {
            state.pending_text_mime = text_mime.clone();
            state.stage = X11SelectionStage::ReadText;
            let target_atom = intern_atom(ctx.conn, text_mime.as_bytes())?;
            let _ = ctx.conn.convert_selection(
                ctx.window,
                ctx.atom_clipboard,
                target_atom,
                ctx.atom_prop,
                notify.time,
            );
            let _ = ctx.conn.flush();
        }
    }
    Ok(())
}

fn handle_read_text(
    ctx: &X11LoopContext<'_>,
    notify: &xproto::SelectionNotifyEvent,
    state: &X11LoopState,
) -> Result<(), KeynearError> {
    let bytes = read_x11_property_bytes(ctx.conn, ctx.window, notify.property)?;
    let is_echo = {
        let mut tracker = ctx
            .self_writes
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        tracker.is_recent_echo(&bytes, std::time::Instant::now())
    };
    if !is_echo {
        if let Ok(text) = String::from_utf8(bytes) {
            if !text.is_empty() {
                let _ = ctx.event_tx.send(ClipboardChange::Text {
                    mime_type: state.pending_text_mime.clone(),
                    text,
                });
            }
        }
    }
    Ok(())
}

pub(super) fn handle_selection_notify(
    ctx: &X11LoopContext<'_>,
    notify: &xproto::SelectionNotifyEvent,
    state: &mut X11LoopState,
) -> Result<(), KeynearError> {
    match state.stage {
        X11SelectionStage::InspectTargets => handle_inspect_targets(ctx, notify, state),
        X11SelectionStage::ReadText => handle_read_text(ctx, notify, state),
        X11SelectionStage::FetchImage(seq) => {
            let bytes = read_x11_property_bytes(ctx.conn, ctx.window, notify.property);
            if let Some(reply) = state.pending_image_replies.remove(&seq) {
                let _ = reply.send(bytes);
            }
            Ok(())
        }
    }
}

pub(super) fn handle_x11_cmd(
    ctx: &X11LoopContext<'_>,
    cmd: BackendCmd,
    state: &mut X11LoopState,
) -> Result<bool, KeynearError> {
    match cmd {
        BackendCmd::WriteClipboard {
            mime_type,
            data,
            reply,
        } => {
            state.owned_clipboard = Some((mime_type, data));
            let res = ctx
                .conn
                .set_selection_owner(ctx.window, ctx.atom_clipboard, x11rb::CURRENT_TIME)
                .map_err(|err| KeynearError::Transport(err.to_string()))
                .and_then(|_| {
                    ctx.conn
                        .flush()
                        .map_err(|err| KeynearError::Transport(err.to_string()))
                });
            let _ = reply.send(res);
            Ok(true)
        }
        BackendCmd::RenderOffer {
            offer_seq,
            mime_type,
            reply,
        } => {
            if offer_seq != state.active_offer_seq {
                let _ = reply.send(Err(KeynearError::Transfer(
                    "clipboard selection changed before fetch".to_string(),
                )));
                return Ok(true);
            }
            let target_atom = intern_atom(ctx.conn, mime_type.as_bytes())?;
            state.pending_image_replies.insert(offer_seq, reply);
            state.stage = X11SelectionStage::FetchImage(offer_seq);
            let _ = ctx.conn.convert_selection(
                ctx.window,
                ctx.atom_clipboard,
                target_atom,
                ctx.atom_prop,
                x11rb::CURRENT_TIME,
            );
            let _ = ctx.conn.flush();
            Ok(true)
        }
        BackendCmd::Shutdown => Ok(false),
    }
}
