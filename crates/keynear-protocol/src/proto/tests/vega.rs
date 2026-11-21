use super::*;

pub(super) fn sample_vega_envelopes() -> Vec<v1::Envelope> {
    vec![
        sample_envelope(v1::envelope::Payload::AudioHandoff(v1::AudioHandoff {
            peripheral_id: "AA:BB:CC:DD:EE:FF".to_string(),
            sequence: 3,
            phase: v1::HandoffPhase::HandoffPrepare as i32,
            sink_profile: v1::SinkProfile::SinkA2dp as i32,
            playback_paused: true,
            reason: String::new(),
        })),
        sample_envelope(v1::envelope::Payload::CapabilityNegotiation(
            v1::CapabilityNegotiation {
                local_flags: 0x0f,
                peer_flags: 0x1b,
                negotiated_flags: 0x0b,
                preferred_ranging: v1::RangingMethod::RangingBleCs as i32,
            },
        )),
        sample_envelope(v1::envelope::Payload::FallbackProbe(v1::FallbackProbe {
            media: v1::FallbackMedia::MediaWifiAwareNan as i32,
            status: v1::FallbackStatus::StatusAvailable as i32,
            detail: "nan-available".to_string(),
        })),
    ]
}

#[test]
fn vega_tags_and_capability_bits_stay_stable() {
    let audio = v1::AudioHandoff {
        peripheral_id: "headset".to_string(),
        sequence: 1,
        phase: v1::HandoffPhase::HandoffPrepare as i32,
        sink_profile: v1::SinkProfile::SinkBap as i32,
        playback_paused: true,
        reason: String::new(),
    };
    let frame =
        envelope_to_control_frame(&sample_envelope(v1::envelope::Payload::AudioHandoff(audio)))
            .expect("vega frame");
    assert_eq!(frame.kind, "audio_handoff");

    let oversized = sample_envelope(v1::envelope::Payload::AudioHandoff(v1::AudioHandoff {
        peripheral_id: "x".repeat(MAX_CONTROL_FRAME_BYTES),
        sequence: 1,
        phase: v1::HandoffPhase::HandoffPrepare as i32,
        sink_profile: v1::SinkProfile::SinkHfpHf as i32,
        playback_paused: false,
        reason: String::new(),
    }));
    let err = encode_envelope(&oversized).expect_err("oversized vega envelope must fail");
    assert!(matches!(err, KeynearError::FrameTooLarge { .. }));
}
