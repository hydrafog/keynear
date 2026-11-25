use super::*;

pub(super) fn sample_phoenix_envelopes() -> Vec<v1::Envelope> {
    vec![
        sample_envelope(v1::envelope::Payload::AudioContinuityObservation(
            v1::AudioContinuityObservation {
                device_id: "phone-1".to_string(),
                peripheral_id: "AA:BB:CC:DD:EE:FF".to_string(),
                kind: v1::ContinuityObservationKind::ContinuityPlaybackStarted as i32,
                sequence: 9,
                volume: 72,
                sink_profile: v1::SinkProfile::SinkBap as i32,
            },
        )),
        sample_envelope(v1::envelope::Payload::AudioContinuityCommand(
            v1::AudioContinuityCommand {
                owner_device_id: "desktop-1".to_string(),
                peripheral_id: "AA:BB:CC:DD:EE:FF".to_string(),
                kind: v1::ContinuityCommandKind::ContinuityClaim as i32,
                sequence: 10,
                volume: 72,
                sink_profile: v1::SinkProfile::SinkBap as i32,
                reason: "playback_started".to_string(),
            },
        )),
        sample_envelope(v1::envelope::Payload::RangingReport(v1::RangingReport {
            device_id: "phone-1".to_string(),
            technology: v1::RangingTechnology::Uwb as i32,
            distance_meters: 1.75,
            rssi_dbm: -58,
            sequence: 11,
        })),
        sample_envelope(v1::envelope::Payload::ProximityState(v1::ProximityState {
            device_id: "phone-1".to_string(),
            status: v1::ProximityStatus::ProximityOutOfRange as i32,
            distance_meters: 6.25,
            session_locked: true,
            sequence: 12,
        })),
    ]
}

#[test]
fn phoenix_payloads_use_tags_33_through_36() {
    use prost::Message;

    let expected_keys: [(&str, [u8; 2]); 4] = [
        ("audio_continuity_observation", [0x8A, 0x02]),
        ("audio_continuity_command", [0x92, 0x02]),
        ("ranging_report", [0x9A, 0x02]),
        ("proximity_state", [0xA2, 0x02]),
    ];
    for (env, (name, key)) in sample_phoenix_envelopes()
        .iter()
        .zip(expected_keys.iter())
    {
        let encoded = env.encode_to_vec();
        assert!(
            encoded.windows(2).any(|window| window == key),
            "{name} must use its reserved Envelope tag"
        );
        assert!(env.payload.is_some());
    }

    for env in sample_phoenix_envelopes() {
        let frame = envelope_to_control_frame(&env).expect("phoenix control frame");
        assert!(frame.kind.starts_with("audio_continuity")
            || frame.kind == "ranging_report"
            || frame.kind == "proximity_state");
        let decoded = control_frame_to_envelope(&frame).expect("decoded phoenix frame");
        assert_eq!(decoded, env);
    }
}
