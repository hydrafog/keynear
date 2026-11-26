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
    out.push(u8::from(config.audio_continuity_enabled));
    out.push(u8::from(config.proximity_lock_enabled));
    out.push(u8::from(config.presence_gating_enabled));
    write_u32(out, config.proximity_distance_meters.to_bits());
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
    let audio_continuity_enabled = cur.read_bool()?;
    let proximity_lock_enabled = cur.read_bool()?;
    let presence_gating_enabled = cur.read_bool()?;
    let proximity_distance_meters = f32::from_bits(cur.read_u32()?);
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
        audio_continuity_enabled,
        proximity_lock_enabled,
        presence_gating_enabled,
        proximity_distance_meters,
    };
    config.validate()?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_device() -> PairedDevice {
        PairedDevice {
            device_id: "pixel-9".to_string(),
            display_name: "Pixel 9".to_string(),
            device_role: DeviceRole::Mobile,
            static_x25519_pubkey: "pubkey-hex".to_string(),
            last_ble_address: "AA:BB:CC:DD:EE:FF".to_string(),
            last_lan_address: "192.168.1.42:5555".to_string(),
            last_endpoint_id: "ef".repeat(32),
            capability_flags: 0x0102,
            auto_accept_files: true,
            paired_at: "2026-11-14T00:00:00Z".to_string(),
        }
    }

    fn sample_config() -> DaemonConfig {
        DaemonConfig {
            visibility_mode: VisibilityMode::EveryoneNearby,
            incoming_directory: "/tmp/keynear-in".to_string(),
            auto_accept_from_paired: true,
            clipboard_sync_enabled: false,
            clipboard_images_enabled: true,
            otp_passthrough_enabled: false,
            audio_handoff_enabled: true,
            ignored_mime_types: vec!["text/plain".to_string(), "image/png".to_string()],
            notify_auto_accept_complete: false,
            audio_continuity_enabled: true,
            proximity_lock_enabled: true,
            presence_gating_enabled: false,
            proximity_distance_meters: 4.25,
        }
    }

    #[test]
    fn paired_device_codec_preserves_every_field() {
        let mut out = Vec::new();
        write_paired_device(&mut out, &sample_device()).expect("encode device");
        let decoded = read_paired_device(&mut Cursor::new(&out)).expect("decode device");
        assert_eq!(decoded, sample_device());
    }

    #[test]
    fn paired_device_codec_rejects_truncated_and_invalid_fields() {
        let mut out = Vec::new();
        write_paired_device(&mut out, &sample_device()).expect("encode device");
        assert!(matches!(
            read_paired_device(&mut Cursor::new(&out[..out.len() - 1])),
            Err(KeynearError::Protocol(_))
        ));

        let mut bad_id = Vec::new();
        write_paired_device(
            &mut bad_id,
            &PairedDevice {
                device_id: "../evil".to_string(),
                ..sample_device()
            },
        )
        .expect("encode bad id");
        assert!(matches!(
            read_paired_device(&mut Cursor::new(&bad_id)),
            Err(KeynearError::Validation {
                field: "deviceId",
                ..
            })
        ));

        let mut bad_role = Vec::new();
        write_str(&mut bad_role, "pixel-9").expect("write id");
        write_str(&mut bad_role, "Pixel 9").expect("write name");
        bad_role.push(0x7f);
        assert!(matches!(
            read_paired_device(&mut Cursor::new(&bad_role)),
            Err(KeynearError::Protocol(_))
        ));
    }

    #[test]
    fn daemon_config_codec_preserves_all_flags_and_mime_types() {
        let mut out = Vec::new();
        write_daemon_config(&mut out, &sample_config()).expect("encode config");
        let decoded = read_daemon_config(&mut Cursor::new(&out)).expect("decode config");
        assert_eq!(decoded, sample_config());
    }

    #[test]
    fn daemon_config_codec_rejects_oversized_mimes_and_bad_values() {
        let too_many = DaemonConfig {
            ignored_mime_types: vec!["text/plain".to_string(); MAX_IPC_MIMES + 1],
            ..sample_config()
        };
        assert!(matches!(
            write_daemon_config(&mut Vec::new(), &too_many),
            Err(KeynearError::FrameTooLarge { .. })
        ));

        let mut out = Vec::new();
        write_daemon_config(&mut out, &sample_config()).expect("encode config");
        assert!(matches!(
            read_daemon_config(&mut Cursor::new(&out[..out.len() - 1])),
            Err(KeynearError::Protocol(_))
        ));

        let invalid = DaemonConfig {
            proximity_distance_meters: f32::NAN,
            ..sample_config()
        };
        let mut invalid_out = Vec::new();
        write_daemon_config(&mut invalid_out, &invalid).expect("encode invalid config");
        assert!(matches!(
            read_daemon_config(&mut Cursor::new(&invalid_out)),
            Err(KeynearError::Validation {
                field: "proximityDistanceMeters",
                ..
            })
        ));
    }
}
