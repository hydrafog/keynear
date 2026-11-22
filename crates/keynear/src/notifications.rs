mod mirror;

#[cfg(test)]
mod tests;

pub use mirror::{
    call_alert_content, mirrored_notification_content, parse_mirrored_action, MirroredContentInput,
};

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

type ActionStream = zbus::proxy::SignalStream<'static>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotificationAction {
    AcceptTransfer(String),
    DeclineTransfer(String),
    CancelTransfer(String),
    ApprovePairing(String),
    RejectPairing(String),
    MirrorReply(String),
    MirrorMarkRead(String),
    MirrorDismiss(String),
    CallAnswer(String),
    CallDecline(String),
    CallMute(String),
}

pub fn format_transfer_action(transfer_id: &str, verb: &str) -> String {
    format!("transfer:{transfer_id}:{verb}")
}

pub fn parse_notification_action(action_key: &str) -> Option<NotificationAction> {
    if let Some(action) = parse_mirrored_action(action_key) {
        return Some(mirror::into_notification_action(action));
    }
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

fn transfer_item_summary(session: &TransferSession) -> String {
    match session.items.len() {
        0 => "files".to_string(),
        1 => session.items[0].name.clone(),
        _ => format!(
            "{} (+{} more)",
            session.items[0].name,
            session.items.len() - 1
        ),
    }
}

pub fn transfer_notification_content(
    session: &TransferSession,
    peer_name: &str,
) -> (String, String, Vec<String>) {
    let item_summary = transfer_item_summary(session);
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

async fn connect_bus() -> Option<zbus::Connection> {
    zbus::Connection::session()
        .await
        .map_err(|err| warn!(event = "notifications_unavailable", error = %err))
        .ok()
}

async fn create_proxy(conn: &zbus::Connection) -> Option<zbus::Proxy<'static>> {
    let args = (NOTIFICATIONS_DEST, NOTIFICATIONS_PATH, NOTIFICATIONS_IFACE);
    zbus::Proxy::new(conn, args.0, args.1, args.2)
        .await
        .map_err(|err| warn!(event = "notifications_proxy_failed", error = %err))
        .ok()
}

async fn receive_actions(proxy: &zbus::Proxy<'static>) -> Option<ActionStream> {
    proxy
        .receive_signal("ActionInvoked")
        .await
        .map_err(|err| warn!(event = "notifications_signal_unavailable", error = %err))
        .ok()
}

async fn run_action_listener(mut stream: ActionStream, tx: UnboundedSender<NotificationAction>) {
    while let Some(msg) = stream.next().await {
        if let Ok((_nid, key)) = msg.body().deserialize::<(u32, String)>() {
            if let Some(action) = parse_notification_action(&key) {
                let _ = tx.send(action);
            }
        }
    }
}

fn log_notifications_connected() {
    info!(event = "notifications_connected");
}

impl NotificationsClient {
    pub async fn connect_and_listen(
        action_tx: UnboundedSender<NotificationAction>,
    ) -> Option<(Self, JoinHandle<()>)> {
        let conn = connect_bus().await?;
        let proxy = create_proxy(&conn).await?;
        let actions_stream = receive_actions(&proxy).await?;
        log_notifications_connected();
        let handle = tokio::spawn(run_action_listener(actions_stream, action_tx));
        Some((Self { proxy }, handle))
    }

    #[allow(clippy::too_many_arguments)]
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
        self.proxy
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
            .map_err(|err| warn!(event = "notifications_send_failed", error = %err))
            .ok()
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

    #[allow(dead_code, clippy::too_many_arguments)]
    pub async fn notify_mirrored_notification(
        &self,
        peer_name: &str,
        notification_key: &str,
        title: &str,
        body: &str,
        supports: (bool, bool, bool),
        replaces_id: u32,
    ) -> Option<u32> {
        let (summary, text, actions) = mirrored_notification_content(MirroredContentInput {
            peer_name,
            notification_key,
            title,
            body,
            supports,
        });
        self.notify(
            replaces_id,
            "mail-unread-symbolic",
            &summary,
            &text,
            &actions,
            1,
        )
        .await
    }

    #[allow(dead_code)]
    pub async fn notify_call_alert(&self, caller: &str, replaces_id: u32) -> Option<u32> {
        let (s, b, a) = call_alert_content(caller);
        self.notify(replaces_id, "call-start-symbolic", &s, &b, &a, 2)
            .await
    }

    pub async fn notify_pairing_verification(&self, code: &str) -> Option<u32> {
        let b = format!("Verify code {code} matches your phone, then approve within 120s.");
        let a = [
            "pairing:active:approve",
            "Approve",
            "pairing:active:reject",
            "Reject",
        ]
        .map(String::from);
        let title = "Keynear pairing request";
        self.notify(0, "bluetooth-active-symbolic", title, &b, &a, 2)
            .await
    }

    pub async fn notify_pairing_completed(&self, peer_device_id: &str) -> Option<u32> {
        let b = format!("Paired with {peer_device_id}.");
        self.notify(0, "emblem-default", "Keynear device paired", &b, &[], 1)
            .await
    }
}
