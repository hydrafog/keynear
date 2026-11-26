use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use keynear_protocol::{DaemonConfig, KeynearError};

use super::otp::check_otp_policy;

pub const BUILT_IN_IGNORED_MIMES: [&str; 2] =
    ["x-kde-passwordManagerHint", "application/x-password"];
pub const MAX_CLIPBOARD_IMAGE_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_INLINE_CLIPBOARD_BYTES: usize = 48 * 1024;
pub const MAX_CLIPBOARD_QUEUE: usize = 32;
pub const MAX_OUTGOING_OFFERS: usize = 16;
pub const MAX_INCOMING_OFFERS: usize = 32;
pub const CLIPBOARD_OFFER_TTL_SECS: u64 = 120;
pub const DEFAULT_TEXT_MIME: &str = "text/plain;charset=utf-8";

pub(crate) const MAX_CLIPBOARD_MIME_BYTES: usize = 256;
pub(crate) const MAX_OFFER_MIME_COUNT: usize = 64;

pub type DelayedRenderCallback = Arc<dyn Fn(&str) -> Result<Vec<u8>, KeynearError> + Send + Sync>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipboardPolicy {
    pub clipboard_sync_enabled: bool,
    pub clipboard_images_enabled: bool,
    pub otp_passthrough_enabled: bool,
    pub ignored_mime_types: Vec<String>,
}

impl Default for ClipboardPolicy {
    fn default() -> Self {
        Self::from(&DaemonConfig::default())
    }
}

