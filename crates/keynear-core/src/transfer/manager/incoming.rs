use super::*;

use tokio::sync::oneshot;

struct IncomingContext<'a> {
    send: &'a mut SendStream,
    offer: &'a v1::TransferOffer,
}

impl TransferEngine {
    pub async fn handle_incoming_connection(
        &self,
        peer_device_id: String,
        conn: Connection,
    ) -> Result<(), KeynearError> {
        loop {
            let Ok((send, recv)) = conn.accept_bi().await else {
                break;
            };
            self.handle_stream_logged(&peer_device_id, send, recv).await;
        }
        Ok(())
    }

    async fn handle_stream_logged(&self, peer_device_id: &str, send: SendStream, recv: RecvStream) {
        if let Err(err) = self
            .handle_incoming_stream(peer_device_id, send, recv)
            .await
        {
            warn!(
                event = "incoming_transfer_failed",
                peer_device_id = %peer_device_id,
                error = %err
            );
        }
    }

    pub async fn handle_incoming_stream(
        &self,
        peer_device_id: &str,
        send: SendStream,
        mut recv: RecvStream,
    ) -> Result<TransferSession, KeynearError> {
        let kind = read_stream_kind(&mut recv).await?;
        self.handle_incoming_stream_with_kind(peer_device_id, kind, send, recv)
            .await
    }

