use super::*;

fn oneof_key_bytes(payload: v1::envelope::Payload) -> Vec<u8> {
    let envelope = v1::Envelope {
        version: 0,
        sequence: 0,
        timestamp_ms: 0,
        payload: Some(payload),
    };
    encode_envelope(&envelope).expect("encode envelope")
}

fn sample_carina_envelopes() -> Vec<v1::Envelope> {
    use v1::envelope::Payload as P;
    vec![
        sample_envelope(P::MirroredNotification(v1::MirroredNotification {
            notification_key: "n-1".to_string(),
            package_name: "org.example.chat".to_string(),
            title: "Alice".to_string(),
            body: "Hello".to_string(),
            category: "msg".to_string(),
            sequence: 1,
            supports_reply: true,
            supports_mark_read: true,
            supports_dismiss: true,
        })),
        sample_envelope(P::NotificationAction(v1::NotificationAction {
            notification_key: "n-1".to_string(),
            kind: v1::NotificationActionKind::NotificationActionReply as i32,
            reply_text: "Okay".to_string(),
            sequence: 2,
        })),
        sample_envelope(P::CallAlert(v1::CallAlert {
            call_id: "call-1".to_string(),
            caller: "Pixel 9".to_string(),
            state: v1::CallState::Incoming as i32,
            sink_profile: v1::SinkProfile::SinkHfpHf as i32,
            sequence: 3,
            supports_answer: true,
            supports_decline: true,
            supports_mute: true,
        })),
        sample_envelope(P::CallAction(v1::CallAction {
            call_id: "call-1".to_string(),
            kind: v1::CallActionKind::CallActionAnswer as i32,
            sequence: 4,
        })),
        sample_envelope(P::MediaSessionState(v1::MediaSessionState {
            player_id: "player-1".to_string(),
            title: "Track".to_string(),
            artist: "Artist".to_string(),
            album: "Album".to_string(),
            playback: v1::PlaybackState::PlaybackPlaying as i32,
            position_ms: 1_000,
            duration_ms: 200_000,
            sequence: 5,
            can_pause: true,
            can_next: true,
            can_previous: true,
        })),
        sample_envelope(P::MediaCommand(v1::MediaCommand {
            player_id: "player-1".to_string(),
            kind: v1::MediaCommandKind::MediaCommandPause as i32,
            sequence: 6,
        })),
    ]
}

#[test]
fn carina_tags_and_perseus_tags_stay_stable() {
    use v1::envelope::Payload as P;
    assert!(
        oneof_key_bytes(P::MirroredNotification(v1::MirroredNotification::default()))
            .starts_with(&[0xDA, 0x01])
    );
    assert!(
        oneof_key_bytes(P::NotificationAction(v1::NotificationAction::default()))
            .starts_with(&[0xE2, 0x01])
    );
    assert!(oneof_key_bytes(P::CallAlert(v1::CallAlert::default())).starts_with(&[0xEA, 0x01]));
    assert!(oneof_key_bytes(P::CallAction(v1::CallAction::default())).starts_with(&[0xF2, 0x01]));
    assert!(
        oneof_key_bytes(P::MediaSessionState(v1::MediaSessionState::default()))
            .starts_with(&[0xFA, 0x01])
    );
    assert!(
        oneof_key_bytes(P::MediaCommand(v1::MediaCommand::default())).starts_with(&[0x82, 0x02])
    );
    assert!(
        oneof_key_bytes(P::TransferResumeAck(v1::TransferResumeAck::default()))
            .starts_with(&[0xD2, 0x01])
    );
}

#[test]
fn carina_payloads_roundtrip_through_control_frames() {
    use v1::envelope::Payload as P;
    for envelope in sample_carina_envelopes() {
        let bytes = encode_envelope(&envelope).expect("encode envelope");
        let decoded = decode_envelope(&bytes).expect("decode envelope");
        assert_eq!(decoded, envelope);

        let frame = envelope_to_control_frame(&envelope).expect("to frame");
        let expected_kind = match &envelope.payload {
            Some(P::MirroredNotification(_)) => "mirrored_notification",
            Some(P::NotificationAction(_)) => "notification_action",
            Some(P::CallAlert(_)) => "call_alert",
            Some(P::CallAction(_)) => "call_action",
            Some(P::MediaSessionState(_)) => "media_session_state",
            Some(P::MediaCommand(_)) => "media_command",
            _ => unreachable!(),
        };
        assert_eq!(frame.kind, expected_kind);
        let restored = control_frame_to_envelope(&frame).expect("from frame");
        assert_eq!(restored, envelope);
    }
}

#[test]
fn oversized_carina_text_is_rejected_before_decode() {
    use v1::envelope::Payload as P;
    let envelope = sample_envelope(P::MirroredNotification(v1::MirroredNotification {
        body: "x".repeat(MAX_CONTROL_FRAME_BYTES),
        ..v1::MirroredNotification::default()
    }));
    let err = encode_envelope(&envelope).expect_err("oversized envelope rejected");
    assert!(matches!(err, KeynearError::FrameTooLarge { .. }));
}
