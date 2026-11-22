use keynear_protocol::KeynearError;

use super::*;
use crate::CAP_NOTIFICATION_MIRROR;

fn descriptor<'a>(key: &'a str, package: &'a str, sequence: u64) -> NotificationDescriptor<'a> {
    NotificationDescriptor {
        key,
        package,
        title: "Message",
        body: "Hello",
        category: "msg",
        sequence,
        supports_reply: true,
        supports_mark_read: true,
        supports_dismiss: true,
    }
}

fn flags() -> MirrorCapabilities {
    MirrorCapabilities {
        local: CAP_NOTIFICATION_MIRROR,
        peer: CAP_NOTIFICATION_MIRROR,
    }
}

#[test]
fn excluded_package_never_reaches_envelope() {
    let mut engine = MirrorEngine::new();
    let exclusions = vec!["com.secret.chat".to_string()];
    let result = engine
        .accept(
            descriptor("key-1", "com.secret.chat", 1),
            &exclusions,
            flags(),
        )
        .expect("accept");
    assert!(result.is_none());
    let allowed = engine
        .accept(
            descriptor("key-2", "org.example.chat", 1),
            &exclusions,
            flags(),
        )
        .expect("accept");
    assert_eq!(allowed.expect("allowed").package_name, "org.example.chat");
}

#[test]
fn duplicate_notification_key_is_ignored() {
    let mut engine = MirrorEngine::new();
    assert!(engine
        .accept(descriptor("key-1", "org.example.chat", 5), &[], flags())
        .expect("first")
        .is_some());
    assert!(engine
        .accept(descriptor("key-1", "org.example.chat", 5), &[], flags())
        .expect("duplicate")
        .is_none());
    assert!(engine
        .accept(descriptor("key-1", "org.example.chat", 4), &[], flags())
        .expect("stale")
        .is_none());
    assert!(engine
        .accept(descriptor("key-1", "org.example.chat", 6), &[], flags())
        .expect("fresh")
        .is_some());
}

#[test]
fn control_characters_in_notification_text_are_rejected() {
    let bad = NotificationDescriptor {
        title: "bad\nline",
        ..descriptor("key-1", "org.example.chat", 1)
    };
    let err = validate_descriptor(&bad).expect_err("control character rejected");
    assert!(matches!(err, KeynearError::Validation { .. }));
}

#[test]
fn oversized_notification_text_is_rejected() {
    let body = "x".repeat(MAX_NOTIFICATION_BODY_LEN + 1);
    let bad = NotificationDescriptor {
        body: &body,
        ..descriptor("key-1", "org.example.chat", 1)
    };
    let err = validate_descriptor(&bad).expect_err("oversized body rejected");
    assert!(matches!(err, KeynearError::Validation { .. }));
}

#[test]
fn peer_without_capability_yields_none() {
    let mut engine = MirrorEngine::new();
    let result = engine
        .accept(
            descriptor("key-1", "org.example.chat", 1),
            &[],
            MirrorCapabilities {
                local: CAP_NOTIFICATION_MIRROR,
                peer: 0,
            },
        )
        .expect("no capability");
    assert!(result.is_none());
}

#[test]
fn empty_or_oversized_reply_is_rejected() {
    assert!(matches!(
        validate_reply_text(""),
        Err(KeynearError::Validation { .. })
    ));
    assert!(matches!(
        validate_reply_text(&"x".repeat(MAX_NOTIFICATION_BODY_LEN + 1)),
        Err(KeynearError::Validation { .. })
    ));
    assert!(validate_reply_text("Okay").is_ok());
}

#[test]
fn forget_clears_a_single_key_without_touching_others() {
    let mut engine = MirrorEngine::new();
    engine
        .accept(descriptor("key-1", "org.example.chat", 1), &[], flags())
        .expect("first");
    engine
        .accept(descriptor("key-2", "org.example.chat", 1), &[], flags())
        .expect("second");
    engine.forget("key-1");
    assert!(engine
        .accept(descriptor("key-1", "org.example.chat", 1), &[], flags())
        .expect("replayed")
        .is_some());
    assert!(engine
        .accept(descriptor("key-2", "org.example.chat", 1), &[], flags())
        .expect("still remembered")
        .is_none());
}
