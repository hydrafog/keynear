use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

use crate::error::KeynearError;

pub const EVERYONE_NEARBY_TIMEOUT_SECS: u64 = 600;
pub const PAIRING_TIMEOUT_SECS: u64 = 120;
pub const STREAM_CHUNK_BYTES: usize = 61_440;
pub const ZIP_IO_BUFFER_BYTES: usize = 1_048_576;
pub const QUIC_ALPN: &[u8] = b"keynear/orion/1";
pub const MAX_DISPLAY_NAME_LEN: usize = 64;
const MAX_DEVICE_ID_BYTES: usize = 64;
static TEMP_WRITE_SEQ: AtomicU64 = AtomicU64::new(0);

mod io;
mod store;
mod types;
mod validation;

pub use io::{default_config_dir, default_config_path, default_devices_path};
pub use store::PairedDeviceStore;
pub use types::{
    DaemonConfig, DeviceRole, NearbyPeer, PairedDevice, TransferDirection, TransferItem,
    TransferSession, TransferState, VisibilityMode,
};
pub use validation::{sanitize_device_id, validate_display_name};

#[cfg(test)]
#[allow(clippy::too_many_lines, clippy::cognitive_complexity)]
mod tests;

#[cfg(test)]
mod mapping_tests;
