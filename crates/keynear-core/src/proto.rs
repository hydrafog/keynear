use prost::Message;

use crate::error::KeynearError;

pub mod v1 {
    include!(concat!(env!("OUT_DIR"), "/keynear.v1.rs"));
}

pub const MAX_CONTROL_FRAME_BYTES: usize = 65_536;

#[derive(Clone, PartialEq, Message)]
pub struct ControlFrame {
    #[prost(uint32, tag = "1")]
    pub version: u32,
    #[prost(uint64, tag = "2")]
    pub sequence: u64,
    #[prost(int64, tag = "3")]
    pub timestamp_ms: i64,
    #[prost(string, tag = "4")]
    pub kind: String,
    #[prost(bytes = "vec", tag = "5")]
    pub payload: Vec<u8>,
}

pub fn encode_control_frame(frame: &ControlFrame) -> Result<Vec<u8>, KeynearError> {
    let encoded_len = frame.encoded_len();
    if encoded_len > MAX_CONTROL_FRAME_BYTES {
        return Err(KeynearError::FrameTooLarge {
            size: encoded_len,
            max: MAX_CONTROL_FRAME_BYTES,
        });
    }
    Ok(frame.encode_to_vec())
}

pub fn decode_control_frame(bytes: &[u8]) -> Result<ControlFrame, KeynearError> {
    if bytes.len() > MAX_CONTROL_FRAME_BYTES {
        return Err(KeynearError::FrameTooLarge {
            size: bytes.len(),
            max: MAX_CONTROL_FRAME_BYTES,
        });
    }
    ControlFrame::decode(bytes).map_err(|err| KeynearError::Protocol(err.to_string()))
}

pub fn encode_envelope(envelope: &v1::Envelope) -> Result<Vec<u8>, KeynearError> {
    if envelope.payload.is_none() {
        return Err(KeynearError::Protocol(
            "envelope payload must not be empty".to_string(),
        ));
    }
    let encoded_len = envelope.encoded_len();
    if encoded_len > MAX_CONTROL_FRAME_BYTES {
        return Err(KeynearError::FrameTooLarge {
            size: encoded_len,
            max: MAX_CONTROL_FRAME_BYTES,
        });
    }
    Ok(envelope.encode_to_vec())
}

pub fn decode_envelope(bytes: &[u8]) -> Result<v1::Envelope, KeynearError> {
    if bytes.len() > MAX_CONTROL_FRAME_BYTES {
        return Err(KeynearError::FrameTooLarge {
            size: bytes.len(),
            max: MAX_CONTROL_FRAME_BYTES,
        });
    }
    let envelope =
        v1::Envelope::decode(bytes).map_err(|err| KeynearError::Protocol(err.to_string()))?;
    if envelope.payload.is_none() {
        return Err(KeynearError::Protocol(
            "envelope payload must not be empty".to_string(),
        ));
    }
    Ok(envelope)
}

pub fn envelope_to_control_frame(envelope: &v1::Envelope) -> Result<ControlFrame, KeynearError> {
    let (kind, payload) = match envelope.payload.as_ref() {
        Some(v1::envelope::Payload::Handshake(inner)) => ("handshake", inner.encode_to_vec()),
        Some(v1::envelope::Payload::TransferOffer(inner)) => {
            ("transfer_offer", inner.encode_to_vec())
        }
        Some(v1::envelope::Payload::Clipboard(inner)) => ("clipboard", inner.encode_to_vec()),
        Some(v1::envelope::Payload::AudioState(inner)) => ("audio_state", inner.encode_to_vec()),
        Some(v1::envelope::Payload::ControlAck(inner)) => ("control_ack", inner.encode_to_vec()),
        None => {
            return Err(KeynearError::Protocol(
                "envelope payload must not be empty".to_string(),
            ))
        }
    };

    let frame = ControlFrame {
        version: envelope.version,
        sequence: envelope.sequence,
        timestamp_ms: envelope.timestamp_ms,
        kind: kind.to_string(),
        payload,
    };

    let encoded_len = frame.encoded_len();
    if encoded_len > MAX_CONTROL_FRAME_BYTES {
        return Err(KeynearError::FrameTooLarge {
            size: encoded_len,
            max: MAX_CONTROL_FRAME_BYTES,
        });
    }

    Ok(frame)
}

pub fn control_frame_to_envelope(frame: &ControlFrame) -> Result<v1::Envelope, KeynearError> {
    let encoded_len = frame.encoded_len();
    if encoded_len > MAX_CONTROL_FRAME_BYTES {
        return Err(KeynearError::FrameTooLarge {
            size: encoded_len,
            max: MAX_CONTROL_FRAME_BYTES,
        });
    }

    let slice = frame.payload.as_slice();
    let payload = match frame.kind.as_str() {
        "handshake" => v1::envelope::Payload::Handshake(
            v1::HandshakePayload::decode(slice)
                .map_err(|err| KeynearError::Protocol(err.to_string()))?,
        ),
        "transfer_offer" => v1::envelope::Payload::TransferOffer(
            v1::TransferOffer::decode(slice)
                .map_err(|err| KeynearError::Protocol(err.to_string()))?,
        ),
        "clipboard" => v1::envelope::Payload::Clipboard(
            v1::ClipboardPayload::decode(slice)
                .map_err(|err| KeynearError::Protocol(err.to_string()))?,
        ),
        "audio_state" => v1::envelope::Payload::AudioState(
            v1::AudioStatePayload::decode(slice)
                .map_err(|err| KeynearError::Protocol(err.to_string()))?,
        ),
        "control_ack" => v1::envelope::Payload::ControlAck(
            v1::ControlAck::decode(slice).map_err(|err| KeynearError::Protocol(err.to_string()))?,
        ),
        other => {
            return Err(KeynearError::Protocol(format!(
                "unknown control frame kind '{other}'"
            )))
        }
    };

    Ok(v1::Envelope {
        version: frame.version,
        sequence: frame.sequence,
        timestamp_ms: frame.timestamp_ms,
        payload: Some(payload),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn envelope_roundtrip_all_variants() {
        let cases = vec![
            sample_envelope(v1::envelope::Payload::Handshake(v1::HandshakePayload {
                device_id: "dev-1".to_string(),
                display_name: "Pixel".to_string(),
                ephemeral_pubkey: vec![0xAA; 32],
                capability_flags: 3,
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
        ];

        for env in cases {
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
}
