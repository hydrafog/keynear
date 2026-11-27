#![allow(clippy::too_many_lines)]

use super::*;

mod continuity;
mod phoenix;
mod resume;
mod vega;

fn sample_envelope(payload: v1::envelope::Payload) -> v1::Envelope {
    v1::Envelope {
        version: 1,
        sequence: 7,
        timestamp_ms: 1_763_000_000_000,
        payload: Some(payload),
    }
}

#[test]
fn frame_roundtrip_succeeds() {
    let frame = ControlFrame {
        version: 1,
        sequence: 42,
        timestamp_ms: 1_763_000_000_000,
        kind: "control_ack".to_string(),
        payload: vec![1, 2, 3, 4],
    };
    let bytes = encode_control_frame(&frame).expect("encode frame");
    let decoded = decode_control_frame(&bytes).expect("decode frame");
    assert_eq!(decoded, frame);
}

#[test]
fn frame_oversized_rejected_before_decode() {
    let oversized = vec![0u8; MAX_CONTROL_FRAME_BYTES + 1];
    let err = decode_control_frame(&oversized).expect_err("oversized frame must fail");
    assert!(matches!(
        err,
        KeynearError::FrameTooLarge {
            size,
            max: MAX_CONTROL_FRAME_BYTES
        } if size == MAX_CONTROL_FRAME_BYTES + 1
    ));
}

#[test]
fn frame_malformed_rejected() {
    let malformed = [
        0xffu8, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x01,
    ];
    let err = decode_control_frame(&malformed).expect_err("malformed protobuf must fail");
    assert!(matches!(err, KeynearError::Protocol(_)));
}

fn sample_session_envelopes() -> Vec<v1::Envelope> {
    vec![
        sample_envelope(v1::envelope::Payload::Handshake(v1::HandshakePayload {
            device_id: "dev-1".to_string(),
            display_name: "Pixel".to_string(),
            ephemeral_pubkey: vec![0xAA; 32],
            capability_flags: 3,
            auth_proof: vec![0x55; 32],
        })),
        sample_envelope(v1::envelope::Payload::TransferOffer(v1::TransferOffer {
            transfer_id: "tx-1".to_string(),
            items: vec![v1::TransferItemMeta {
                name: "notes.txt".to_string(),
                mime_type: "text/plain".to_string(),
                size_bytes: 128,
                is_directory: false,
            }],
            total_bytes: 128,
        })),
        sample_envelope(v1::envelope::Payload::Clipboard(v1::ClipboardPayload {
            mime_type: "text/plain".to_string(),
            data: b"hello".to_vec(),
            is_otp: false,
        })),
        sample_envelope(v1::envelope::Payload::AudioState(v1::AudioStatePayload {
            peripheral_id: "headset-1".to_string(),
            active_call: true,
            sink_claimed: false,
        })),
        sample_envelope(v1::envelope::Payload::ControlAck(v1::ControlAck {
            acked_sequence: 6,
            accepted: true,
            reason: String::new(),
        })),
        sample_envelope(v1::envelope::Payload::LanEndpoint(v1::LanEndpoint {
            device_id: "dev-1".to_string(),
            endpoint_id: vec![0x11; 32],
            socket_addrs: vec!["192.168.1.25:49152".to_string()],
            capability_flags: 1,
            issued_at_ms: 1_763_000_000_000,
        })),
    ]
}

