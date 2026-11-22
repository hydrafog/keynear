use keynear_core::{
    is_package_excluded, negotiate_capabilities, validate_descriptor, validate_metadata,
    validate_reply_text, CallActionKind, CallPhase, MediaMetadata, NotificationDescriptor,
    PlaybackState, CAP_CALL_ALERTS, CAP_MEDIA_BRIDGE, CAP_NOTIFICATION_MIRROR,
};

use crate::callbacks::FfiError;

#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiNotificationDescriptor {
    pub key: String,
    pub package: String,
    pub title: String,
    pub body: String,
    pub category: String,
    pub sequence: u64,
    pub supports_reply: bool,
    pub supports_mark_read: bool,
    pub supports_dismiss: bool,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiMirroredNotification {
    pub notification_key: String,
    pub package_name: String,
    pub title: String,
    pub body: String,
    pub category: String,
    pub sequence: u64,
    pub supports_reply: bool,
    pub supports_mark_read: bool,
    pub supports_dismiss: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct FfiCapabilityPair {
    pub local: u32,
    pub peer: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum FfiNotificationAction {
    Reply,
    MarkRead,
    Dismiss,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum FfiCallAction {
    Answer,
    Decline,
    Mute,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum FfiCallPhase {
    Incoming,
    Active,
    Ended,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum FfiPlayback {
    Playing,
    Paused,
    Stopped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum FfiMediaCommand {
    Play,
    Pause,
    Next,
    Previous,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiMediaState {
    pub player_id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub playback: FfiPlayback,
    pub position_ms: u64,
    pub duration_ms: u64,
    pub sequence: u64,
    pub can_pause: bool,
    pub can_next: bool,
    pub can_previous: bool,
}

#[uniffi::export]
pub fn continuity_capability_flags() -> u32 {
    CAP_NOTIFICATION_MIRROR | CAP_MEDIA_BRIDGE | CAP_CALL_ALERTS
}

#[uniffi::export]
pub fn mirror_notification(
    descriptor: FfiNotificationDescriptor,
    exclusions: Vec<String>,
    capabilities: FfiCapabilityPair,
    last_sequence: u64,
) -> Result<Option<FfiMirroredNotification>, FfiError> {
    if negotiate_capabilities(capabilities.local, capabilities.peer) & CAP_NOTIFICATION_MIRROR == 0
    {
        return Ok(None);
    }
    let view = NotificationDescriptor {
        key: &descriptor.key,
        package: &descriptor.package,
        title: &descriptor.title,
        body: &descriptor.body,
        category: &descriptor.category,
        sequence: descriptor.sequence,
        supports_reply: descriptor.supports_reply,
        supports_mark_read: descriptor.supports_mark_read,
        supports_dismiss: descriptor.supports_dismiss,
    };
    validate_descriptor(&view)?;
    if is_package_excluded(descriptor.package.as_str(), &exclusions)
        || descriptor.sequence <= last_sequence
    {
        return Ok(None);
    }
    Ok(Some(FfiMirroredNotification {
        notification_key: descriptor.key,
        package_name: descriptor.package,
        title: descriptor.title,
        body: descriptor.body,
        category: descriptor.category,
        sequence: descriptor.sequence,
        supports_reply: descriptor.supports_reply,
        supports_mark_read: descriptor.supports_mark_read,
        supports_dismiss: descriptor.supports_dismiss,
    }))
}

#[uniffi::export]
pub fn validate_notification_reply(text: String) -> Result<(), FfiError> {
    validate_reply_text(&text)?;
    Ok(())
}

#[uniffi::export]
pub fn call_action_allowed(phase: FfiCallPhase, action: FfiCallAction) -> bool {
    let phase = match phase {
        FfiCallPhase::Incoming => CallPhase::Incoming,
        FfiCallPhase::Active => CallPhase::Active,
        FfiCallPhase::Ended => CallPhase::Ended,
    };
    let action = match action {
        FfiCallAction::Answer => CallActionKind::Answer,
        FfiCallAction::Decline => CallActionKind::Decline,
        FfiCallAction::Mute => CallActionKind::Mute,
    };
    matches!(
        (phase, action),
        (
            CallPhase::Incoming,
            CallActionKind::Answer | CallActionKind::Decline
        ) | (CallPhase::Active, CallActionKind::Mute)
    )
}

#[uniffi::export]
pub fn media_command_allowed(
    state: FfiMediaState,
    command: FfiMediaCommand,
    last_command_sequence: u64,
    capabilities: FfiCapabilityPair,
) -> Result<bool, FfiError> {
    if negotiate_capabilities(capabilities.local, capabilities.peer) & CAP_MEDIA_BRIDGE == 0 {
        return Ok(false);
    }
    let metadata = MediaMetadata {
        player_id: state.player_id,
        title: state.title,
        artist: state.artist,
        album: state.album,
        playback: match state.playback {
            FfiPlayback::Playing => PlaybackState::Playing,
            FfiPlayback::Paused => PlaybackState::Paused,
            FfiPlayback::Stopped => PlaybackState::Stopped,
        },
        position_ms: state.position_ms,
        duration_ms: state.duration_ms,
        sequence: state.sequence,
        can_pause: state.can_pause,
        can_next: state.can_next,
        can_previous: state.can_previous,
    };
    validate_metadata(&metadata)?;
    let permitted = match command {
        FfiMediaCommand::Play | FfiMediaCommand::Pause => metadata.can_pause,
        FfiMediaCommand::Next => metadata.can_next,
        FfiMediaCommand::Previous => metadata.can_previous,
    };
    Ok(permitted && metadata.sequence > last_command_sequence)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn descriptor() -> FfiNotificationDescriptor {
        FfiNotificationDescriptor {
            key: "n-1".to_string(),
            package: "org.example.chat".to_string(),
            title: "Alice".to_string(),
            body: "Hello".to_string(),
            category: "msg".to_string(),
            sequence: 2,
            supports_reply: true,
            supports_mark_read: true,
            supports_dismiss: true,
        }
    }

    fn flags() -> FfiCapabilityPair {
        FfiCapabilityPair {
            local: continuity_capability_flags(),
            peer: continuity_capability_flags(),
        }
    }

    #[test]
    fn exported_flags_cover_all_three_continuity_capabilities() {
        assert_eq!(
            continuity_capability_flags(),
            CAP_NOTIFICATION_MIRROR | CAP_MEDIA_BRIDGE | CAP_CALL_ALERTS
        );
    }

    #[test]
    fn excluded_package_and_stale_sequence_are_dropped() {
        let excluded = mirror_notification(
            descriptor(),
            vec!["org.example.chat".to_string()],
            flags(),
            0,
        )
        .expect("excluded");
        assert!(excluded.is_none());
        let stale = mirror_notification(descriptor(), vec![], flags(), 2).expect("stale");
        assert!(stale.is_none());
        let fresh = mirror_notification(descriptor(), vec![], flags(), 1).expect("fresh");
        assert_eq!(fresh.expect("view").notification_key, "n-1");
    }

    #[test]
    fn call_actions_follow_the_alert_phase() {
        assert!(call_action_allowed(
            FfiCallPhase::Incoming,
            FfiCallAction::Answer
        ));
        assert!(call_action_allowed(
            FfiCallPhase::Incoming,
            FfiCallAction::Decline
        ));
        assert!(!call_action_allowed(
            FfiCallPhase::Incoming,
            FfiCallAction::Mute
        ));
        assert!(call_action_allowed(
            FfiCallPhase::Active,
            FfiCallAction::Mute
        ));
    }

    #[test]
    fn media_command_requires_capability_and_fresh_sequence() {
        let state = FfiMediaState {
            player_id: "player-1".to_string(),
            title: "Track".to_string(),
            artist: "Artist".to_string(),
            album: "Album".to_string(),
            playback: FfiPlayback::Playing,
            position_ms: 0,
            duration_ms: 100,
            sequence: 3,
            can_pause: true,
            can_next: true,
            can_previous: true,
        };
        assert!(
            media_command_allowed(state.clone(), FfiMediaCommand::Pause, 2, flags())
                .expect("fresh")
        );
        assert!(!media_command_allowed(state, FfiMediaCommand::Pause, 3, flags()).expect("stale"));
    }
}
