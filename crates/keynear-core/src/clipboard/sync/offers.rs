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

#[cfg(test)]
mod tests {
    use super::*;

    fn engine() -> ClipboardEngine {
        ClipboardEngine::new(ClipboardPolicy::default())
    }

    #[test]
    fn validate_rendered_payload_bounds_empty_and_oversized_data() {
        assert!(validate_rendered_payload(b"payload").is_ok());
        assert!(matches!(
            validate_rendered_payload(&[]),
            Err(KeynearError::Transfer(_))
        ));
        assert!(matches!(
            validate_rendered_payload(&vec![0u8; MAX_CLIPBOARD_IMAGE_BYTES + 1]),
            Err(KeynearError::FrameTooLarge { .. })
        ));
    }

    #[test]
    fn outgoing_offer_lookup_returns_metadata_and_rejects_unknown_ids() {
        let engine = engine();
        let offer = engine
            .register_delayed_offer(
                vec!["text/plain".to_string()],
                false,
                false,
                2,
                Arc::new(|_| Ok(b"ok".to_vec())),
            )
            .expect("register");
        let (mimes, _, is_image, is_otp) = engine
            .lookup_outgoing_offer(&offer.offer_id)
            .expect("lookup");
        assert_eq!(mimes, vec!["text/plain".to_string()]);
        assert!(!is_image && !is_otp);
        assert!(matches!(
            engine.lookup_outgoing_offer("missing"),
            Err(KeynearError::Transfer(_))
        ));
    }

    #[test]
    fn incoming_enqueue_queues_items_and_lookup_reports_unknown_offers() {
        let engine = engine();
        let item = ClipboardItem {
            id: "item-1".to_string(),
            peer_device_id: "phone-1".to_string(),
            mime_type: "text/plain".to_string(),
            data: b"hello".to_vec(),
            is_otp: false,
            is_image_offer: false,
            offer_id: String::new(),
            offered_mimes: Vec::new(),
        };
        engine.enqueue_incoming(item.clone());
        assert_eq!(engine.poll_incoming(), vec![item]);
        assert!(engine.lookup_incoming_offer("missing").is_none());

        let mut offers = engine
            .incoming_offers
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        offers.insert(
            "offer-1".to_string(),
            IncomingOfferEntry {
                peer_device_id: "phone-1".to_string(),
                mime_types: vec!["image/png".to_string()],
                is_image: true,
                is_otp: false,
                created_at: Instant::now(),
            },
        );
        drop(offers);
        let found = engine.lookup_incoming_offer("offer-1").expect("found");
        assert_eq!(found.0, "phone-1");
        assert_eq!(found.1, vec!["image/png".to_string()]);
        assert!(found.2);
        assert!(!found.3);
    }
}