fn sample_transfer_envelopes() -> Vec<v1::Envelope> {
    vec![
        sample_envelope(v1::envelope::Payload::TransferConsent(
            v1::TransferConsent {
                transfer_id: "tx-1".to_string(),
                accepted: true,
                reason: String::new(),
            },
        )),
        sample_envelope(v1::envelope::Payload::TransferCancel(v1::TransferCancel {
            transfer_id: "tx-1".to_string(),
            reason: "user_cancelled".to_string(),
        })),
        sample_envelope(v1::envelope::Payload::TransferComplete(
            v1::TransferComplete {
                transfer_id: "tx-1".to_string(),
                bytes_transferred: 128,
                sha256_digest: vec![0x99; 32],
            },
        )),
        sample_envelope(v1::envelope::Payload::ClipboardOffer(v1::ClipboardOffer {
            offer_id: "offer-1".to_string(),
            mime_types: vec!["image/png".to_string(), "image/jpeg".to_string()],
            is_image: true,
            is_otp: false,
            byte_size: 131_072,
        })),
        sample_envelope(v1::envelope::Payload::ClipboardFetch(v1::ClipboardFetch {
            offer_id: "offer-1".to_string(),
            mime_type: "image/png".to_string(),
        })),
    ]
}

fn sample_envelopes() -> Vec<v1::Envelope> {
    let mut envelopes = sample_session_envelopes();
    envelopes.extend(sample_transfer_envelopes());
    envelopes.extend(vega::sample_vega_envelopes());
    envelopes.extend(resume::sample_resume_envelopes());
    envelopes.extend(phoenix::sample_phoenix_envelopes());
    envelopes
}

#[test]
fn envelope_roundtrip_all_variants() {
    for env in sample_envelopes() {
        let encoded = encode_envelope(&env).expect("encode envelope");
        let decoded = decode_envelope(&encoded).expect("decode envelope");
        assert_eq!(decoded, env);

        let frame = envelope_to_control_frame(&env).expect("convert to control frame");
        let framed_bytes = encode_control_frame(&frame).expect("encode control frame");
        let decoded_frame = decode_control_frame(&framed_bytes).expect("decode control frame");
        let roundtripped =
            control_frame_to_envelope(&decoded_frame).expect("convert back to envelope");
        assert_eq!(roundtripped, env);
    }
}

#[test]
fn envelope_empty_payload_rejected() {
    let empty = v1::Envelope {
        version: 1,
        sequence: 1,
        timestamp_ms: 100,
        payload: None,
    };
    assert!(matches!(
        encode_envelope(&empty),
        Err(KeynearError::Protocol(_))
    ));
    assert!(matches!(
        envelope_to_control_frame(&empty),
        Err(KeynearError::Protocol(_))
    ));

    let raw_empty = empty.encode_to_vec();
    assert!(matches!(
        decode_envelope(&raw_empty),
        Err(KeynearError::Protocol(_))
    ));
}

#[test]
fn envelope_oversized_rejected_before_decode() {
    let oversized = vec![0u8; MAX_CONTROL_FRAME_BYTES + 1];
    assert!(matches!(
        decode_envelope(&oversized),
        Err(KeynearError::FrameTooLarge { .. })
    ));
}

#[test]
fn control_frame_unknown_kind_rejected() {
    let frame = ControlFrame {
        version: 1,
        sequence: 1,
        timestamp_ms: 1,
        kind: "unknown_kind".to_string(),
        payload: vec![],
    };
    assert!(matches!(
        control_frame_to_envelope(&frame),
        Err(KeynearError::Protocol(_))
    ));
}

#[test]
fn quic_stream_kind_roundtrip_and_rejection() {
    for (byte, kind) in [
        (0x01, QuicStreamKind::Control),
        (0x02, QuicStreamKind::RawFileChunk),
        (0x03, QuicStreamKind::StoredZipArchive),
        (0x04, QuicStreamKind::ClipboardBlob),
        (0x05, QuicStreamKind::VfsRead),
    ] {
        assert_eq!(QuicStreamKind::try_from(byte).unwrap(), kind);
    }
    assert!(matches!(
        QuicStreamKind::try_from(0xff),
        Err(KeynearError::Protocol(_))
    ));
}

