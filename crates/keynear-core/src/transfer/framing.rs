use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use iroh::endpoint::{RecvStream, SendStream};
use subtle::ConstantTimeEq;
use tokio::sync::mpsc;

use keynear_protocol::{
    read_control_envelope, v1, write_control_envelope, KeynearError, STREAM_CHUNK_BYTES,
};

use super::io::sanitize_relative_path;
use super::state::now_millis;

pub(crate) const CONSENT_TIMEOUT_SECS: u64 = 120;
pub(crate) const PIPE_CHANNEL_SLOTS: usize = 8;

pub(crate) fn validate_offer(offer: &v1::TransferOffer) -> Result<(), KeynearError> {
    if offer.transfer_id.trim().is_empty() || offer.transfer_id.len() > 64 {
        return Err(KeynearError::Validation {
            field: "transferId",
            message: "transfer_id must be 1..64 bytes".to_string(),
        });
    }
    if offer.items.is_empty() {
        return Err(KeynearError::Validation {
            field: "items",
            message: "transfer offer must contain at least one item".to_string(),
        });
    }
    let mut sum: u64 = 0;
    for item in &offer.items {
        sanitize_relative_path(&item.name)?;
        sum = sum
            .checked_add(item.size_bytes)
            .ok_or_else(|| KeynearError::Transfer("transfer offer size overflow".to_string()))?;
    }
    if sum != offer.total_bytes {
        return Err(KeynearError::Validation {
            field: "totalBytes",
            message: "offer total_bytes does not match item sum".to_string(),
        });
    }
    Ok(())
}

pub(crate) async fn write_data_chunk(
    send: &mut SendStream,
    chunk: &[u8],
) -> Result<(), KeynearError> {
    let len = u32::try_from(chunk.len())
        .map_err(|_| KeynearError::Transfer("data chunk exceeds u32 length".to_string()))?;
    if len == 0 || (len as usize) > STREAM_CHUNK_BYTES {
        return Err(KeynearError::Transfer(
            "data chunk length out of bounds".to_string(),
        ));
    }
    send.write_all(&len.to_be_bytes())
        .await
        .map_err(|err| KeynearError::Transport(format!("write chunk header failed: {err}")))?;
    send.write_all(chunk)
        .await
        .map_err(|err| KeynearError::Transport(format!("write chunk body failed: {err}")))?;
    Ok(())
}

pub(crate) async fn read_data_chunk(
    recv: &mut RecvStream,
) -> Result<Option<Vec<u8>>, KeynearError> {
    let mut len_buf = [0u8; 4];
    recv.read_exact(&mut len_buf)
        .await
        .map_err(|err| KeynearError::Transport(format!("read chunk header failed: {err}")))?;
    let len = u32::from_be_bytes(len_buf) as usize;
    if len == 0 {
        return Ok(None);
    }
    if len > STREAM_CHUNK_BYTES {
        return Err(KeynearError::Transfer(format!(
            "incoming chunk length {len} exceeds maximum {STREAM_CHUNK_BYTES}"
        )));
    }
    let mut buf = vec![0u8; len];
    recv.read_exact(&mut buf)
        .await
        .map_err(|err| KeynearError::Transport(format!("read chunk body failed: {err}")))?;
    Ok(Some(buf))
}

pub(crate) struct CompletionDetails<'a> {
    pub transfer_id: &'a str,
    pub bytes_transferred: u64,
    pub digest: &'a [u8; 32],
}

pub(crate) async fn send_completion_and_wait_ack(
    send: &mut SendStream,
    recv: &mut RecvStream,
    completion: CompletionDetails<'_>,
) -> Result<(), KeynearError> {
    send.write_all(&0u32.to_be_bytes())
        .await
        .map_err(|err| KeynearError::Transport(format!("write EOF marker failed: {err}")))?;
    let complete = wrap_payload(v1::envelope::Payload::TransferComplete(
        v1::TransferComplete {
            transfer_id: completion.transfer_id.to_string(),
            bytes_transferred: completion.bytes_transferred,
            sha256_digest: completion.digest.to_vec(),
        },
    ));
    write_control_envelope(send, &complete).await?;
    send.finish()
        .map_err(|err| KeynearError::Transport(format!("finish send stream failed: {err}")))?;

    let reply = read_control_envelope(recv).await?;
    match reply.payload {
        Some(v1::envelope::Payload::ControlAck(ack)) if ack.accepted => Ok(()),
        Some(v1::envelope::Payload::ControlAck(ack)) => Err(KeynearError::Transfer(ack.reason)),
        Some(v1::envelope::Payload::TransferCancel(cancel)) => {
            Err(KeynearError::TransferCancelled(cancel.reason))
        }
        _ => Err(KeynearError::Protocol(
            "expected ControlAck after TransferComplete".to_string(),
        )),
    }
}

