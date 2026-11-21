use prost::Message;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::error::KeynearError;

#[allow(clippy::too_many_arguments)]
pub mod v1 {
    include!(concat!(env!("OUT_DIR"), "/keynear.v1.rs"));
}

#[cfg(test)]
mod tests;

pub const MAX_CONTROL_FRAME_BYTES: usize = 65_536;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuicStreamKind {
    Control = 0x01,
    RawFileChunk = 0x02,
    StoredZipArchive = 0x03,
    ClipboardBlob = 0x04,
}

impl TryFrom<u8> for QuicStreamKind {
    type Error = KeynearError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x01 => Ok(Self::Control),
            0x02 => Ok(Self::RawFileChunk),
            0x03 => Ok(Self::StoredZipArchive),
            0x04 => Ok(Self::ClipboardBlob),
            other => Err(KeynearError::Protocol(format!(
                "unknown QUIC stream kind 0x{other:02x}"
            ))),
        }
    }
}

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
        Some(v1::envelope::Payload::LanEndpoint(inner)) => ("lan_endpoint", inner.encode_to_vec()),
        Some(v1::envelope::Payload::TransferConsent(inner)) => {
            ("transfer_consent", inner.encode_to_vec())
        }
        Some(v1::envelope::Payload::TransferCancel(inner)) => {
            ("transfer_cancel", inner.encode_to_vec())
        }
        Some(v1::envelope::Payload::TransferComplete(inner)) => {
            ("transfer_complete", inner.encode_to_vec())
        }
        Some(v1::envelope::Payload::ClipboardOffer(inner)) => {
            ("clipboard_offer", inner.encode_to_vec())
        }
        Some(v1::envelope::Payload::ClipboardFetch(inner)) => {
            ("clipboard_fetch", inner.encode_to_vec())
        }
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

fn decode_frame_payload(frame: &ControlFrame) -> Result<v1::envelope::Payload, KeynearError> {
    let slice = frame.payload.as_slice();
    let decoded = |err: prost::DecodeError| KeynearError::Protocol(err.to_string());
    let payload = match frame.kind.as_str() {
        "handshake" => {
            v1::envelope::Payload::Handshake(v1::HandshakePayload::decode(slice).map_err(decoded)?)
        }
        "transfer_offer" => {
            v1::envelope::Payload::TransferOffer(v1::TransferOffer::decode(slice).map_err(decoded)?)
        }
        "clipboard" => {
            v1::envelope::Payload::Clipboard(v1::ClipboardPayload::decode(slice).map_err(decoded)?)
        }
        "audio_state" => v1::envelope::Payload::AudioState(
            v1::AudioStatePayload::decode(slice).map_err(decoded)?,
        ),
        "control_ack" => {
            v1::envelope::Payload::ControlAck(v1::ControlAck::decode(slice).map_err(decoded)?)
        }
        "lan_endpoint" => {
            v1::envelope::Payload::LanEndpoint(v1::LanEndpoint::decode(slice).map_err(decoded)?)
        }
        "transfer_consent" => v1::envelope::Payload::TransferConsent(
            v1::TransferConsent::decode(slice).map_err(decoded)?,
        ),
        "transfer_cancel" => v1::envelope::Payload::TransferCancel(
            v1::TransferCancel::decode(slice).map_err(decoded)?,
        ),
        "transfer_complete" => v1::envelope::Payload::TransferComplete(
            v1::TransferComplete::decode(slice).map_err(decoded)?,
        ),
        "clipboard_offer" => v1::envelope::Payload::ClipboardOffer(
            v1::ClipboardOffer::decode(slice).map_err(decoded)?,
        ),
        "clipboard_fetch" => v1::envelope::Payload::ClipboardFetch(
            v1::ClipboardFetch::decode(slice).map_err(decoded)?,
        ),
        other => {
            return Err(KeynearError::Protocol(format!(
                "unknown control frame kind '{other}'"
            )))
        }
    };
    Ok(payload)
}

pub fn control_frame_to_envelope(frame: &ControlFrame) -> Result<v1::Envelope, KeynearError> {
    let encoded_len = frame.encoded_len();
    if encoded_len > MAX_CONTROL_FRAME_BYTES {
        return Err(KeynearError::FrameTooLarge {
            size: encoded_len,
            max: MAX_CONTROL_FRAME_BYTES,
        });
    }

    Ok(v1::Envelope {
        version: frame.version,
        sequence: frame.sequence,
        timestamp_ms: frame.timestamp_ms,
        payload: Some(decode_frame_payload(frame)?),
    })
}

pub async fn write_stream_kind<W: AsyncWrite + Unpin>(
    writer: &mut W,
    kind: QuicStreamKind,
) -> Result<(), KeynearError> {
    writer.write_all(&[kind as u8]).await?;
    Ok(())
}

pub async fn read_stream_kind<R: AsyncRead + Unpin>(
    reader: &mut R,
) -> Result<QuicStreamKind, KeynearError> {
    let mut buf = [0u8; 1];
    reader.read_exact(&mut buf).await?;
    QuicStreamKind::try_from(buf[0])
}

pub async fn write_control_envelope<W: AsyncWrite + Unpin>(
    writer: &mut W,
    envelope: &v1::Envelope,
) -> Result<(), KeynearError> {
    let encoded = encode_envelope(envelope)?;
    let len = u32::try_from(encoded.len()).map_err(|_| KeynearError::FrameTooLarge {
        size: encoded.len(),
        max: MAX_CONTROL_FRAME_BYTES,
    })?;
    writer.write_all(&len.to_be_bytes()).await?;
    writer.write_all(&encoded).await?;
    Ok(())
}

pub async fn read_control_envelope<R: AsyncRead + Unpin>(
    reader: &mut R,
) -> Result<v1::Envelope, KeynearError> {
    let mut len_buf = [0u8; 4];
    reader.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf) as usize;
    if len == 0 {
        return Err(KeynearError::Protocol(
            "control frame length must be positive".to_string(),
        ));
    }
    if len > MAX_CONTROL_FRAME_BYTES {
        return Err(KeynearError::FrameTooLarge {
            size: len,
            max: MAX_CONTROL_FRAME_BYTES,
        });
    }
    let mut payload = vec![0u8; len];
    reader.read_exact(&mut payload).await?;
    decode_envelope(&payload)
}
