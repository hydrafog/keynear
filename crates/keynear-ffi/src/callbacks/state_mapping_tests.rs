use super::*;

use keynear_protocol::{encode_control_frame, ControlFrame};
use prost::Message;

#[test]
fn to_key_32_preserves_exact_bytes_and_reports_actual_length() {
    let pattern: Vec<u8> = (0..32).collect();
    let key = to_key_32(&pattern, "reconnectKey").expect("exact bytes");
    assert_eq!(&key[..], &pattern[..]);
    let err = to_key_32(&[0u8; 5], "reconnectKey").expect_err("short key");
    assert!(err.to_string().contains("expected 32 bytes, got 5"));
}

#[test]
fn validate_control_frame_impl_accepts_known_kind_and_rejects_bad_frames() {
    let frame = ControlFrame {
        version: 1,
        sequence: 1,
        timestamp_ms: 1,
        kind: "control_ack".to_string(),
        payload: v1::ControlAck {
            acked_sequence: 1,
            accepted: true,
            reason: String::new(),
        }
        .encode_to_vec(),
    };
    let bytes = encode_control_frame(&frame).expect("encode frame");
    assert_eq!(
        validate_control_frame_impl(bytes).expect("valid frame"),
        "control_ack"
    );

    assert!(matches!(
        validate_control_frame_impl(vec![0xff; 11]),
        Err(FfiError::Protocol { .. })
    ));
    let unknown = ControlFrame {
        kind: "no_such_kind".to_string(),
        ..frame
    };
    let unknown_bytes = encode_control_frame(&unknown).expect("encode unknown frame");
    assert!(matches!(
        validate_control_frame_impl(unknown_bytes),
        Err(FfiError::Protocol { .. })
    ));
    assert!(matches!(
        validate_control_frame_impl(vec![0u8; MAX_CONTROL_FRAME_BYTES + 1]),
        Err(FfiError::FrameTooLarge { .. })
    ));
}
