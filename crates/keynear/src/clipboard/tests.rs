use keynear_core::ClipboardPolicy;
use std::time::{Duration, Instant};

use super::config::CLIPBOARD_ECHO_WINDOW_SECS;
use super::mime::{evaluate_offered_mimes, OfferAction};
use super::tracker::SelfWriteTracker;
use super::*;

#[test]
fn backend_fallback_and_headless_degrade_non_fatally() {
    assert_eq!(
        select_backend_from_caps(ProbeCapabilities {
            has_ext_data_control: true,
            has_zwlr_data_control: true,
            has_x11: true,
        }),
        ClipboardBackendKind::ExtDataControlV1
    );
    assert_eq!(
        select_backend_from_caps(ProbeCapabilities {
            has_ext_data_control: false,
            has_zwlr_data_control: true,
            has_x11: true,
        }),
        ClipboardBackendKind::ZwlrDataControlV1
    );
    assert_eq!(
        select_backend_from_caps(ProbeCapabilities {
            has_ext_data_control: false,
            has_zwlr_data_control: false,
            has_x11: true,
        }),
        ClipboardBackendKind::X11
    );
    assert_eq!(
        select_backend_from_caps(ProbeCapabilities::default()),
        ClipboardBackendKind::HeadlessUnavailable
    );

    let _ = probe_clipboard_backend();
    let headless = ClipboardMonitorHandle::headless();
    assert_eq!(
        headless.backend(),
        ClipboardBackendKind::HeadlessUnavailable
    );
    headless
        .write_clipboard("text/plain", b"safe in headless")
        .expect("headless write succeeds non-fatally");
}

#[test]
fn pre_read_mime_filter_skips_before_read() {
    let mut policy = ClipboardPolicy::default();

    assert_eq!(
        evaluate_offered_mimes(
            &[
                "text/plain;charset=utf-8".to_string(),
                "x-kde-passwordManagerHint".to_string(),
            ],
            &policy,
        ),
        OfferAction::SkipIgnored
    );
    assert_eq!(
        evaluate_offered_mimes(&["application/x-password".to_string()], &policy),
        OfferAction::SkipIgnored
    );

    policy
        .ignored_mime_types
        .push("application/x-custom-secret".to_string());
    assert_eq!(
        evaluate_offered_mimes(
            &[
                "text/plain".to_string(),
                "Application/X-Custom-Secret; version=1".to_string(),
            ],
            &policy,
        ),
        OfferAction::SkipIgnored
    );

    assert_eq!(
        evaluate_offered_mimes(
            &["text/plain".to_string(), "image/png".to_string()],
            &policy,
        ),
        OfferAction::OfferImage(vec!["image/png".to_string()])
    );

    policy.clipboard_images_enabled = false;
    assert_eq!(
        evaluate_offered_mimes(&["image/png".to_string()], &policy),
        OfferAction::SkipUnsupported
    );

    assert_eq!(
        evaluate_offered_mimes(&["text/plain;charset=utf-8".to_string()], &policy),
        OfferAction::ReadText("text/plain;charset=utf-8".to_string())
    );
}

#[test]
fn self_write_echo_suppression_drops_recent_digest() {
    let mut tracker = SelfWriteTracker::default();
    let t0 = Instant::now();
    tracker.record_write(b"847291", t0);

    assert!(tracker.is_recent_echo(b"847291", t0 + Duration::from_secs(1)));
    assert!(!tracker.is_recent_echo(b"different-copy", t0 + Duration::from_secs(1)));
    assert!(!tracker.is_recent_echo(
        b"847291",
        t0 + Duration::from_secs(CLIPBOARD_ECHO_WINDOW_SECS + 1)
    ));
}
