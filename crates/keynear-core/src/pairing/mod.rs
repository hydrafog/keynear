pub mod crypto;
pub mod session;

pub use crypto::{EXPORT_CONTRIBUTION_LEN, MAX_GATT_PAIRING_FRAME_BYTES, PAIRING_PROTOCOL};
pub use session::{
    NoisePairingSession, PairingCompleted, PairingPeerMetadata, PairingRole, PairingState,
};

#[cfg(test)]
#[allow(clippy::too_many_lines, clippy::cognitive_complexity)]
mod tests;
