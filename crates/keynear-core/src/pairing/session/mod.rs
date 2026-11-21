use std::fmt;
use std::time::{Duration, Instant};

use rand::Rng;
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

use keynear_protocol::{sanitize_device_id, DeviceRole, KeynearError, PAIRING_TIMEOUT_SECS};

use super::crypto::{
    derive_reconnect_secret, verification_code_from_hash, SecretBytes32, EXPORT_CONTRIBUTION_LEN,
    MAX_DISPLAY_NAME_BYTES, MAX_GATT_PAIRING_FRAME_BYTES, METADATA_FORMAT_VERSION,
    PAIRING_PROTOCOL,
};

mod completed;
mod core;
mod export;
mod handshake;
mod metadata;

pub use completed::PairingCompleted;
pub use core::NoisePairingSession;
pub use metadata::{PairingPeerMetadata, PairingRole, PairingState};
