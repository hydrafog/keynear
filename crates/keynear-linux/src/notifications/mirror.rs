use super::NotificationAction;

const REPLY_PREFIX: &str = "mirror:reply:";
const READ_PREFIX: &str = "mirror:read:";
const DISMISS_PREFIX: &str = "mirror:dismiss:";
const CALL_ANSWER_PREFIX: &str = "call:answer:";
const CALL_DECLINE_PREFIX: &str = "call:decline:";
const CALL_MUTE_PREFIX: &str = "call:mute:";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CarinaAction {
    Reply(String),
    MarkRead(String),
    Dismiss(String),
    CallAnswer(String),
    CallDecline(String),
    CallMute(String),
}

pub fn parse_carina_action(action_key: &str) -> Option<CarinaAction> {
    if let Some(key) = action_key.strip_prefix(REPLY_PREFIX) {
        return non_empty(key).map(CarinaAction::Reply);
    }
    if let Some(key) = action_key.strip_prefix(READ_PREFIX) {
        return non_empty(key).map(CarinaAction::MarkRead);
    }
    if let Some(key) = action_key.strip_prefix(DISMISS_PREFIX) {
        return non_empty(key).map(CarinaAction::Dismiss);
    }
    if let Some(call_id) = action_key.strip_prefix(CALL_ANSWER_PREFIX) {
        return non_empty(call_id).map(CarinaAction::CallAnswer);
    }
    if let Some(call_id) = action_key.strip_prefix(CALL_DECLINE_PREFIX) {
        return non_empty(call_id).map(CarinaAction::CallDecline);
    }
    if let Some(call_id) = action_key.strip_prefix(CALL_MUTE_PREFIX) {
        return non_empty(call_id).map(CarinaAction::CallMute);
    }
    None
}

pub fn into_notification_action(action: CarinaAction) -> NotificationAction {
    match action {
        CarinaAction::Reply(key) => NotificationAction::MirrorReply(key),
        CarinaAction::MarkRead(key) => NotificationAction::MirrorMarkRead(key),
        CarinaAction::Dismiss(key) => NotificationAction::MirrorDismiss(key),
        CarinaAction::CallAnswer(id) => NotificationAction::CallAnswer(id),
        CarinaAction::CallDecline(id) => NotificationAction::CallDecline(id),
        CarinaAction::CallMute(id) => NotificationAction::CallMute(id),
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MirroredContentInput<'a> {
    pub peer_name: &'a str,
    pub notification_key: &'a str,
    pub title: &'a str,
    pub body: &'a str,
    pub supports: (bool, bool, bool),
}

#[allow(dead_code)]
pub fn mirrored_notification_content(
    input: MirroredContentInput<'_>,
) -> (String, String, Vec<String>) {
    let MirroredContentInput {
        peer_name,
        notification_key,
        title,
        body,
        supports,
    } = input;
    let mut actions = Vec::new();
    if supports.0 {
        actions.push(format!("{REPLY_PREFIX}{notification_key}"));
        actions.push("Reply".to_string());
    }
    if supports.1 {
        actions.push(format!("{READ_PREFIX}{notification_key}"));
        actions.push("Mark as read".to_string());
    }
    if supports.2 {
        actions.push(format!("{DISMISS_PREFIX}{notification_key}"));
        actions.push("Dismiss".to_string());
    }
    (format!("{title} — {peer_name}"), body.to_string(), actions)
}

#[allow(dead_code)]
pub fn call_alert_content(caller: &str) -> (String, String, Vec<String>) {
    (
        format!("Incoming call from {caller}"),
        "Keynear".to_string(),
        vec![
            format!("{CALL_ANSWER_PREFIX}{caller}"),
            "Answer".to_string(),
            format!("{CALL_DECLINE_PREFIX}{caller}"),
            "Decline".to_string(),
            format!("{CALL_MUTE_PREFIX}{caller}"),
            "Mute".to_string(),
        ],
    )
}

fn non_empty(value: &str) -> Option<String> {
    (!value.is_empty()).then(|| value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn carina_action_keys_parse_to_typed_variants() {
        assert_eq!(
            parse_carina_action("mirror:reply:n-1"),
            Some(CarinaAction::Reply("n-1".to_string()))
        );
        assert_eq!(
            parse_carina_action("mirror:read:n-1"),
            Some(CarinaAction::MarkRead("n-1".to_string()))
        );
        assert_eq!(
            parse_carina_action("mirror:dismiss:n-1"),
            Some(CarinaAction::Dismiss("n-1".to_string()))
        );
        assert_eq!(
            parse_carina_action("call:answer:Pixel 9"),
            Some(CarinaAction::CallAnswer("Pixel 9".to_string()))
        );
        assert_eq!(
            parse_carina_action("call:decline:Pixel 9"),
            Some(CarinaAction::CallDecline("Pixel 9".to_string()))
        );
        assert_eq!(
            parse_carina_action("call:mute:Pixel 9"),
            Some(CarinaAction::CallMute("Pixel 9".to_string()))
        );
    }

    #[test]
    fn malformed_carina_action_keys_are_rejected() {
        assert_eq!(parse_carina_action("mirror:reply:"), None);
        assert_eq!(parse_carina_action("mirror:unknown:n-1"), None);
        assert_eq!(parse_carina_action("transfer:n-1:accept"), None);
    }

    #[test]
    fn mirrored_content_uses_the_confirmed_action_labels() {
        let (summary, body, actions) = mirrored_notification_content(MirroredContentInput {
            peer_name: "Pixel 9",
            notification_key: "n-1",
            title: "Alice",
            body: "Hello",
            supports: (true, true, true),
        });
        assert_eq!(summary, "Alice — Pixel 9");
        assert_eq!(body, "Hello");
        assert_eq!(
            actions,
            vec![
                "mirror:reply:n-1",
                "Reply",
                "mirror:read:n-1",
                "Mark as read",
                "mirror:dismiss:n-1",
                "Dismiss"
            ]
        );
    }

    #[test]
    fn call_content_uses_answer_decline_and_mute() {
        let (summary, _, actions) = call_alert_content("Pixel 9");
        assert_eq!(summary, "Incoming call from Pixel 9");
        assert_eq!(
            actions,
            vec![
                "call:answer:Pixel 9",
                "Answer",
                "call:decline:Pixel 9",
                "Decline",
                "call:mute:Pixel 9",
                "Mute"
            ]
        );
    }
}
