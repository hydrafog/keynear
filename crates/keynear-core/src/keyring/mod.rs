use std::collections::BTreeMap;
use std::fmt;
use std::sync::Mutex;

use rand::Rng;
use x25519_dalek::{PublicKey, StaticSecret};
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

use keynear_protocol::error::KeynearError;
use keynear_protocol::model::{sanitize_device_id, PairedDevice, PairedDeviceStore};

mod memory;
mod operations;
mod types;

pub use memory::InMemorySecretStore;
pub use operations::{commit_paired_device, remove_paired_device};
pub use types::{PeerSecrets, SecretStore, StaticKeypairRecord};

#[cfg(test)]
#[allow(clippy::too_many_lines, clippy::cognitive_complexity)]
mod tests;
