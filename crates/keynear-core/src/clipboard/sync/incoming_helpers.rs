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
