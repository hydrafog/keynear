use keynear_core::{
    is_image_mime, normalize_mime_type, should_ignore_mime_offers, ClipboardPolicy,
};

#[derive(Debug, PartialEq, Eq)]
pub enum OfferAction {
    SkipIgnored,
    OfferImage(Vec<String>),
    ReadText(String),
    SkipUnsupported,
}

pub fn evaluate_offered_mimes(offered_mimes: &[String], policy: &ClipboardPolicy) -> OfferAction {
    if offered_mimes.is_empty() || !policy.clipboard_sync_enabled {
        return OfferAction::SkipIgnored;
    }
    if should_ignore_mime_offers(offered_mimes, &policy.ignored_mime_types) {
        return OfferAction::SkipIgnored;
    }

    let image_mimes: Vec<String> = offered_mimes
        .iter()
        .filter(|m| is_image_mime(m))
        .cloned()
        .collect();
    if !image_mimes.is_empty() {
        if policy.clipboard_images_enabled {
            return OfferAction::OfferImage(image_mimes);
        }
        return OfferAction::SkipUnsupported;
    }

    if let Some(text_mime) = select_preferred_text_mime(offered_mimes) {
        return OfferAction::ReadText(text_mime);
    }

    OfferAction::SkipUnsupported
}

fn select_preferred_text_mime(mimes: &[String]) -> Option<String> {
    const PREFERRED: [&str; 5] = [
        "text/plain;charset=utf-8",
        "utf8_string",
        "text/plain",
        "utf8-string",
        "string",
    ];
    for candidate in PREFERRED {
        if let Some(found) = mimes
            .iter()
            .find(|m| m.trim().eq_ignore_ascii_case(candidate))
        {
            return Some(found.clone());
        }
    }
    mimes
        .iter()
        .find(|m| normalize_mime_type(m).starts_with("text/"))
        .cloned()
}
