use keynear_core::KeynearError;
use x11rb::connection::Connection as X11Connection;
use x11rb::protocol::xfixes::ConnectionExt as XFixesConnectionExt;
use x11rb::protocol::xproto::{self, AtomEnum, ConnectionExt as XProtoConnectionExt};
use x11rb::rust_connection::RustConnection;

use super::super::config::X11_MAX_TARGET_ATOMS;
use keynear_core::MAX_CLIPBOARD_IMAGE_BYTES;

pub(super) fn setup_x11_window(
    conn: &RustConnection,
    screen_num: usize,
) -> Result<(u32, u32, u32, u32, u32), KeynearError> {
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

    let atom_clipboard = intern_atom(conn, b"CLIPBOARD")?;
    let atom_targets = intern_atom(conn, b"TARGETS")?;
    let atom_utf8 = intern_atom(conn, b"UTF8_STRING")?;
    let atom_prop = intern_atom(conn, b"KEYNEAR_CLIPBOARD_DATA")?;

    conn.xfixes_select_selection_input(
        window,
        atom_clipboard,
        x11rb::protocol::xfixes::SelectionEventMask::SET_SELECTION_OWNER,
    )
    .map_err(|err| KeynearError::Transport(err.to_string()))?;
    conn.flush()
        .map_err(|err| KeynearError::Transport(err.to_string()))?;
    Ok((window, atom_clipboard, atom_targets, atom_utf8, atom_prop))
}

pub(super) fn intern_atom(conn: &RustConnection, name: &[u8]) -> Result<u32, KeynearError> {
    Ok(conn
        .intern_atom(false, name)
        .map_err(|err| KeynearError::Transport(err.to_string()))?
        .reply()
        .map_err(|err| KeynearError::Transport(err.to_string()))?
        .atom)
}

pub(super) fn read_x11_targets(
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

pub(super) fn read_x11_property_bytes(
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
