use super::offers::validate_rendered_payload;
use super::*;

struct IncomingContext<'a> {
    peer_device_id: &'a str,
    send: &'a mut SendStream,
    sequence: u64,
}

impl ClipboardEngine {
    #[allow(clippy::too_many_arguments)]
    pub async fn handle_incoming_stream_with_kind(
        &self,
        peer_device_id: &str,
        kind: QuicStreamKind,
        mut send: SendStream,
        mut recv: RecvStream,
    ) -> Result<(), KeynearError> {
        match kind {
            QuicStreamKind::Control => {
                let env = read_control_envelope(&mut recv).await?;
                let mut ctx = IncomingContext {
                    peer_device_id,
                    send: &mut send,
                    sequence: env.sequence,
                };
                match env.payload {
                    Some(v1::envelope::Payload::Clipboard(payload)) => {
                        self.handle_clipboard(&mut ctx, payload).await
                    }
                    Some(v1::envelope::Payload::ClipboardOffer(offer)) => {
                        self.handle_offer(&mut ctx, offer).await
                    }
                    Some(v1::envelope::Payload::ClipboardFetch(fetch)) => {
                        self.handle_fetch(&mut ctx, fetch).await
                    }
                    _ => Err(KeynearError::Protocol(
                        "unexpected envelope payload on clipboard control stream".to_string(),
                    )),
                }
            }
            QuicStreamKind::ClipboardBlob => {
                self.handle_blob(peer_device_id, &mut send, &mut recv).await
            }
            _ => Err(KeynearError::Protocol(
                "unexpected stream kind on clipboard stream".to_string(),
            )),
        }
    }

    async fn handle_clipboard(
        &self,
        ctx: &mut IncomingContext<'_>,
        payload: v1::ClipboardPayload,
    ) -> Result<(), KeynearError> {
        let policy = self.policy();
        let valid_mime = match validate_mime_type(&payload.mime_type) {
            Ok(m) => m,
            Err(err) => {
                send_control_ack(ctx.send, ctx.sequence, false, &err.to_string()).await?;
                return Err(err);
            }
        };
        if let Err(err) = check_policy(
            &policy,
            std::slice::from_ref(&valid_mime),
            is_image_mime(&valid_mime),
            payload.is_otp,
        ) {
            send_control_ack(ctx.send, ctx.sequence, false, &err.to_string()).await?;
            return Ok(());
        }
        let invalid_size =
            payload.data.is_empty() || payload.data.len() > MAX_CLIPBOARD_IMAGE_BYTES;
        if invalid_size {
            send_control_ack(
                ctx.send,
                ctx.sequence,
                false,
                "invalid clipboard payload size",
            )
            .await?;
            return Err(KeynearError::FrameTooLarge {
                size: payload.data.len(),
                max: MAX_CLIPBOARD_IMAGE_BYTES,
            });
        }

        let item = ClipboardItem {
            id: Uuid::new_v4().to_string(),
            peer_device_id: ctx.peer_device_id.to_string(),
            mime_type: valid_mime,
            data: payload.data,
            is_otp: payload.is_otp,
            is_image_offer: false,
            offer_id: String::new(),
            offered_mimes: Vec::new(),
        };
        self.enqueue_incoming(item);
        send_control_ack(ctx.send, ctx.sequence, true, "").await
    }

    async fn handle_offer(
        &self,
        ctx: &mut IncomingContext<'_>,
        offer: v1::ClipboardOffer,
    ) -> Result<(), KeynearError> {
        if !offer_metadata_valid(&offer) {
            send_control_ack(ctx.send, ctx.sequence, false, "invalid clipboard offer").await?;
            return Err(KeynearError::Protocol(
                "invalid ClipboardOffer metadata".to_string(),
            ));
        }
        let clean_mimes = match clean_offer_mimes(&offer) {
            Ok(mimes) => mimes,
            Err(err) => {
                send_control_ack(ctx.send, ctx.sequence, false, &err.to_string()).await?;
                return Err(err);
            }
        };

        let policy = self.policy();
        if let Err(err) = check_policy(&policy, &clean_mimes, offer.is_image, offer.is_otp) {
            send_control_ack(ctx.send, ctx.sequence, false, &err.to_string()).await?;
            return Ok(());
        }

        let preferred_mime = pick_preferred_image_mime(&clean_mimes)
            .or_else(|| clean_mimes.first().cloned())
            .unwrap_or_else(|| DEFAULT_TEXT_MIME.to_string());
        self.insert_incoming_offer(ctx.peer_device_id, &offer, &clean_mimes);

        let item = ClipboardItem {
            id: offer.offer_id.clone(),
            peer_device_id: ctx.peer_device_id.to_string(),
            mime_type: preferred_mime,
            data: Vec::new(),
            is_otp: offer.is_otp,
            is_image_offer: true,
            offer_id: offer.offer_id,
            offered_mimes: clean_mimes,
        };
        self.enqueue_incoming(item);
        send_control_ack(ctx.send, ctx.sequence, true, "").await
    }

