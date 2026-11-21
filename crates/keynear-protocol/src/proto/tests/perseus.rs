use super::*;

pub(super) fn sample_perseus_envelopes() -> Vec<v1::Envelope> {
    vec![
        sample_envelope(v1::envelope::Payload::TransferCheckpoint(
            v1::TransferCheckpoint {
                transfer_id: "tx-perseus".to_string(),
                sequence: 4,
                chunk_count: 12,
                chunk_bitmap: vec![0b0000_0111, 0b0000_0001],
                rolling_sha256: vec![0x44; 32],
                verified_bytes: 7 * crate::STREAM_CHUNK_BYTES as u64,
            },
        )),
        sample_envelope(v1::envelope::Payload::TransferResumeRequest(
            v1::TransferResumeRequest {
                transfer_id: "tx-perseus".to_string(),
                sequence: 5,
                first_missing_chunk: 8,
                verified_bytes: 7 * crate::STREAM_CHUNK_BYTES as u64,
            },
        )),
        sample_envelope(v1::envelope::Payload::TransferResumeAck(
            v1::TransferResumeAck {
                transfer_id: "tx-perseus".to_string(),
                sequence: 5,
                accepted: true,
                resume_from_chunk: 8,
                reason: String::new(),
            },
        )),
    ]
}

fn oneof_key_bytes(payload: v1::envelope::Payload) -> Vec<u8> {
    let envelope = v1::Envelope {
        version: 0,
        sequence: 0,
        timestamp_ms: 0,
        payload: Some(payload),
    };
    encode_envelope(&envelope).expect("encode envelope")
}

#[test]
fn perseus_tags_and_capability_bits_stay_stable() {
    let checkpoint = oneof_key_bytes(v1::envelope::Payload::TransferCheckpoint(
        v1::TransferCheckpoint::default(),
    ));
    assert!(checkpoint.starts_with(&[0xC2, 0x01]));

    let request = oneof_key_bytes(v1::envelope::Payload::TransferResumeRequest(
        v1::TransferResumeRequest::default(),
    ));
    assert!(request.starts_with(&[0xCA, 0x01]));

    let ack = oneof_key_bytes(v1::envelope::Payload::TransferResumeAck(
        v1::TransferResumeAck::default(),
    ));
    assert!(ack.starts_with(&[0xD2, 0x01]));

    let legacy = oneof_key_bytes(v1::envelope::Payload::FallbackProbe(
        v1::FallbackProbe::default(),
    ));
    assert!(legacy.starts_with(&[0xBA, 0x01]));
}

#[test]
fn perseus_kinds_map_to_stable_control_frames() {
    use v1::envelope::Payload as P;
    for envelope in sample_perseus_envelopes() {
        let expected = match &envelope.payload {
            Some(P::TransferCheckpoint(_)) => "transfer_checkpoint",
            Some(P::TransferResumeRequest(_)) => "transfer_resume_request",
            Some(P::TransferResumeAck(_)) => "transfer_resume_ack",
            _ => panic!("unexpected perseus payload"),
        };
        let frame = envelope_to_control_frame(&envelope).expect("perseus control frame");
        assert_eq!(frame.kind, expected);
        let decoded = control_frame_to_envelope(&frame).expect("decode perseus frame");
        assert_eq!(decoded, envelope);
    }
}

#[test]
fn perseus_oversized_checkpoint_is_rejected_before_decode() {
    let oversized = sample_envelope(v1::envelope::Payload::TransferCheckpoint(
        v1::TransferCheckpoint {
            transfer_id: "tx-perseus".to_string(),
            sequence: 1,
            chunk_count: 1,
            chunk_bitmap: vec![0u8; MAX_CONTROL_FRAME_BYTES],
            rolling_sha256: vec![0u8; 32],
            verified_bytes: 0,
        },
    ));
    let err = encode_envelope(&oversized).expect_err("oversized perseus envelope must fail");
    assert!(matches!(err, KeynearError::FrameTooLarge { .. }));
}