    fn requires_explicit_fd(&self, kind: QuicStreamKind) -> bool {
        kind == QuicStreamKind::RawFileChunk
            && self
                .require_explicit_fd_for_single_file
                .load(Ordering::Acquire)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn handle_incoming_stream_with_kind(
        &self,
        peer_device_id: &str,
        kind: QuicStreamKind,
        mut send: SendStream,
        mut recv: RecvStream,
    ) -> Result<TransferSession, KeynearError> {
        require_supported_kind(kind)?;

        let env = read_control_envelope(&mut recv).await?;
        let offer = extract_offer(env.payload)?;
        let auto_accept = self.is_auto_accept_allowed(peer_device_id);
        let require_fd = self.requires_explicit_fd(kind);

        let reg = self.registry.register_inbound(
            peer_device_id.to_string(),
            &offer,
            auto_accept,
            require_fd,
        )?;
        let mut ctx = IncomingContext {
            send: &mut send,
            offer: &offer,
        };
        self.await_consent(&mut ctx, reg.consent_rx).await?;
        let explicit_file = self.await_explicit_file(&mut ctx, reg.fd_rx).await?;
        send_consent_frame(&mut *ctx.send, &offer.transfer_id, true, "accepted").await?;

        let result = match kind {
            QuicStreamKind::RawFileChunk => {
                receive_raw_file_stream(
                    &self.registry,
                    self.incoming_directory(),
                    &offer,
                    &mut recv,
                    explicit_file,
                    &reg.cancel_flag,
                )
                .await
            }
            QuicStreamKind::StoredZipArchive => {
                receive_zip_stream(
                    &self.registry,
                    self.incoming_directory(),
                    &offer,
                    &mut recv,
                    &reg.cancel_flag,
                )
                .await
            }
            QuicStreamKind::Control | QuicStreamKind::ClipboardBlob => unreachable!(),
        };

        self.finish_incoming(&mut ctx, result).await
    }

    async fn await_consent(
        &self,
        ctx: &mut IncomingContext<'_>,
        consent_rx: Option<oneshot::Receiver<bool>>,
    ) -> Result<(), KeynearError> {
        let Some(consent_rx) = consent_rx else {
            return Ok(());
        };
        let accepted =
            match tokio::time::timeout(Duration::from_secs(CONSENT_TIMEOUT_SECS), consent_rx).await
            {
                Ok(Ok(true)) => true,
                Ok(Ok(false)) | Ok(Err(_)) | Err(_) => false,
            };
        if !accepted {
            decline_transfer(self, ctx, "declined").await;
            return Err(KeynearError::TransferCancelled(
                "incoming transfer declined or timed out".to_string(),
            ));
        }
        Ok(())
    }

    async fn await_explicit_file(
        &self,
        ctx: &mut IncomingContext<'_>,
        fd_rx: Option<oneshot::Receiver<File>>,
    ) -> Result<Option<File>, KeynearError> {
        let Some(fd_rx) = fd_rx else {
            return Ok(None);
        };
        match tokio::time::timeout(Duration::from_secs(CONSENT_TIMEOUT_SECS), fd_rx).await {
            Ok(Ok(file)) => Ok(Some(file)),
            _ => {
                decline_transfer(self, ctx, "no_output_fd").await;
                Err(KeynearError::TransferCancelled(
                    "no output file descriptor supplied".to_string(),
                ))
            }
        }
    }

    async fn finish_incoming(
        &self,
        ctx: &mut IncomingContext<'_>,
        result: Result<(u64, Vec<String>, String), KeynearError>,
    ) -> Result<TransferSession, KeynearError> {
        match result {
            Ok(values) => self.finish_incoming_ok(ctx, values).await,
            Err(KeynearError::TransferCancelled(reason)) => {
                let _ = send_cancel_frame(ctx.send, &ctx.offer.transfer_id, &reason).await;
                let _ = ctx.send.finish();
                self.registry.finish_transfer(
                    &ctx.offer.transfer_id,
                    TransferState::Cancelled,
                    0,
                    None,
                    None,
                );
                Err(KeynearError::TransferCancelled(reason))
            }
            Err(err) => {
                let _ = send_cancel_frame(ctx.send, &ctx.offer.transfer_id, &err.to_string()).await;
                let _ = ctx.send.finish();
                self.registry.finish_transfer(
                    &ctx.offer.transfer_id,
                    TransferState::Failed,
                    0,
                    None,
                    None,
                );
                Err(err)
            }
        }
    }

    async fn finish_incoming_ok(
        &self,
        ctx: &mut IncomingContext<'_>,
        values: (u64, Vec<String>, String),
    ) -> Result<TransferSession, KeynearError> {
        let (bytes_received, target_paths, digest_hex) = values;
        let ack = wrap_payload(v1::envelope::Payload::ControlAck(v1::ControlAck {
            acked_sequence: 1,
            accepted: true,
            reason: String::new(),
        }));
        write_control_envelope(ctx.send, &ack).await?;
        let _ = ctx.send.finish();
        self.registry.finish_transfer(
            &ctx.offer.transfer_id,
            TransferState::Completed,
            bytes_received,
            Some(&target_paths),
            Some(&digest_hex),
        );
        info!(
            event = "incoming_transfer_completed",
            transfer_id = %ctx.offer.transfer_id,
            bytes = bytes_received
        );
        self.registry
            .get_session(&ctx.offer.transfer_id)
            .ok_or_else(|| KeynearError::Transfer("completed transfer session missing".to_string()))
    }
}

fn require_supported_kind(kind: QuicStreamKind) -> Result<(), KeynearError> {
    let supported = matches!(
        kind,
        QuicStreamKind::RawFileChunk | QuicStreamKind::StoredZipArchive
    );
    if !supported {
        return Err(KeynearError::Protocol(
            "unexpected stream kind on incoming transfer stream".to_string(),
        ));
    }
    Ok(())
}

fn extract_offer(
    payload: Option<v1::envelope::Payload>,
) -> Result<v1::TransferOffer, KeynearError> {
    let Some(v1::envelope::Payload::TransferOffer(offer)) = payload else {
        return Err(KeynearError::Protocol(
            "expected TransferOffer on incoming transfer stream".to_string(),
        ));
    };
    Ok(offer)
}

async fn decline_transfer(engine: &TransferEngine, ctx: &mut IncomingContext<'_>, reason: &str) {
    engine.registry.finish_transfer(
        &ctx.offer.transfer_id,
        TransferState::Cancelled,
        0,
        None,
        None,
    );
    let _ = send_consent_frame(ctx.send, &ctx.offer.transfer_id, false, reason).await;
    let _ = ctx.send.finish();
}
