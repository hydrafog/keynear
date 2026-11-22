use keynear_core::{KeynearError, MediaCommandKind, PlaybackState};
use tracing::{info, warn};
use zbus::Variant;

const MPRIS_PREFIX: &str = "org.mpris.MediaPlayer2.";
const MPRIS_PATH: &str = "/org/mpris/MediaPlayer2";
const MPRIS_PLAYER: &str = "org.mpris.MediaPlayer2.Player";
const DBUS_PROPERTIES: &str = "org.freedesktop.DBus.Properties";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaSnapshot {
    pub player_id: String,
    pub title: String,
    pub artist: String,
    pub playback: PlaybackState,
    pub sequence: u64,
    pub can_pause: bool,
    pub can_next: bool,
}

pub fn playback_from_status(status: &str) -> PlaybackState {
    match status {
        "Playing" => PlaybackState::Playing,
        "Paused" => PlaybackState::Paused,
        _ => PlaybackState::Stopped,
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SnapshotInput<'a> {
    pub player_id: &'a str,
    pub title: &'a str,
    pub artist: &'a str,
    pub status: &'a str,
    pub sequence: u64,
}

pub fn snapshot(input: SnapshotInput<'_>) -> MediaSnapshot {
    MediaSnapshot {
        player_id: input.player_id.to_string(),
        title: input.title.to_string(),
        artist: input.artist.to_string(),
        playback: playback_from_status(input.status),
        sequence: input.sequence,
        can_pause: true,
        can_next: true,
    }
}

fn backend_error(err: impl std::fmt::Display) -> KeynearError {
    KeynearError::Protocol(format!("mpris media backend: {err}"))
}

async fn session_bus() -> Result<zbus::Connection, KeynearError> {
    zbus::Connection::session().await.map_err(backend_error)
}

async fn player_names(conn: &zbus::Connection) -> Result<Vec<String>, KeynearError> {
    let reply = conn
        .call_method(
            Some("org.freedesktop.DBus"),
            "/org/freedesktop/DBus",
            Some("org.freedesktop.DBus"),
            "ListNames",
            &(),
        )
        .await
        .map_err(backend_error)?;
    let names: Vec<String> = reply.body().deserialize().map_err(backend_error)?;
    Ok(names
        .into_iter()
        .filter(|name| name.starts_with(MPRIS_PREFIX))
        .collect())
}

async fn read_snapshot(
    conn: &zbus::Connection,
    player: &str,
    sequence: u64,
) -> Result<MediaSnapshot, KeynearError> {
    let status_reply = conn
        .call_method(
            Some(player),
            MPRIS_PATH,
            Some(DBUS_PROPERTIES),
            "Get",
            &(MPRIS_PLAYER, "PlaybackStatus"),
        )
        .await
        .map_err(backend_error)?;
    let status_body = status_reply.body();
    let status_value: Variant<'_> = status_body.deserialize().map_err(backend_error)?;
    let status: String = status_value
        .downcast_ref::<&str>()
        .map(|value| (*value).to_string())
        .map_err(backend_error)?;

    Ok(snapshot(SnapshotInput {
        player_id: player,
        title: "",
        artist: "",
        status: &status,
        sequence,
    }))
}

fn log_media_unavailable(reason: &'static str) {
    warn!(event = "media_bridge_unavailable", reason = reason);
}

fn log_media_player_unavailable(err: &KeynearError) {
    warn!(event = "media_player_unavailable", error = %err);
}

fn log_media_command_unavailable(reason: &'static str) {
    warn!(event = "media_command_unavailable", reason = reason);
}

fn log_media_command(delivered: usize) {
    info!(event = "media_command_dispatched", players = delivered);
}

pub async fn observe(sequence: u64) -> Vec<MediaSnapshot> {
    let Ok(conn) = session_bus().await else {
        log_media_unavailable("session_bus_absent");
        return Vec::new();
    };
    let Ok(names) = player_names(&conn).await else {
        log_media_unavailable("player_list_failed");
        return Vec::new();
    };
    let mut snapshots = Vec::new();
    for name in names {
        match read_snapshot(&conn, &name, sequence).await {
            Ok(value) => snapshots.push(value),
            Err(err) => log_media_player_unavailable(&err),
        }
    }
    snapshots
}

async fn dispatch_to_players(conn: &zbus::Connection, players: &[String], method: &str) -> usize {
    let mut delivered = 0;
    for player in players {
        let result = conn
            .call_method(
                Some(player.as_str()),
                MPRIS_PATH,
                Some(MPRIS_PLAYER),
                method,
                &(),
            )
            .await;
        if result.is_ok() {
            delivered += 1;
        }
    }
    delivered
}

pub async fn dispatch_command(kind: MediaCommandKind) -> usize {
    let method = match kind {
        MediaCommandKind::Play => "Play",
        MediaCommandKind::Pause => "Pause",
        MediaCommandKind::Next => "Next",
        MediaCommandKind::Previous => "Previous",
    };
    let Ok(conn) = session_bus().await else {
        log_media_command_unavailable("session_bus_absent");
        return 0;
    };
    let Ok(players) = player_names(&conn).await else {
        log_media_command_unavailable("player_list_failed");
        return 0;
    };
    let delivered = dispatch_to_players(&conn, &players, method).await;
    log_media_command(delivered);
    delivered
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mpris_status_maps_to_playback_state() {
        assert_eq!(playback_from_status("Playing"), PlaybackState::Playing);
        assert_eq!(playback_from_status("Paused"), PlaybackState::Paused);
        assert_eq!(playback_from_status("Stopped"), PlaybackState::Stopped);
        assert_eq!(playback_from_status("unknown"), PlaybackState::Stopped);
    }

    #[test]
    fn snapshot_carries_title_artist_and_capabilities() {
        let value = snapshot(SnapshotInput {
            player_id: "org.mpris.MediaPlayer2.vlc",
            title: "Track",
            artist: "Artist",
            status: "Playing",
            sequence: 4,
        });
        assert_eq!(value.title, "Track");
        assert_eq!(value.artist, "Artist");
        assert_eq!(value.playback, PlaybackState::Playing);
        assert_eq!(value.player_id, "org.mpris.MediaPlayer2.vlc");
        assert!(value.can_pause && value.can_next);
    }
}
