use keynear_core::KeynearError;
use std::collections::HashMap;
use x11rb::connection::Connection as X11Connection;
use x11rb::protocol::xproto::AtomEnum;
use x11rb::protocol::xproto::ConnectionExt as XProtoConnectionExt;
use x11rb::protocol::Event as X11Event;

use super::super::config::X11_POLL_INTERVAL;
use super::events::{handle_selection_notify, handle_x11_cmd};
use super::serve::{serve_x11_selection_request, SelectionRequestContext};
use super::setup::setup_x11_window;
use super::state::{X11LoopContext, X11LoopParams, X11LoopState, X11SelectionStage};

fn dispatch_x11_event(
    ev: X11Event,
    ctx: &X11LoopContext<'_>,
    state: &mut X11LoopState,
    window: u32,
) -> Result<(), KeynearError> {
    let conn = ctx.conn;
    let atom_clipboard = ctx.atom_clipboard;
    match ev {
        X11Event::XfixesSelectionNotify(notify) if notify.selection == atom_clipboard => {
            if notify.owner == window || notify.owner == x11rb::NONE {
                return Ok(());
            }
            state.active_offer_seq += 1;
            state.stage = X11SelectionStage::InspectTargets;
            let _ = conn.convert_selection(
                window,
                atom_clipboard,
                ctx.atom_targets,
                ctx.atom_prop,
                notify.timestamp,
            );
            let _ = conn.flush();
        }
        X11Event::SelectionNotify(notify)
            if notify.selection == atom_clipboard && notify.property != AtomEnum::NONE.into() =>
        {
            handle_selection_notify(ctx, &notify, state)?;
        }
        X11Event::SelectionRequest(req) if req.selection == atom_clipboard => {
            serve_x11_selection_request(
                &SelectionRequestContext {
                    conn,
                    atom_targets: ctx.atom_targets,
                    atom_utf8: ctx.atom_utf8,
                },
                &req,
                state.owned_clipboard.as_ref(),
            )?;
        }
        X11Event::SelectionClear(clr) if clr.selection == atom_clipboard => {
            state.owned_clipboard = None;
        }
        _ => {}
    }
    Ok(())
}

pub(crate) fn run_x11_loop(params: X11LoopParams) -> Result<(), KeynearError> {
    let X11LoopParams {
        conn,
        screen_num,
        config_rx,
        event_tx,
        self_writes,
        cmd_tx,
        mut cmd_rx,
    } = params;
    let (window, atom_clipboard, atom_targets, atom_utf8, atom_prop) =
        setup_x11_window(&conn, screen_num)?;

    let ctx = X11LoopContext {
        conn: &conn,
        window,
        atom_clipboard,
        atom_targets,
        atom_utf8,
        atom_prop,
        config_rx: &config_rx,
        event_tx: &event_tx,
        self_writes: &self_writes,
        cmd_tx: &cmd_tx,
    };
    let mut state = X11LoopState {
        stage: X11SelectionStage::InspectTargets,
        active_offer_seq: 0,
        pending_text_mime: String::from("text/plain;charset=utf-8"),
        owned_clipboard: None,
        pending_image_replies: HashMap::new(),
    };

    loop {
        while let Some(ev) = conn
            .poll_for_event()
            .map_err(|err| KeynearError::Transport(err.to_string()))?
        {
            dispatch_x11_event(ev, &ctx, &mut state, window)?;
        }

        while let Ok(cmd) = cmd_rx.try_recv() {
            if !handle_x11_cmd(&ctx, cmd, &mut state)? {
                return Ok(());
            }
        }

        std::thread::sleep(X11_POLL_INTERVAL);
    }
}
