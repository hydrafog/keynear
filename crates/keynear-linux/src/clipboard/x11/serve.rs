use keynear_core::KeynearError;
use x11rb::connection::Connection as X11Connection;
use x11rb::protocol::xproto::{self, AtomEnum, ConnectionExt as XProtoConnectionExt};
use x11rb::rust_connection::RustConnection;

use super::setup::intern_atom;

pub(super) struct SelectionRequestContext<'a> {
    pub(super) conn: &'a RustConnection,
    pub(super) atom_targets: u32,
    pub(super) atom_utf8: u32,
}

fn grant_selection(
    ctx: &SelectionRequestContext<'_>,
    req: &xproto::SelectionRequestEvent,
    owned: Option<&(String, Vec<u8>)>,
    prop: u32,
) -> Result<bool, KeynearError> {
    let conn = ctx.conn;
    let atom_targets = ctx.atom_targets;
    let atom_utf8 = ctx.atom_utf8;
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
                || (keynear_core::normalize_mime_type(mime) == "text/plain"
                    && req.target == atom_utf8)
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

    Ok(granted)
}

pub(super) fn serve_x11_selection_request(
    ctx: &SelectionRequestContext<'_>,
    req: &xproto::SelectionRequestEvent,
    owned: Option<&(String, Vec<u8>)>,
) -> Result<(), KeynearError> {
    let prop = if req.property == AtomEnum::NONE.into() {
        req.target
    } else {
        req.property
    };
    let granted = grant_selection(ctx, req, owned, prop)?;
    let notify = xproto::SelectionNotifyEvent {
        response_type: xproto::SELECTION_NOTIFY_EVENT,
        sequence: 0,
        time: req.time,
        requestor: req.requestor,
        selection: req.selection,
        target: req.target,
        property: if granted { prop } else { AtomEnum::NONE.into() },
    };
    let _ = ctx
        .conn
        .send_event(false, req.requestor, xproto::EventMask::NO_EVENT, notify);
    let _ = ctx.conn.flush();
    Ok(())
}
