#![allow(clippy::cognitive_complexity, clippy::too_many_lines)]

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

#[test]
fn validate_mime_type_trims_and_rejects_bad_values() {
    assert_eq!(
        validate_mime_type("  text/plain  ").expect("trimmed"),
        "text/plain"
    );
    assert!(matches!(
        validate_mime_type(""),
        Err(KeynearError::Validation {
            field: "mimeType",
            ..
        })
    ));
    assert!(matches!(
        validate_mime_type(&"x".repeat(MAX_CLIPBOARD_MIME_BYTES + 1)),
        Err(KeynearError::Validation {
            field: "mimeType",
            ..
        })
    ));
    assert_eq!(
        validate_mime_type("text/plain\n").expect("surrounding whitespace is trimmed"),
        "text/plain"
    );
}

#[test]
fn check_policy_enforces_otp_sync_image_and_ignore_rules() {
    let base = ClipboardPolicy {
        clipboard_sync_enabled: true,
        clipboard_images_enabled: true,
        otp_passthrough_enabled: true,
        ignored_mime_types: Vec::new(),
    };
    assert!(check_policy(&base, &["text/plain".to_string()], false, false).is_ok());
    let no_otp = ClipboardPolicy {
        otp_passthrough_enabled: false,
        ..base.clone()
    };
    assert!(matches!(
        check_policy(&no_otp, &[], false, true),
        Err(KeynearError::Validation {
            field: "otpPassthroughEnabled",
            ..
        })
    ));
    assert!(check_policy(&no_otp, &["text/plain".to_string()], false, false).is_ok());
    let no_sync = ClipboardPolicy {
        clipboard_sync_enabled: false,
        ..base.clone()
    };
    assert!(matches!(
        check_policy(&no_sync, &[], false, false),
        Err(KeynearError::Validation {
            field: "clipboardSyncEnabled",
            ..
        })
    ));
    assert!(check_policy(&no_sync, &[], false, true).is_ok());
    let no_image = ClipboardPolicy {
        clipboard_images_enabled: false,
        ..base.clone()
    };
    assert!(matches!(
        check_policy(&no_image, &[], true, false),
        Err(KeynearError::Validation {
            field: "clipboardImagesEnabled",
            ..
        })
    ));
    let ignored = ClipboardPolicy {
        ignored_mime_types: vec!["x-secret".to_string()],
        ..base
    };
    assert!(matches!(
        check_policy(&ignored, &["x-secret".to_string()], false, false),
        Err(KeynearError::Validation {
            field: "ignoredMimeTypes",
            ..
        })
    ));
}

#[test]
fn prune_helpers_drop_expired_offers_and_keep_fresh_ones() {
    let base = Instant::now();
    let now = base + Duration::from_secs(CLIPBOARD_OFFER_TTL_SECS + 1);
    let fresh_at = base + Duration::from_secs(10);
    let mut outgoing: HashMap<String, OutgoingOfferEntry> = HashMap::new();
    outgoing.insert(
        "old".to_string(),
        OutgoingOfferEntry {
            mime_types: vec!["text/plain".to_string()],
            render: Arc::new(|_| Ok(vec![1])),
            created_at: base,
            is_image: false,
            is_otp: false,
        },
    );
    outgoing.insert(
        "fresh".to_string(),
        OutgoingOfferEntry {
            mime_types: vec!["text/plain".to_string()],
            render: Arc::new(|_| Ok(vec![2])),
            created_at: fresh_at,
            is_image: false,
            is_otp: false,
        },
    );
    prune_outgoing_offers(&mut outgoing, now);
    assert!(!outgoing.contains_key("old"));
    assert!(outgoing.contains_key("fresh"));

    let mut incoming: HashMap<String, IncomingOfferEntry> = HashMap::new();
    for (id, created_at) in [("old", base), ("fresh", fresh_at)] {
        incoming.insert(
            id.to_string(),
            IncomingOfferEntry {
                peer_device_id: "phone-1".to_string(),
                mime_types: vec!["text/plain".to_string()],
                is_image: false,
                is_otp: false,
                created_at,
            },
        );
    }
    prune_incoming_offers(&mut incoming, now);
    assert!(!incoming.contains_key("old"));
    assert!(incoming.contains_key("fresh"));
}
