use std::collections::HashMap;
use std::fs::File;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use tokio::sync::oneshot;

use keynear_protocol::{
    sanitize_device_id, v1, KeynearError, TransferDirection, TransferItem, TransferSession,
    TransferState,
};

use super::framing::validate_offer;

mod registry;
mod types;
mod util;

pub use types::{InboundRegistration, PeerTransferPolicy, TransferRegistry, MAX_ACTIVE_TRANSFERS};
pub(crate) use util::now_millis;
