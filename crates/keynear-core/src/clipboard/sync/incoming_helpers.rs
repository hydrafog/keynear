use super::*;

pub(super) fn offer_metadata_valid(offer: &v1::ClipboardOffer) -> bool {
    !offer.offer_id.trim().is_empty()
        && !offer.mime_types.is_empty()
        && offer.mime_types.len() <= MAX_OFFER_MIME_COUNT
}

pub(super) fn clean_offer_mimes(offer: &v1::ClipboardOffer) -> Result<Vec<String>, KeynearError> {
    let mut clean_mimes = Vec::with_capacity(offer.mime_types.len());
    for mime in &offer.mime_types {
        clean_mimes.push(validate_mime_type(mime)?);
    }
    Ok(clean_mimes)
}

pub(super) fn choose_fetch_mime(mime_types: &[String], mime_type: &str) -> Option<String> {
    if mime_type.trim().is_empty() {
        pick_preferred_image_mime(mime_types).or_else(|| mime_types.first().cloned())
    } else {
        let req_norm = normalize_mime_type(mime_type);
        mime_types
            .iter()
            .find(|m| normalize_mime_type(m) == req_norm)
            .cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn offer(id: &str, mimes: &[&str]) -> v1::ClipboardOffer {
        v1::ClipboardOffer {
            offer_id: id.to_string(),
            mime_types: mimes.iter().map(|m| m.to_string()).collect(),
            ..Default::default()
        }
    }

    #[test]
    fn offer_metadata_valid_rejects_blank_id_empty_and_overlimit_mimes() {
        assert!(!offer_metadata_valid(&offer("   ", &["text/plain"])));
        assert!(!offer_metadata_valid(&offer("offer-1", &[])));
        let oversized = v1::ClipboardOffer {
            offer_id: "offer-1".to_string(),
            mime_types: (0..=MAX_OFFER_MIME_COUNT)
                .map(|i| format!("type/{i}"))
                .collect(),
            ..Default::default()
        };
        assert!(!offer_metadata_valid(&oversized));
    }

    #[test]
    fn offer_metadata_valid_accepts_normal_offer() {
        assert!(offer_metadata_valid(&offer(
            "offer-1",
            &["text/plain", "image/png"]
        )));
    }

    #[test]
    fn clean_offer_mimes_validates_each_mime() {
        let cleaned = clean_offer_mimes(&offer("offer-1", &["text/plain", "  image/png  "]))
            .expect("valid mimes");
        assert_eq!(cleaned, vec!["text/plain", "image/png"]);

        let err =
            clean_offer_mimes(&offer("offer-1", &["text/plain", "\u{7}"])).expect_err("bad mime");
        assert!(matches!(
            err,
            KeynearError::Validation {
                field: "mimeType",
                ..
            }
        ));
    }

    #[test]
    fn choose_fetch_mime_falls_back_to_preferred_then_first() {
        let mimes = ["application/octet-stream", "image/png"]
            .map(String::from)
            .to_vec();
        assert_eq!(
            choose_fetch_mime(&mimes, "  "),
            Some("image/png".to_string())
        );

        let no_images = ["application/octet-stream", "text/plain"]
            .map(String::from)
            .to_vec();
        assert_eq!(
            choose_fetch_mime(&no_images, ""),
            Some("application/octet-stream".to_string())
        );
    }

    #[test]
    fn choose_fetch_mime_matches_normalized_request_and_rejects_unmatched() {
        let mimes = ["image/png", "text/plain"].map(String::from).to_vec();
        assert_eq!(
            choose_fetch_mime(&mimes, "IMAGE/PNG; charset=binary"),
            Some("image/png".to_string())
        );
        assert_eq!(choose_fetch_mime(&mimes, "image/webp"), None);
    }
}
