use keynear_protocol::KeynearError;

use super::*;

fn metadata(player_id: &str, title: &str, sequence: u64) -> MediaMetadata {
    MediaMetadata {
        player_id: player_id.to_string(),
        title: title.to_string(),
        artist: "Artist".to_string(),
        album: "Album".to_string(),
        playback: PlaybackState::Playing,
        position_ms: 1_000,
        duration_ms: 200_000,
        sequence,
        can_pause: true,
        can_next: true,
        can_previous: true,
    }
}

#[test]
fn metadata_converges_across_players() {
    let mut bridge = MediaSessionBridge::new();
    assert!(bridge
        .observe(metadata("player-1", "Track A", 1))
        .expect("first"));
    assert!(bridge
        .observe(metadata("player-2", "Track B", 1))
        .expect("second"));
    assert_eq!(bridge.players().count(), 2);
    assert_eq!(bridge.snapshot("player-1").expect("state").title, "Track A");
}

#[test]
fn stale_media_sequence_is_ignored() {
    let mut bridge = MediaSessionBridge::new();
    bridge
        .observe(metadata("player-1", "Track A", 5))
        .expect("first");
    assert!(!bridge
        .observe(metadata("player-1", "Old", 4))
        .expect("stale"));
    assert_eq!(bridge.snapshot("player-1").expect("state").title, "Track A");
    assert!(bridge
        .observe(metadata("player-1", "New", 6))
        .expect("fresh"));
}

#[test]
fn unknown_media_player_command_is_ignored() {
    let mut bridge = MediaSessionBridge::new();
    let err = bridge
        .command("missing", MediaCommandKind::Pause, 1)
        .expect_err("unknown player rejected");
    assert!(matches!(err, KeynearError::Protocol(_)));
}

#[test]
fn media_command_round_trips_with_a_fresh_sequence() {
    let mut bridge = MediaSessionBridge::new();
    bridge
        .observe(metadata("player-1", "Track A", 1))
        .expect("observe");
    let plan = bridge
        .command("player-1", MediaCommandKind::Next, 2)
        .expect("command");
    assert_eq!(plan.player_id, "player-1");
    assert_eq!(plan.kind, MediaCommandKind::Next);
    assert_eq!(plan.sequence, 2);
    let stale = bridge.command("player-1", MediaCommandKind::Pause, 2);
    assert!(matches!(stale, Err(KeynearError::Protocol(_))));
}

#[test]
fn oversized_or_unsafe_metadata_is_rejected() {
    let bad = metadata("player-1", "bad\nname", 1);
    assert!(matches!(
        validate_metadata(&bad),
        Err(KeynearError::Validation { .. })
    ));
    let bad = metadata("player-1", &"x".repeat(MAX_METADATA_LEN + 1), 1);
    assert!(matches!(
        validate_metadata(&bad),
        Err(KeynearError::Validation { .. })
    ));
    let bad = metadata("", "Track", 1);
    assert!(matches!(
        validate_metadata(&bad),
        Err(KeynearError::Validation { .. })
    ));
}
