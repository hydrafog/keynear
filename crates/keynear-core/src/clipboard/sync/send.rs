use super::*;

impl ClipboardEngine {
    #[allow(clippy::too_many_arguments)]
    pub async fn send_clipboard_text(
        &self,
        node: &OrionQuicNode,
        peer_device_id: &str,
        text: &str,
        is_otp: bool,
    ) -> Result<(), KeynearError> {
        if text.is_empty() {
            return Err(KeynearError::Validation {
                field: "text",
                message: "clipboard text must not be empty".to_string(),
            });
        }
        self.send_clipboard_payload(
            node,
            peer_device_id,
            DEFAULT_TEXT_MIME,
            text.as_bytes(),
            is_otp,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn send_clipboard_payload(
        &self,
        node: &OrionQuicNode,
        peer_device_id: &str,
        mime_type: &str,
        data: &[u8],
        is_otp: bool,
    ) -> Result<(), KeynearError> {
        let clean_mime = validate_mime_type(mime_type)?;
        let is_image = is_image_mime(&clean_mime);
        let policy = self.policy();
        check_policy(&policy, std::slice::from_ref(&clean_mime), is_image, is_otp)?;

        if data.is_empty() {
            return Err(KeynearError::Validation {
                field: "data",
                message: "clipboard payload must not be empty".to_string(),
            });
        }
        if data.len() > MAX_CLIPBOARD_IMAGE_BYTES {
            return Err(KeynearError::FrameTooLarge {
                size: data.len(),
                max: MAX_CLIPBOARD_IMAGE_BYTES,
            });
        }

        let conn = node.connect_peer(peer_device_id).await?;
        let (mut send, mut recv) = conn.open_bi().await.map_err(|err| {
            KeynearError::Transport(format!("open clipboard stream failed: {err}"))
        })?;

        if data.len() <= MAX_INLINE_CLIPBOARD_BYTES {
            write_stream_kind(&mut send, QuicStreamKind::Control).await?;
            let env = self.wrap_payload(v1::envelope::Payload::Clipboard(v1::ClipboardPayload {
                mime_type: clean_mime,
                data: data.to_vec(),
                is_otp,
            }));
            write_control_envelope(&mut send, &env).await?;
        } else {
            write_stream_kind(&mut send, QuicStreamKind::ClipboardBlob).await?;
            write_clipboard_blob(&mut send, &clean_mime, data, is_otp).await?;
        }
        send.finish().map_err(|err| {
            KeynearError::Transport(format!("finish clipboard stream failed: {err}"))
        })?;

        await_control_ack(&mut recv).await
    }

    pub async fn send_delayed_offer(
        &self,
        node: &OrionQuicNode,
        peer_device_id: &str,
        offer: &DelayedClipboardOffer,
    ) -> Result<(), KeynearError> {
        let policy = self.policy();
        check_policy(&policy, &offer.mime_types, offer.is_image, offer.is_otp)?;

        let conn = node.connect_peer(peer_device_id).await?;
        let (mut send, mut recv) = conn
            .open_bi()
            .await
            .map_err(|err| KeynearError::Transport(format!("open offer stream failed: {err}")))?;

        write_stream_kind(&mut send, QuicStreamKind::Control).await?;
        let env = self.wrap_payload(v1::envelope::Payload::ClipboardOffer(v1::ClipboardOffer {
            offer_id: offer.offer_id.clone(),
            mime_types: offer.mime_types.clone(),
            is_image: offer.is_image,
            is_otp: offer.is_otp,
            byte_size: offer.byte_size,
        }));
        write_control_envelope(&mut send, &env).await?;
        send.finish()
            .map_err(|err| KeynearError::Transport(format!("finish offer stream failed: {err}")))?;

        await_control_ack(&mut recv).await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn offer_clipboard_image(
        &self,
        node: &OrionQuicNode,
        peer_device_id: &str,
        mime_types: Vec<String>,
        render: DelayedRenderCallback,
    ) -> Result<DelayedClipboardOffer, KeynearError> {
        let offer = self.register_delayed_offer(mime_types, true, false, 0, render)?;
        self.send_delayed_offer(node, peer_device_id, &offer)
            .await?;
        Ok(offer)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn fetch_clipboard_image(
        &self,
        node: &OrionQuicNode,
        peer_device_id: &str,
        offer_id: &str,
        mime_type: &str,
    ) -> Result<ClipboardItem, KeynearError> {
        let clean_offer_id = validate_fetch_offer_id(offer_id)?;
        let clean_mime = self.prepare_fetch_mime(mime_type)?;

        let conn = node.connect_peer(peer_device_id).await?;
        let (mut send, mut recv) = conn
            .open_bi()
            .await
            .map_err(|err| KeynearError::Transport(format!("open fetch stream failed: {err}")))?;

        write_stream_kind(&mut send, QuicStreamKind::Control).await?;
        let env = self.wrap_payload(v1::envelope::Payload::ClipboardFetch(v1::ClipboardFetch {
            offer_id: clean_offer_id.to_string(),
            mime_type: clean_mime,
        }));
        write_control_envelope(&mut send, &env).await?;
        send.finish()
            .map_err(|err| KeynearError::Transport(format!("finish fetch stream failed: {err}")))?;

        let (resolved_mime, data, is_otp) = read_fetch_reply(&mut recv).await?;

        let policy_now = self.policy();
        check_policy(
            &policy_now,
            std::slice::from_ref(&resolved_mime),
            is_image_mime(&resolved_mime),
            is_otp,
        )?;

        Ok(ClipboardItem {
            id: clean_offer_id.to_string(),
            peer_device_id: peer_device_id.to_string(),
            mime_type: resolved_mime,
            data,
            is_otp,
            is_image_offer: false,
            offer_id: clean_offer_id.to_string(),
            offered_mimes: Vec::new(),
        })
    }

    fn prepare_fetch_mime(&self, mime_type: &str) -> Result<String, KeynearError> {
        let clean_mime = if mime_type.trim().is_empty() {
            String::new()
        } else {
            validate_mime_type(mime_type)?
        };
        let policy = self.policy();
        if !clean_mime.is_empty() {
            check_policy(
                &policy,
                std::slice::from_ref(&clean_mime),
                is_image_mime(&clean_mime),
                false,
            )?;
        } else if !policy.clipboard_sync_enabled || !policy.clipboard_images_enabled {
            return Err(KeynearError::Validation {
                field: "clipboardImagesEnabled",
                message: "clipboard image sync is disabled by policy".to_string(),
            });
        }
        Ok(clean_mime)
    }
}

fn validate_fetch_offer_id(offer_id: &str) -> Result<&str, KeynearError> {
    let clean_offer_id = offer_id.trim();
    if clean_offer_id.is_empty() {
        return Err(KeynearError::Validation {
            field: "offerId",
            message: "offer_id must not be empty".to_string(),
        });
    }
    Ok(clean_offer_id)
}

async fn read_fetch_reply(recv: &mut RecvStream) -> Result<(String, Vec<u8>, bool), KeynearError> {
    let reply_kind = read_stream_kind(recv).await?;
    match reply_kind {
        QuicStreamKind::Control => {
            let reply = read_control_envelope(recv).await?;
            match reply.payload {
                Some(v1::envelope::Payload::Clipboard(payload)) => {
                    let valid_mime = validate_mime_type(&payload.mime_type)?;
                    if payload.data.len() > MAX_CLIPBOARD_IMAGE_BYTES {
                        return Err(KeynearError::FrameTooLarge {
                            size: payload.data.len(),
                            max: MAX_CLIPBOARD_IMAGE_BYTES,
                        });
                    }
                    Ok((valid_mime, payload.data, payload.is_otp))
                }
                Some(v1::envelope::Payload::ControlAck(ack)) if !ack.accepted => {
                    Err(KeynearError::Transfer(ack.reason))
                }
                _ => Err(KeynearError::Protocol(
                    "unexpected response payload for ClipboardFetch".to_string(),
                )),
            }
        }
        QuicStreamKind::ClipboardBlob => read_clipboard_blob(recv).await,
        _ => Err(KeynearError::Protocol(
            "unexpected stream kind in ClipboardFetch reply".to_string(),
        )),
    }
}
