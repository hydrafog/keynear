use std::collections::HashMap;

use futures_util::StreamExt;
use keynear_core::{TransferDirection, TransferSession, TransferState};
use tokio::sync::mpsc::UnboundedSender;
use tokio::task::JoinHandle;
use tracing::{info, warn};
use zbus::Variant;

const NOTIFICATIONS_DEST: &str = "org.freedesktop.Notifications";
const NOTIFICATIONS_PATH: &str = "/org/freedesktop/Notifications";
const NOTIFICATIONS_IFACE: &str = "org.freedesktop.Notifications";
const APP_NAME: &str = "Keynear";
const DEFAULT_EXPIRE_MS: i32 = -1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotificationAction {
    AcceptTransfer(String),
    DeclineTransfer(String),
    CancelTransfer(String),
    ApprovePairing(String),
    RejectPairing(String),
}

pub fn format_transfer_action(transfer_id: &str, verb: &str) -> String {
    format!("transfer:{transfer_id}:{verb}")
}

pub fn parse_notification_action(action_key: &str) -> Option<NotificationAction> {
    let mut parts = action_key.splitn(3, ':');
    let kind = parts.next()?;
    let target_id = parts.next()?.trim();
    let verb = parts.next()?.trim();
    if target_id.is_empty() {
        return None;
    }
    match (kind, verb) {
        ("transfer", "accept") => Some(NotificationAction::AcceptTransfer(target_id.to_string())),
        ("transfer", "decline") => Some(NotificationAction::DeclineTransfer(target_id.to_string())),
        ("transfer", "cancel") => Some(NotificationAction::CancelTransfer(target_id.to_string())),
        ("pairing", "approve") => Some(NotificationAction::ApprovePairing(target_id.to_string())),
        ("pairing", "reject") => Some(NotificationAction::RejectPairing(target_id.to_string())),
        _ => None,
    }
}

pub fn transfer_notification_content(
    session: &TransferSession,
    peer_name: &str,
) -> (String, String, Vec<String>) {
    let item_summary = if session.items.is_empty() {
        "files".to_string()
    } else if session.items.len() == 1 {
        session.items[0].name.clone()
    } else {
        format!(
            "{} (+{} more)",
            session.items[0].name,
            session.items.len() - 1
        )
    };
    match session.state {
        TransferState::PendingConsent => (
            format!("Incoming transfer from {peer_name}"),
            format!("{item_summary} ({} bytes)", session.total_bytes),
            vec![
                format_transfer_action(&session.transfer_id, "accept"),
                "Accept".to_string(),
                format_transfer_action(&session.transfer_id, "decline"),
                "Decline".to_string(),
            ],
        ),
        TransferState::Active => {
            let verb = match session.direction {
                TransferDirection::Send => "Sending",
                TransferDirection::Receive => "Receiving",
            };
            let pct = session
                .transferred_bytes
                .saturating_mul(100)
                .checked_div(session.total_bytes)
                .unwrap_or(0)
                .min(100);
            (
                format!("{verb} {item_summary}"),
                format!("{peer_name} — {pct}%"),
                vec![
                    format_transfer_action(&session.transfer_id, "cancel"),
                    "Cancel".to_string(),
                ],
            )
        }
        TransferState::Completed => (
            format!("Transfer complete from {peer_name}"),
            format!("Saved {item_summary}"),
            Vec::new(),
        ),
        TransferState::Cancelled => (
            "Transfer cancelled".to_string(),
            format!("{item_summary} ({peer_name})"),
            Vec::new(),
        ),
        TransferState::Failed => (
            "Transfer failed".to_string(),
            format!("{item_summary} ({peer_name})"),
            Vec::new(),
        ),
    }
}

#[derive(Clone)]
pub struct NotificationsClient {
    proxy: zbus::Proxy<'static>,
}

impl NotificationsClient {
    pub async fn connect_and_listen(
        action_tx: UnboundedSender<NotificationAction>,
    ) -> Option<(Self, JoinHandle<()>)> {
        let conn = match zbus::Connection::session().await {
            Ok(conn) => conn,
            Err(err) => {
                warn!(event = "notifications_unavailable", error = %err);
                return None;
            }
        };

        let proxy = match zbus::Proxy::new(
            &conn,
            NOTIFICATIONS_DEST,
            NOTIFICATIONS_PATH,
            NOTIFICATIONS_IFACE,
        )
        .await
        {
            Ok(proxy) => proxy,
            Err(err) => {
                warn!(event = "notifications_proxy_failed", error = %err);
                return None;
            }
        };

        let mut actions_stream = match proxy.receive_signal("ActionInvoked").await {
            Ok(stream) => stream,
            Err(err) => {
                warn!(event = "notifications_signal_unavailable", error = %err);
                return None;
            }
        };

        info!(event = "notifications_connected");
        let handle = tokio::spawn(async move {
            while let Some(msg) = actions_stream.next().await {
                let body = msg.body();
                if let Ok((_nid, action_key)) = body.deserialize::<(u32, String)>() {
                    if let Some(action) = parse_notification_action(&action_key) {
                        let _ = action_tx.send(action);
                    }
                }
            }
        });

        Some((Self { proxy }, handle))
    }

    pub async fn notify(
        &self,
        replaces_id: u32,
        icon: &str,
        summary: &str,
        body: &str,
        actions: &[String],
        urgency: u8,
    ) -> Option<u32> {
        let action_refs: Vec<&str> = actions.iter().map(String::as_str).collect();
        let mut hints: HashMap<&str, Variant<'_>> = HashMap::new();
        hints.insert("urgency", Variant::U8(urgency));
        match self
            .proxy
            .call::<_, _, u32>(
                "Notify",
                &(
                    APP_NAME,
                    replaces_id,
                    icon,
                    summary,
                    body,
                    action_refs,
                    hints,
                    DEFAULT_EXPIRE_MS,
                ),
            )
            .await
        {
            Ok(nid) => Some(nid),
            Err(err) => {
                warn!(event = "notifications_send_failed", error = %err);
                None
            }
        }
    }

    pub async fn notify_transfer_session(
        &self,
        session: &TransferSession,
        peer_name: &str,
        replaces_id: u32,
    ) -> Option<u32> {
        let (summary, body, actions) = transfer_notification_content(session, peer_name);
        let urgency = if session.state == TransferState::PendingConsent {
            2
        } else {
            1
        };
        let icon = match session.state {
            TransferState::Completed => "emblem-default",
            TransferState::Failed | TransferState::Cancelled => "dialog-warning",
            TransferState::PendingConsent | TransferState::Active => "document-send",
        };
        self.notify(replaces_id, icon, &summary, &body, &actions, urgency)
            .await
    }
}

#[cfg(test)]
mod tests {
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
}