#[tokio::test]
async fn control_envelope_stream_roundtrip_and_bounds() {
    let env = sample_envelope(v1::envelope::Payload::TransferConsent(
        v1::TransferConsent {
            transfer_id: "tx-42".to_string(),
            accepted: true,
            reason: "ok".to_string(),
        },
    ));

    let mut buf = Vec::new();
    write_stream_kind(&mut buf, QuicStreamKind::Control)
        .await
        .expect("write kind");
    write_control_envelope(&mut buf, &env)
        .await
        .expect("write envelope");

    let mut reader = buf.as_slice();
    assert_eq!(
        read_stream_kind(&mut reader).await.expect("read kind"),
        QuicStreamKind::Control
    );
    let decoded = read_control_envelope(&mut reader)
        .await
        .expect("read envelope");
    assert_eq!(decoded, env);

    let zero_len = 0u32.to_be_bytes();
    assert!(matches!(
        read_control_envelope(&mut zero_len.as_slice()).await,
        Err(KeynearError::Protocol(_))
    ));

    let oversized_len = ((MAX_CONTROL_FRAME_BYTES + 1) as u32).to_be_bytes();
    assert!(matches!(
        read_control_envelope(&mut oversized_len.as_slice()).await,
        Err(KeynearError::FrameTooLarge {
            size,
            max: MAX_CONTROL_FRAME_BYTES
        }) if size == MAX_CONTROL_FRAME_BYTES + 1
    ));
}

#[test]
fn validate_control_envelope_accepts_every_sample_payload() {
    for env in sample_envelopes() {
        validate_control_envelope(&env).expect("valid envelope");
    }
}

#[test]
fn validate_control_envelope_rejects_empty_and_unknown_payloads() {
    let empty = v1::Envelope {
        version: 1,
        sequence: 1,
        timestamp_ms: 1,
        payload: None,
    };
    assert!(matches!(
        validate_control_envelope(&empty),
        Err(KeynearError::Protocol(_))
    ));
    let frame = ControlFrame {
        version: 1,
        sequence: 1,
        timestamp_ms: 1,
        kind: "unknown_kind".to_string(),
        payload: vec![],
    };
    assert!(control_frame_to_envelope(&frame).is_err());
    let oversized = ControlFrame {
        version: 1,
        sequence: 1,
        timestamp_ms: 1,
        kind: "control_ack".to_string(),
        payload: vec![0u8; MAX_CONTROL_FRAME_BYTES],
    };
    let decoded_oversized = control_frame_to_envelope(&oversized);
    assert!(
        decoded_oversized.is_err()
            || matches!(
                envelope_to_control_frame(&decoded_oversized.expect("decoded")),
                Err(KeynearError::FrameTooLarge { .. })
            )
    );
}

fn all_payload_cases() -> Vec<(&'static str, v1::envelope::Payload)> {
    use v1::envelope::Payload as P;
    vec![
        ("handshake", P::Handshake(Default::default())),
        ("transfer_offer", P::TransferOffer(Default::default())),
        ("clipboard", P::Clipboard(Default::default())),
        ("audio_state", P::AudioState(Default::default())),
        ("control_ack", P::ControlAck(Default::default())),
        ("lan_endpoint", P::LanEndpoint(Default::default())),
        ("transfer_consent", P::TransferConsent(Default::default())),
        ("transfer_cancel", P::TransferCancel(Default::default())),
        ("transfer_complete", P::TransferComplete(Default::default())),
        ("clipboard_offer", P::ClipboardOffer(Default::default())),
        ("clipboard_fetch", P::ClipboardFetch(Default::default())),
        ("audio_handoff", P::AudioHandoff(Default::default())),
        (
            "capability_negotiation",
            P::CapabilityNegotiation(Default::default()),
        ),
        ("fallback_probe", P::FallbackProbe(Default::default())),
        (
            "transfer_checkpoint",
            P::TransferCheckpoint(Default::default()),
        ),
        (
            "transfer_resume_request",
            P::TransferResumeRequest(Default::default()),
        ),
        (
            "transfer_resume_ack",
            P::TransferResumeAck(Default::default()),
        ),
        (
            "mirrored_notification",
            P::MirroredNotification(Default::default()),
        ),
        (
            "notification_action",
            P::NotificationAction(Default::default()),
        ),
        ("call_alert", P::CallAlert(Default::default())),
        ("call_action", P::CallAction(Default::default())),
        (
            "media_session_state",
            P::MediaSessionState(Default::default()),
        ),
        ("media_command", P::MediaCommand(Default::default())),
        (
            "audio_continuity_observation",
            P::AudioContinuityObservation(Default::default()),
        ),
        (
            "audio_continuity_command",
            P::AudioContinuityCommand(Default::default()),
        ),
        ("ranging_report", P::RangingReport(Default::default())),
        ("proximity_state", P::ProximityState(Default::default())),
    ]
}

