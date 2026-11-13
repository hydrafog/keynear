use prost::Message;

use crate::error::KeynearError;

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
