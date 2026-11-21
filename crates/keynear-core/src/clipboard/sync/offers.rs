use super::*;

impl ClipboardEngine {
    #[allow(clippy::too_many_arguments)]
    pub fn register_delayed_offer(
        &self,
        mime_types: Vec<String>,
        is_image: bool,
        is_otp: bool,
        byte_size: u64,
        render: DelayedRenderCallback,
    ) -> Result<DelayedClipboardOffer, KeynearError> {
        if mime_types.is_empty() || mime_types.len() > MAX_OFFER_MIME_COUNT {
            return Err(KeynearError::Validation {
                field: "mimeTypes",
                message: format!("MIME list must contain 1..={MAX_OFFER_MIME_COUNT} entries"),
            });
        }
        let cleaned: Vec<String> = mime_types
            .iter()
            .map(|m| validate_mime_type(m))
            .collect::<Result<Vec<_>, _>>()?;

        let policy = self.policy();
        check_policy(&policy, &cleaned, is_image, is_otp)?;

        let offer_id = Uuid::new_v4().to_string();
        let entry = OutgoingOfferEntry {
            mime_types: cleaned.clone(),
            render,
            created_at: Instant::now(),
            is_image,
            is_otp,
        };

        {
            let mut offers = self
                .outgoing_offers
                .lock()
                .unwrap_or_else(|p| p.into_inner());
            prune_outgoing_offers(&mut offers, Instant::now());
            if offers.len() >= MAX_OUTGOING_OFFERS {
                if let Some(oldest_key) = offers
                    .iter()
                    .min_by_key(|(_, v)| v.created_at)
                    .map(|(k, _)| k.clone())
                {
                    offers.remove(&oldest_key);
                }
            }
            offers.insert(offer_id.clone(), entry);
        }

        Ok(DelayedClipboardOffer {
            offer_id,
            mime_types: cleaned,
            byte_size,
            is_image,
            is_otp,
        })
    }

    pub fn render_outgoing_offer(
        &self,
        offer_id: &str,
        requested_mime: &str,
    ) -> Result<(String, Vec<u8>, bool), KeynearError> {
        let policy = self.policy();
        let (mime_types, render, is_image, is_otp) = self.lookup_outgoing_offer(offer_id)?;
        check_policy(&policy, &mime_types, is_image, is_otp)?;

        let chosen_mime = select_offer_mime(&mime_types, requested_mime, is_image)?;
        check_policy(
            &policy,
            std::slice::from_ref(&chosen_mime),
            is_image,
            is_otp,
        )?;

        let data = render(&chosen_mime)?;
        validate_rendered_payload(&data)?;
        Ok((chosen_mime, data, is_otp))
    }

    pub(super) fn lookup_outgoing_offer(
        &self,
        offer_id: &str,
    ) -> Result<(Vec<String>, DelayedRenderCallback, bool, bool), KeynearError> {
        let mut offers = self
            .outgoing_offers
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        prune_outgoing_offers(&mut offers, Instant::now());
        let entry = offers.get(offer_id).ok_or_else(|| {
            KeynearError::Transfer("clipboard offer expired or unknown".to_string())
        })?;
        Ok((
            entry.mime_types.clone(),
            Arc::clone(&entry.render),
            entry.is_image,
            entry.is_otp,
        ))
    }

    pub fn lookup_incoming_offer(
        &self,
        offer_id: &str,
    ) -> Option<(String, Vec<String>, bool, bool)> {
        let mut offers = self
            .incoming_offers
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        prune_incoming_offers(&mut offers, Instant::now());
        offers.get(offer_id).map(|e| {
            (
                e.peer_device_id.clone(),
                e.mime_types.clone(),
                e.is_image,
                e.is_otp,
            )
        })
    }

    pub(super) fn enqueue_incoming(&self, item: ClipboardItem) {
        if let Ok(guard) = self.incoming_tx.lock() {
            if let Some(ref tx) = *guard {
                let _ = tx.send(item);
                return;
            }
        }
        let mut queue = self
            .incoming_queue
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        if queue.len() >= MAX_CLIPBOARD_QUEUE {
            queue.pop_front();
        }
        queue.push_back(item);
    }

    pub(super) fn wrap_payload(&self, payload: v1::envelope::Payload) -> v1::Envelope {
        let sequence = self.sequence.fetch_add(1, Ordering::Relaxed);
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        v1::Envelope {
            version: 1,
            sequence,
            timestamp_ms,
            payload: Some(payload),
        }
    }
}

fn select_offer_mime(
    mime_types: &[String],
    requested_mime: &str,
    is_image: bool,
) -> Result<String, KeynearError> {
    if requested_mime.trim().is_empty() {
        if is_image {
            pick_preferred_image_mime(mime_types)
                .or_else(|| mime_types.first().cloned())
                .ok_or_else(|| KeynearError::Validation {
                    field: "mimeType",
                    message: "no MIME type available for offer".to_string(),
                })
        } else {
            mime_types
                .first()
                .cloned()
                .ok_or_else(|| KeynearError::Validation {
                    field: "mimeType",
                    message: "no MIME type available for offer".to_string(),
                })
        }
    } else {
        let req_norm = normalize_mime_type(requested_mime);
        mime_types
            .iter()
            .find(|m| normalize_mime_type(m) == req_norm)
            .cloned()
            .ok_or_else(|| KeynearError::Validation {
                field: "mimeType",
                message: format!("requested MIME '{requested_mime}' was not offered"),
            })
    }
}

pub(super) fn validate_rendered_payload(data: &[u8]) -> Result<(), KeynearError> {
    if data.is_empty() {
        return Err(KeynearError::Transfer(
            "rendered clipboard payload is empty".to_string(),
        ));
    }
    if data.len() > MAX_CLIPBOARD_IMAGE_BYTES {
        return Err(KeynearError::FrameTooLarge {
            size: data.len(),
            max: MAX_CLIPBOARD_IMAGE_BYTES,
        });
    }
    Ok(())
}
