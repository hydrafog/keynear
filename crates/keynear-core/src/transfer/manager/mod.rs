use std::collections::HashMap;
use std::fs::File;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use iroh::endpoint::{Connection, RecvStream, SendStream};
use tracing::{info, warn};
use uuid::Uuid;

use keynear_protocol::{
    read_control_envelope, read_stream_kind, sanitize_device_id, v1, write_control_envelope,
    KeynearError, QuicStreamKind, TransferItem, TransferSession, TransferState,
};

use crate::transport::OrionQuicNode;

use super::framing::{send_cancel_frame, send_consent_frame, wrap_payload, CONSENT_TIMEOUT_SECS};
use super::io::{
    expand_tilde, prepare_send_plan, receive_raw_file_stream, receive_zip_stream,
    sanitize_relative_path, send_file_handle_stream, send_zip_stream, OutboundPayload,
};
use super::state::{PeerTransferPolicy, TransferRegistry};

mod core;
mod incoming;
mod send;

pub use core::TransferEngine;