pub(crate) async fn verify_completion_envelope(
    recv: &mut RecvStream,
    transfer_id: &str,
    expected_bytes: u64,
    expected_digest: &[u8; 32],
) -> Result<(), KeynearError> {
    let env = read_control_envelope(recv).await?;
    let Some(v1::envelope::Payload::TransferComplete(complete)) = env.payload else {
        return Err(KeynearError::Protocol(
            "expected TransferComplete envelope at end of stream".to_string(),
        ));
    };
    if complete.transfer_id != transfer_id {
        return Err(KeynearError::Protocol(
            "TransferComplete transfer_id mismatch".to_string(),
        ));
    }
    if complete.bytes_transferred != expected_bytes {
        return Err(KeynearError::Transfer(format!(
            "transferred byte count {} does not match sender {}",
            expected_bytes, complete.bytes_transferred
        )));
    }
    if complete.sha256_digest.len() != 32
        || !bool::from(complete.sha256_digest.ct_eq(expected_digest))
    {
        return Err(KeynearError::Crypto(
            "transfer SHA-256 verification failed".to_string(),
        ));
    }
    Ok(())
}

pub(crate) async fn await_transfer_consent(
    recv: &mut RecvStream,
    transfer_id: &str,
) -> Result<(), KeynearError> {
    let env = read_control_envelope(recv).await?;
    match env.payload {
        Some(v1::envelope::Payload::TransferConsent(consent)) => {
            if consent.transfer_id != transfer_id {
                return Err(KeynearError::Protocol(
                    "TransferConsent transfer_id mismatch".to_string(),
                ));
            }
            if consent.accepted {
                Ok(())
            } else {
                Err(KeynearError::TransferCancelled(consent.reason))
            }
        }
        Some(v1::envelope::Payload::TransferCancel(cancel)) => {
            Err(KeynearError::TransferCancelled(cancel.reason))
        }
        _ => Err(KeynearError::Protocol(
            "expected TransferConsent response".to_string(),
        )),
    }
}

pub(crate) async fn send_consent_frame(
    send: &mut SendStream,
    transfer_id: &str,
    accepted: bool,
    reason: &str,
) -> Result<(), KeynearError> {
    let env = wrap_payload(v1::envelope::Payload::TransferConsent(
        v1::TransferConsent {
            transfer_id: transfer_id.to_string(),
            accepted,
            reason: reason.to_string(),
        },
    ));
    write_control_envelope(send, &env).await
}

pub(crate) async fn send_cancel_frame(
    send: &mut SendStream,
    transfer_id: &str,
    reason: &str,
) -> Result<(), KeynearError> {
    let env = wrap_payload(v1::envelope::Payload::TransferCancel(v1::TransferCancel {
        transfer_id: transfer_id.to_string(),
        reason: reason.to_string(),
    }));
    write_control_envelope(send, &env).await
}

pub(crate) fn wrap_payload(payload: v1::envelope::Payload) -> v1::Envelope {
    v1::Envelope {
        version: 1,
        sequence: 1,
        timestamp_ms: now_millis(),
        payload: Some(payload),
    }
}

pub(crate) struct ChannelChunkWriter {
    tx: mpsc::Sender<Result<Vec<u8>, KeynearError>>,
    cancel_flag: Arc<AtomicBool>,
}

impl ChannelChunkWriter {
    pub(crate) fn new(
        tx: mpsc::Sender<Result<Vec<u8>, KeynearError>>,
        cancel_flag: Arc<AtomicBool>,
    ) -> Self {
        Self { tx, cancel_flag }
    }
}

impl Write for ChannelChunkWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if self.cancel_flag.load(Ordering::Acquire) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Interrupted,
                "transfer cancelled",
            ));
        }
        if buf.is_empty() {
            return Ok(0);
        }
        for slice in buf.chunks(STREAM_CHUNK_BYTES) {
            self.tx.blocking_send(Ok(slice.to_vec())).map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::BrokenPipe, "receiver closed")
            })?;
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
