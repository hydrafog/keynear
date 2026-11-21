use super::*;

pub(super) async fn send_control_ack(
    send: &mut SendStream,
    acked_sequence: u64,
    accepted: bool,
    reason: &str,
) -> Result<(), KeynearError> {
    let env = v1::Envelope {
        version: 1,
        sequence: acked_sequence,
        timestamp_ms: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0),
        payload: Some(v1::envelope::Payload::ControlAck(v1::ControlAck {
            acked_sequence,
            accepted,
            reason: reason.to_string(),
        })),
    };
    write_control_envelope(send, &env).await?;
    send.finish()
        .map_err(|err| KeynearError::Transport(format!("finish control ack failed: {err}")))?;
    Ok(())
}

pub(super) async fn await_control_ack(recv: &mut RecvStream) -> Result<(), KeynearError> {
    let env = read_control_envelope(recv).await?;
    match env.payload {
        Some(v1::envelope::Payload::ControlAck(ack)) if ack.accepted => Ok(()),
        Some(v1::envelope::Payload::ControlAck(ack)) => Err(KeynearError::Transfer(ack.reason)),
        _ => Err(KeynearError::Protocol(
            "expected ControlAck on clipboard stream".to_string(),
        )),
    }
}
