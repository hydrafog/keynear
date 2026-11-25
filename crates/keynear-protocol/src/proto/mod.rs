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
    VfsRead = 0x05,
}

impl TryFrom<u8> for QuicStreamKind {
    type Error = KeynearError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x01 => Ok(Self::Control),
            0x02 => Ok(Self::RawFileChunk),
            0x03 => Ok(Self::StoredZipArchive),
            0x04 => Ok(Self::ClipboardBlob),
            0x05 => Ok(Self::VfsRead),
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

fn payload_kind_and_bytes(payload: &v1::envelope::Payload) -> (&'static str, Vec<u8>) {
    use v1::envelope::Payload as P;
    match payload {
        P::Handshake(value) => ("handshake", value.encode_to_vec()),
        P::TransferOffer(value) => ("transfer_offer", value.encode_to_vec()),
        P::Clipboard(value) => ("clipboard", value.encode_to_vec()),
        P::AudioState(value) => ("audio_state", value.encode_to_vec()),
        P::ControlAck(value) => ("control_ack", value.encode_to_vec()),
        P::LanEndpoint(value) => ("lan_endpoint", value.encode_to_vec()),
        P::TransferConsent(value) => ("transfer_consent", value.encode_to_vec()),
        P::TransferCancel(value) => ("transfer_cancel", value.encode_to_vec()),
        P::TransferComplete(value) => ("transfer_complete", value.encode_to_vec()),
        P::ClipboardOffer(value) => ("clipboard_offer", value.encode_to_vec()),
        P::ClipboardFetch(value) => ("clipboard_fetch", value.encode_to_vec()),
        P::AudioHandoff(value) => ("audio_handoff", value.encode_to_vec()),
        P::CapabilityNegotiation(value) => ("capability_negotiation", value.encode_to_vec()),
        P::FallbackProbe(value) => ("fallback_probe", value.encode_to_vec()),
        P::TransferCheckpoint(value) => ("transfer_checkpoint", value.encode_to_vec()),
        P::TransferResumeRequest(value) => ("transfer_resume_request", value.encode_to_vec()),
        P::TransferResumeAck(value) => ("transfer_resume_ack", value.encode_to_vec()),
        P::MirroredNotification(value) => ("mirrored_notification", value.encode_to_vec()),
        P::NotificationAction(value) => ("notification_action", value.encode_to_vec()),
        P::CallAlert(value) => ("call_alert", value.encode_to_vec()),
        P::CallAction(value) => ("call_action", value.encode_to_vec()),
        P::MediaSessionState(value) => ("media_session_state", value.encode_to_vec()),
        P::MediaCommand(value) => ("media_command", value.encode_to_vec()),
        P::AudioContinuityObservation(value) => {
            ("audio_continuity_observation", value.encode_to_vec())
        }
        P::AudioContinuityCommand(value) => ("audio_continuity_command", value.encode_to_vec()),
        P::RangingReport(value) => ("ranging_report", value.encode_to_vec()),
        P::ProximityState(value) => ("proximity_state", value.encode_to_vec()),
    }
}

pub fn envelope_to_control_frame(envelope: &v1::Envelope) -> Result<ControlFrame, KeynearError> {
    let payload = envelope
        .payload
        .as_ref()
        .ok_or_else(|| KeynearError::Protocol("envelope payload must not be empty".to_string()))?;
    let (kind, payload) = payload_kind_and_bytes(payload);

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

fn decode_payload(kind: &str, slice: &[u8]) -> Result<v1::envelope::Payload, KeynearError> {
    use v1::envelope::Payload as P;
    fn decode<T: prost::Message + Default>(slice: &[u8]) -> Result<T, KeynearError> {
        T::decode(slice).map_err(|err| KeynearError::Protocol(err.to_string()))
    }
    Ok(match kind {
        "handshake" => P::Handshake(decode(slice)?),
        "transfer_offer" => P::TransferOffer(decode(slice)?),
        "clipboard" => P::Clipboard(decode(slice)?),
        "audio_state" => P::AudioState(decode(slice)?),
        "control_ack" => P::ControlAck(decode(slice)?),
        "lan_endpoint" => P::LanEndpoint(decode(slice)?),
        "transfer_consent" => P::TransferConsent(decode(slice)?),
        "transfer_cancel" => P::TransferCancel(decode(slice)?),
        "transfer_complete" => P::TransferComplete(decode(slice)?),
        "clipboard_offer" => P::ClipboardOffer(decode(slice)?),
        "clipboard_fetch" => P::ClipboardFetch(decode(slice)?),
        "audio_handoff" => P::AudioHandoff(decode(slice)?),
        "capability_negotiation" => P::CapabilityNegotiation(decode(slice)?),
        "fallback_probe" => P::FallbackProbe(decode(slice)?),
        "transfer_checkpoint" => P::TransferCheckpoint(decode(slice)?),
        "transfer_resume_request" => P::TransferResumeRequest(decode(slice)?),
        "transfer_resume_ack" => P::TransferResumeAck(decode(slice)?),
        "mirrored_notification" => P::MirroredNotification(decode(slice)?),
        "notification_action" => P::NotificationAction(decode(slice)?),
        "call_alert" => P::CallAlert(decode(slice)?),
        "call_action" => P::CallAction(decode(slice)?),
        "media_session_state" => P::MediaSessionState(decode(slice)?),
        "media_command" => P::MediaCommand(decode(slice)?),
        "audio_continuity_observation" => P::AudioContinuityObservation(decode(slice)?),
        "audio_continuity_command" => P::AudioContinuityCommand(decode(slice)?),
        "ranging_report" => P::RangingReport(decode(slice)?),
        "proximity_state" => P::ProximityState(decode(slice)?),
        other => {
            return Err(KeynearError::Protocol(format!(
                "unknown control frame kind '{other}'"
            )))
        }
    })
}

fn decode_frame_payload(frame: &ControlFrame) -> Result<v1::envelope::Payload, KeynearError> {
    decode_payload(&frame.kind, frame.payload.as_slice())
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

pub fn validate_control_envelope(envelope: &v1::Envelope) -> Result<(), KeynearError> {
    let frame = envelope_to_control_frame(envelope)?;
    let bytes = encode_control_frame(&frame)?;
    let decoded = decode_control_frame(&bytes)?;
    control_frame_to_envelope(&decoded)?;
    Ok(())
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
