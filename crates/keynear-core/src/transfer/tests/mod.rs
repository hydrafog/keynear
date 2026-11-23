use std::fs;
use std::io::{Cursor, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use iroh::endpoint::Connection;
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use keynear_protocol::{
    v1, write_control_envelope, write_stream_kind, KeynearError, QuicStreamKind, TransferState,
    STREAM_CHUNK_BYTES,
};

use crate::transport::{OrionNodeConfig, OrionQuicNode};

use super::framing::{
    await_transfer_consent, send_completion_and_wait_ack, wrap_payload, write_data_chunk,
    CompletionDetails,
};
use super::*;

mod cases;
mod folders;
mod setup;
