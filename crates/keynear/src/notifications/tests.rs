use keynear_core::TransferItem;

use super::*;

#[test]
fn parse_notification_action_handles_transfer_and_pairing_actions() {
    assert_eq!(
        parse_notification_action("transfer:tx-99:accept"),
        Some(NotificationAction::AcceptTransfer("tx-99".to_string()))
    );
    assert_eq!(
        parse_notification_action("transfer:tx-99:decline"),
        Some(NotificationAction::DeclineTransfer("tx-99".to_string()))
    );
    assert_eq!(
        parse_notification_action("transfer:tx-99:cancel"),
        Some(NotificationAction::CancelTransfer("tx-99".to_string()))
    );
    assert_eq!(
        parse_notification_action("pairing:peer-a:approve"),
        Some(NotificationAction::ApprovePairing("peer-a".to_string()))
    );
    assert_eq!(
        parse_notification_action("pairing:peer-a:reject"),
        Some(NotificationAction::RejectPairing("peer-a".to_string()))
    );
}

#[test]
fn parse_notification_action_rejects_malformed_or_unknown() {
    assert_eq!(parse_notification_action(""), None);
    assert_eq!(parse_notification_action("transfer::accept"), None);
    assert_eq!(parse_notification_action("transfer:tx-1:unknown"), None);
    assert_eq!(parse_notification_action("other:tx-1:accept"), None);
}

#[test]
fn parse_notification_action_handles_mirrored_and_call_actions() {
    assert_eq!(
        parse_notification_action("mirror:reply:n-1"),
        Some(NotificationAction::MirrorReply("n-1".to_string()))
    );
    assert_eq!(
        parse_notification_action("mirror:read:n-1"),
        Some(NotificationAction::MirrorMarkRead("n-1".to_string()))
    );
    assert_eq!(
        parse_notification_action("mirror:dismiss:n-1"),
        Some(NotificationAction::MirrorDismiss("n-1".to_string()))
    );
    assert_eq!(
        parse_notification_action("call:answer:Pixel 9"),
        Some(NotificationAction::CallAnswer("Pixel 9".to_string()))
    );
    assert_eq!(
        parse_notification_action("call:decline:Pixel 9"),
        Some(NotificationAction::CallDecline("Pixel 9".to_string()))
    );
    assert_eq!(
        parse_notification_action("call:mute:Pixel 9"),
        Some(NotificationAction::CallMute("Pixel 9".to_string()))
    );
}

#[test]
fn transfer_notification_content_formats_consent_and_active_progress() {
    let session = TransferSession {
        transfer_id: "tx-5".to_string(),
        peer_device_id: "pixel-9".to_string(),
        direction: TransferDirection::Receive,
        items: vec![TransferItem {
            name: "archive.zip".to_string(),
            mime_type: "application/zip".to_string(),
            size_bytes: 2048,
            is_directory: false,
            target_path: None,
            checksum: None,
        }],
        total_bytes: 2048,
        transferred_bytes: 1024,
        state: TransferState::PendingConsent,
        started_at: "2026-11-14T00:00:00Z".to_string(),
    };

    let (summary, body, actions) = transfer_notification_content(&session, "Pixel 9");
    assert_eq!(summary, "Incoming transfer from Pixel 9");
    assert!(body.contains("archive.zip"));
    assert_eq!(
        actions,
        vec![
            "transfer:tx-5:accept",
            "Accept",
            "transfer:tx-5:decline",
            "Decline"
        ]
    );
}