impl From<&DaemonConfig> for ClipboardPolicy {
    fn from(config: &DaemonConfig) -> Self {
        Self {
            clipboard_sync_enabled: config.clipboard_sync_enabled,
            clipboard_images_enabled: config.clipboard_images_enabled,
            otp_passthrough_enabled: config.otp_passthrough_enabled,
            ignored_mime_types: config.ignored_mime_types.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipboardItem {
    pub id: String,
    pub peer_device_id: String,
    pub mime_type: String,
    pub data: Vec<u8>,
    pub is_otp: bool,
    pub is_image_offer: bool,
    pub offer_id: String,
    pub offered_mimes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DelayedClipboardOffer {
    pub offer_id: String,
    pub mime_types: Vec<String>,
    pub byte_size: u64,
    pub is_image: bool,
    pub is_otp: bool,
}

pub(crate) struct OutgoingOfferEntry {
    pub(crate) mime_types: Vec<String>,
    pub(crate) render: DelayedRenderCallback,
    pub(crate) created_at: Instant,
    pub(crate) is_image: bool,
    pub(crate) is_otp: bool,
}

#[derive(Clone)]
pub(crate) struct IncomingOfferEntry {
    pub(crate) peer_device_id: String,
    pub(crate) mime_types: Vec<String>,
    pub(crate) is_image: bool,
    pub(crate) is_otp: bool,
    pub(crate) created_at: Instant,
}

pub fn normalize_mime_type(raw: &str) -> String {
    raw.split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase()
}

pub fn should_ignore_mime_offers(offered_mimes: &[String], user_ignored: &[String]) -> bool {
    let mut normalized_rules: Vec<String> = BUILT_IN_IGNORED_MIMES
        .iter()
        .map(|m| normalize_mime_type(m))
        .filter(|m| !m.is_empty())
        .collect();
    for rule in user_ignored {
        let norm = normalize_mime_type(rule);
        if !norm.is_empty() && !normalized_rules.contains(&norm) {
            normalized_rules.push(norm);
        }
    }

    for offered in offered_mimes {
        let norm = normalize_mime_type(offered);
        if !norm.is_empty() && normalized_rules.iter().any(|rule| rule == &norm) {
            return true;
        }
    }
    false
}

pub fn is_image_mime(mime: &str) -> bool {
    normalize_mime_type(mime).starts_with("image/")
}

pub fn pick_preferred_image_mime(mimes: &[String]) -> Option<String> {
    const PREFERRED: [&str; 4] = ["image/png", "image/jpeg", "image/webp", "image/bmp"];
    for target in PREFERRED {
        if let Some(found) = mimes.iter().find(|m| normalize_mime_type(m) == target) {
            return Some(found.clone());
        }
    }
    mimes.iter().find(|m| is_image_mime(m)).cloned()
}

pub(crate) fn validate_mime_type(raw: &str) -> Result<String, KeynearError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.len() > MAX_CLIPBOARD_MIME_BYTES {
        return Err(KeynearError::Validation {
            field: "mimeType",
            message: format!("MIME type must be 1..={MAX_CLIPBOARD_MIME_BYTES} bytes"),
        });
    }
    if trimmed.bytes().any(|b| b < 0x20 || b == 0x7f) {
        return Err(KeynearError::Validation {
            field: "mimeType",
            message: "MIME type contains control characters".to_string(),
        });
    }
    Ok(trimmed.to_string())
}

pub(crate) fn check_policy(
    policy: &ClipboardPolicy,
    mimes: &[String],
    is_image: bool,
    is_otp: bool,
) -> Result<(), KeynearError> {
    check_otp_policy(policy, is_otp)?;
    if !is_otp && !policy.clipboard_sync_enabled {
        return Err(KeynearError::Validation {
            field: "clipboardSyncEnabled",
            message: "clipboard sync is disabled".to_string(),
        });
    }
    if is_image && !policy.clipboard_images_enabled {
        return Err(KeynearError::Validation {
            field: "clipboardImagesEnabled",
            message: "clipboard image sync is disabled".to_string(),
        });
    }
    if should_ignore_mime_offers(mimes, &policy.ignored_mime_types) {
        return Err(KeynearError::Validation {
            field: "ignoredMimeTypes",
            message: "clipboard offer matches ignored MIME policy".to_string(),
        });
    }
    Ok(())
}

pub(crate) fn prune_outgoing_offers(
    offers: &mut HashMap<String, OutgoingOfferEntry>,
    now: Instant,
) {
    let ttl = Duration::from_secs(CLIPBOARD_OFFER_TTL_SECS);
    offers.retain(|_, entry| now.saturating_duration_since(entry.created_at) < ttl);
}

pub(crate) fn prune_incoming_offers(
    offers: &mut HashMap<String, IncomingOfferEntry>,
    now: Instant,
) {
    let ttl = Duration::from_secs(CLIPBOARD_OFFER_TTL_SECS);
    offers.retain(|_, entry| now.saturating_duration_since(entry.created_at) < ttl);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_mime_type_strips_parameters_trims_and_lowercases() {
        assert_eq!(
            normalize_mime_type("  IMAGE/PNG ; charset=utf-8 "),
            "image/png"
        );
        assert_eq!(
            normalize_mime_type("text/plain;charset=utf-8"),
            "text/plain"
        );
        assert_eq!(normalize_mime_type("   "), "");
    }

    #[test]
    fn is_image_mime_detects_image_types_with_parameters() {
        assert!(is_image_mime("IMAGE/JPEG; charset=binary"));
        assert!(is_image_mime("image/webp"));
        assert!(!is_image_mime("text/plain; charset=utf-8"));
        assert!(!is_image_mime("application/image+json"));
    }

    #[test]
    fn pick_preferred_image_mime_honors_preference_regardless_of_order() {
        let mimes = [
            "image/bmp",
            "application/octet-stream",
            "image/webp",
            "image/jpeg; q=0.9",
            "image/png",
        ]
        .map(String::from)
        .to_vec();
        assert_eq!(
            pick_preferred_image_mime(&mimes),
            Some("image/png".to_string())
        );

        let fallbacks = ["image/gif", "image/svg+xml"].map(String::from).to_vec();
        assert_eq!(
            pick_preferred_image_mime(&fallbacks),
            Some("image/gif".to_string())
        );
        assert_eq!(pick_preferred_image_mime(&[]), None);
    }
}
