use crate::config::*;
use crate::*;

use super::enums::{decode_device_role, decode_visibility, encode_device_role, encode_visibility};
use super::primitives::{write_str, write_u32, Cursor};

pub(crate) fn write_paired_device(
    out: &mut Vec<u8>,
    device: &PairedDevice,
) -> Result<(), KeynearError> {
    write_str(out, &device.device_id)?;
    write_str(out, &device.display_name)?;
    out.push(encode_device_role(device.device_role));
    write_str(out, &device.static_x25519_pubkey)?;
    write_str(out, &device.last_ble_address)?;
    write_str(out, &device.last_lan_address)?;
    write_str(out, &device.last_endpoint_id)?;
    write_u32(out, device.capability_flags);
    out.push(u8::from(device.auto_accept_files));
    write_str(out, &device.paired_at)?;
    Ok(())
}

pub(crate) fn read_paired_device(cur: &mut Cursor<'_>) -> Result<PairedDevice, KeynearError> {
    let device_id = sanitize_device_id(&cur.read_string()?)?;
    let display_name = validate_display_name(&cur.read_string()?)?;
    let device_role = decode_device_role(cur.read_u8()?)?;
    let static_x25519_pubkey = cur.read_string()?;
    let last_ble_address = cur.read_string()?;
    let last_lan_address = cur.read_string()?;
    let last_endpoint_id = cur.read_string()?;
    let capability_flags = cur.read_u32()?;
    let auto_accept_files = cur.read_bool()?;
    let paired_at = cur.read_string()?;
    Ok(PairedDevice {
        device_id,
        display_name,
        device_role,
        static_x25519_pubkey,
        last_ble_address,
        last_lan_address,
        last_endpoint_id,
        capability_flags,
        auto_accept_files,
        paired_at,
    })
}

pub(crate) fn write_daemon_config(
    out: &mut Vec<u8>,
    config: &DaemonConfig,
) -> Result<(), KeynearError> {
    if config.ignored_mime_types.len() > MAX_IPC_MIMES {
        return Err(KeynearError::FrameTooLarge {
            size: config.ignored_mime_types.len(),
            max: MAX_IPC_MIMES,
        });
    }
    out.push(encode_visibility(config.visibility_mode));
    write_str(out, &config.incoming_directory)?;
    out.push(u8::from(config.auto_accept_from_paired));
    out.push(u8::from(config.clipboard_sync_enabled));
    out.push(u8::from(config.clipboard_images_enabled));
    out.push(u8::from(config.otp_passthrough_enabled));
    out.push(u8::from(config.audio_handoff_enabled));
    write_u32(out, config.ignored_mime_types.len() as u32);
    for mime in &config.ignored_mime_types {
        write_str(out, mime)?;
    }
    out.push(u8::from(config.notify_auto_accept_complete));
    Ok(())
}

pub(crate) fn read_daemon_config(cur: &mut Cursor<'_>) -> Result<DaemonConfig, KeynearError> {
    let visibility_mode = decode_visibility(cur.read_u8()?)?;
    let incoming_directory = cur.read_string()?;
    let auto_accept_from_paired = cur.read_bool()?;
    let clipboard_sync_enabled = cur.read_bool()?;
    let clipboard_images_enabled = cur.read_bool()?;
    let otp_passthrough_enabled = cur.read_bool()?;
    let audio_handoff_enabled = cur.read_bool()?;
    let mime_count = cur.read_u32()? as usize;
    if mime_count > MAX_IPC_MIMES {
        return Err(KeynearError::FrameTooLarge {
            size: mime_count,
            max: MAX_IPC_MIMES,
        });
    }
    let mut ignored_mime_types = Vec::with_capacity(mime_count);
    for _ in 0..mime_count {
        ignored_mime_types.push(cur.read_string()?);
    }
    let notify_auto_accept_complete = cur.read_bool()?;
    let config = DaemonConfig {
        visibility_mode,
        incoming_directory,
        auto_accept_from_paired,
        clipboard_sync_enabled,
        clipboard_images_enabled,
        otp_passthrough_enabled,
        audio_handoff_enabled,
        ignored_mime_types,
        notify_auto_accept_complete,
    };
    config.validate()?;
    Ok(config)
}
