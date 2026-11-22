use std::collections::HashMap;

use keynear_protocol::KeynearError;

pub const MAX_PLAYER_ID_LEN: usize = 64;
pub const MAX_METADATA_LEN: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackState {
    Playing,
    Paused,
    Stopped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaCommandKind {
    Play,
    Pause,
    Next,
    Previous,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaMetadata {
    pub player_id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub playback: PlaybackState,
    pub position_ms: u64,
    pub duration_ms: u64,
    pub sequence: u64,
    pub can_pause: bool,
    pub can_next: bool,
    pub can_previous: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaCommandPlan {
    pub player_id: String,
    pub kind: MediaCommandKind,
    pub sequence: u64,
}

#[derive(Debug, Default)]
pub struct MediaSessionBridge {
    players: HashMap<String, MediaMetadata>,
    last_command_sequence: u64,
}

impl MediaSessionBridge {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn observe(&mut self, metadata: MediaMetadata) -> Result<bool, KeynearError> {
        validate_metadata(&metadata)?;
        if self
            .players
            .get(&metadata.player_id)
            .is_some_and(|old| metadata.sequence <= old.sequence)
        {
            return Ok(false);
        }
        self.players.insert(metadata.player_id.clone(), metadata);
        Ok(true)
    }

    pub fn command(
        &mut self,
        player_id: &str,
        kind: MediaCommandKind,
        sequence: u64,
    ) -> Result<MediaCommandPlan, KeynearError> {
        if !self.players.contains_key(player_id) {
            return Err(KeynearError::Protocol(format!(
                "unknown media player '{player_id}'"
            )));
        }
        if sequence <= self.last_command_sequence {
            return Err(KeynearError::Protocol(
                "media command sequence is stale".to_string(),
            ));
        }
        self.last_command_sequence = sequence;
        Ok(MediaCommandPlan {
            player_id: player_id.to_string(),
            kind,
            sequence,
        })
    }

    pub fn snapshot(&self, player_id: &str) -> Option<&MediaMetadata> {
        self.players.get(player_id)
    }

    pub fn players(&self) -> impl Iterator<Item = &MediaMetadata> {
        self.players.values()
    }

    pub fn clear(&mut self) {
        self.players.clear();
    }
}

pub fn validate_metadata(metadata: &MediaMetadata) -> Result<(), KeynearError> {
    validate("player_id", &metadata.player_id, MAX_PLAYER_ID_LEN, false)?;
    validate("title", &metadata.title, MAX_METADATA_LEN, true)?;
    validate("artist", &metadata.artist, MAX_METADATA_LEN, true)?;
    validate("album", &metadata.album, MAX_METADATA_LEN, true)
}

fn validate(
    field: &'static str,
    value: &str,
    max: usize,
    allow_empty: bool,
) -> Result<(), KeynearError> {
    let empty = value.is_empty();
    if (!allow_empty && empty) || value.len() > max || value.chars().any(char::is_control) {
        let min = if allow_empty { 0 } else { 1 };
        return Err(KeynearError::Validation {
            field,
            message: format!("must be {min}..{max} safe characters"),
        });
    }
    Ok(())
}
