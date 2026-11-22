mod bridge;

#[cfg(test)]
mod tests;

pub use bridge::{
    validate_metadata, MediaCommandKind, MediaCommandPlan, MediaMetadata, MediaSessionBridge,
    PlaybackState, MAX_METADATA_LEN, MAX_PLAYER_ID_LEN,
};