#[test]
fn payload_kind_and_bytes_cover_every_oneof_tag() {
    for (expected_kind, payload) in all_payload_cases() {
        let (kind, bytes) = payload_kind_and_bytes(&payload);
        assert_eq!(kind, expected_kind);

        let restored = decode_payload(kind, &bytes).expect("decode payload bytes");
        let (restored_kind, restored_bytes) = payload_kind_and_bytes(&restored);
        assert_eq!(restored_kind, expected_kind);
        assert_eq!(restored_bytes, bytes);
    }
}

#[test]
fn decode_payload_rejects_unknown_kind_and_malformed_bytes() {
    assert!(matches!(
        decode_payload("no_such_kind", &[]),
        Err(KeynearError::Protocol(_))
    ));
    assert!(matches!(
        decode_payload("control_ack", &[0xff, 0xff]),
        Err(KeynearError::Protocol(_))
    ));
}

#[tokio::test]
async fn stream_helpers_reject_truncated_and_unknown_frames() {
    let mut unknown = Vec::new();
    write_stream_kind(&mut unknown, QuicStreamKind::Control)
        .await
        .expect("write kind");
    unknown.extend_from_slice(&0u32.to_be_bytes());
    assert!(read_control_envelope(&mut unknown.as_slice())
        .await
        .is_err());
    let mut oversized = Vec::new();
    write_stream_kind(&mut oversized, QuicStreamKind::Control)
        .await
        .expect("write kind");
    oversized.extend_from_slice(&((MAX_CONTROL_FRAME_BYTES + 1) as u32).to_be_bytes());
    assert!(matches!(
        read_control_envelope(&mut oversized.as_slice()).await,
        Err(KeynearError::FrameTooLarge { .. })
    ));
    let mut bad_kind = [0xffu8].as_slice();
    assert!(matches!(
        read_stream_kind(&mut bad_kind).await,
        Err(KeynearError::Protocol(_))
    ));
}

#[test]
fn encode_control_frame_rejects_oversized_payload() {
    let frame = ControlFrame {
        version: 1,
        sequence: 1,
        timestamp_ms: 1,
        kind: "clipboard".to_string(),
        payload: vec![0u8; MAX_CONTROL_FRAME_BYTES],
    };
    let err = encode_control_frame(&frame).expect_err("oversized frame must fail");
    assert!(matches!(
        err,
        KeynearError::FrameTooLarge {
            size,
            max: MAX_CONTROL_FRAME_BYTES
        } if size > MAX_CONTROL_FRAME_BYTES
    ));
}

#[test]
fn envelope_to_control_frame_rejects_oversized_clipboard() {
    let env = sample_envelope(v1::envelope::Payload::Clipboard(v1::ClipboardPayload {
        mime_type: "application/octet-stream".to_string(),
        data: vec![0u8; MAX_CONTROL_FRAME_BYTES],
        is_otp: false,
    }));
    let err = envelope_to_control_frame(&env).expect_err("oversized clipboard must fail");
    assert!(matches!(
        err,
        KeynearError::FrameTooLarge {
            size,
            max: MAX_CONTROL_FRAME_BYTES
        } if size > MAX_CONTROL_FRAME_BYTES
    ));
}