    fn insert_incoming_offer(
        &self,
        peer_device_id: &str,
        offer: &v1::ClipboardOffer,
        clean_mimes: &[String],
    ) {
        let mut offers = self
            .incoming_offers
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        prune_incoming_offers(&mut offers, Instant::now());
        if offers.len() >= MAX_INCOMING_OFFERS {
            if let Some(oldest_key) = offers
                .iter()
                .min_by_key(|(_, v)| v.created_at)
                .map(|(k, _)| k.clone())
            {
                offers.remove(&oldest_key);
            }
        }
        offers.insert(
            offer.offer_id.clone(),
            IncomingOfferEntry {
                peer_device_id: peer_device_id.to_string(),
                mime_types: clean_mimes.to_vec(),
                is_image: offer.is_image,
                is_otp: offer.is_otp,
                created_at: Instant::now(),
            },
        );
    }

    async fn handle_fetch(
        &self,
        ctx: &mut IncomingContext<'_>,
        fetch: v1::ClipboardFetch,
    ) -> Result<(), KeynearError> {
        let result = self.render_fetch_reply(&fetch).await;
        self.write_fetch_reply(ctx, result).await
    }

    async fn render_fetch_reply(
        &self,
        fetch: &v1::ClipboardFetch,
    ) -> Result<(String, Vec<u8>, bool), KeynearError> {
        let (mime_types, render, is_image, is_otp) = self.lookup_outgoing_offer(&fetch.offer_id)?;
        let policy = self.policy();
        check_policy(&policy, &mime_types, is_image, is_otp)?;

        let chosen_mime = choose_fetch_mime(&mime_types, &fetch.mime_type).ok_or_else(|| {
            KeynearError::Validation {
                field: "mimeType",
                message: "requested MIME type not offered".to_string(),
            }
        })?;
        check_policy(
            &policy,
            std::slice::from_ref(&chosen_mime),
            is_image,
            is_otp,
        )?;

        let rendered = tokio::task::spawn_blocking(move || {
            render(&chosen_mime).map(|bytes| (chosen_mime, bytes, is_otp))
        })
        .await
        .map_err(|err| KeynearError::Transfer(format!("clipboard render task failed: {err}")))??;
        validate_rendered_payload(&rendered.1)?;
        Ok(rendered)
    }

    async fn write_fetch_reply(
        &self,
        ctx: &mut IncomingContext<'_>,
        result: Result<(String, Vec<u8>, bool), KeynearError>,
    ) -> Result<(), KeynearError> {
        match result {
            Ok((resolved_mime, data, is_otp)) => {
                if data.len() <= MAX_INLINE_CLIPBOARD_BYTES {
                    write_stream_kind(ctx.send, QuicStreamKind::Control).await?;
                    let reply_env =
                        self.wrap_payload(v1::envelope::Payload::Clipboard(v1::ClipboardPayload {
                            mime_type: resolved_mime,
                            data,
                            is_otp,
                        }));
                    write_control_envelope(ctx.send, &reply_env).await?;
                } else {
                    write_stream_kind(ctx.send, QuicStreamKind::ClipboardBlob).await?;
                    write_clipboard_blob(ctx.send, &resolved_mime, &data, is_otp).await?;
                }
                ctx.send.finish().map_err(|err| {
                    KeynearError::Transport(format!("finish fetch reply failed: {err}"))
                })?;
                Ok(())
            }
            Err(err) => {
                write_stream_kind(ctx.send, QuicStreamKind::Control).await?;
                send_control_ack(ctx.send, ctx.sequence, false, &err.to_string()).await?;
                Err(err)
            }
        }
    }

    async fn handle_blob(
        &self,
        peer_device_id: &str,
        send: &mut SendStream,
        recv: &mut RecvStream,
    ) -> Result<(), KeynearError> {
        let (mime_type, data, is_otp) = read_clipboard_blob(recv).await?;
        let policy = self.policy();
        if let Err(err) = check_policy(
            &policy,
            std::slice::from_ref(&mime_type),
            is_image_mime(&mime_type),
            is_otp,
        ) {
            send_control_ack(send, 1, false, &err.to_string()).await?;
            return Ok(());
        }
        let item = ClipboardItem {
            id: Uuid::new_v4().to_string(),
            peer_device_id: peer_device_id.to_string(),
            mime_type,
            data,
            is_otp,
            is_image_offer: false,
            offer_id: String::new(),
            offered_mimes: Vec::new(),
        };
        self.enqueue_incoming(item);
        send_control_ack(send, 1, true, "").await
    }
}
