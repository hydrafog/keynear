use std::fs::{self, File, OpenOptions};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use iroh::endpoint::{Connection, RecvStream};
use sha2::{Digest, Sha256};
use tokio::sync::mpsc;
use uuid::Uuid;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use keynear_protocol::{
    v1, write_control_envelope, write_stream_kind, KeynearError, QuicStreamKind, TransferItem,
    STREAM_CHUNK_BYTES, ZIP_IO_BUFFER_BYTES,
};

use super::framing::{
    await_transfer_consent, read_data_chunk, send_cancel_frame, send_completion_and_wait_ack,
    verify_completion_envelope, wrap_payload, write_data_chunk, ChannelChunkWriter,
    CompletionDetails, PIPE_CHANNEL_SLOTS,
};
use super::state::TransferRegistry;

const MAX_FILES_IN_ARCHIVE: usize = 10_000;
const MAX_ZIP_STREAM_OVERHEAD_BYTES: u64 = (MAX_FILES_IN_ARCHIVE as u64) * 512 + 65_536;
static TEMP_SEQ: AtomicU64 = AtomicU64::new(0);

mod paths;
mod plan;
mod receive_common;
mod receive_file;
mod receive_zip;
mod send;

pub(crate) use paths::expand_tilde;
pub use plan::sanitize_relative_path;
pub(crate) use plan::{prepare_send_plan, ArchiveSourceEntry, OutboundPayload};
pub(crate) use receive_file::receive_raw_file_stream;
pub(crate) use receive_zip::receive_zip_stream;
pub(crate) use send::{send_file_handle_stream, send_zip_stream};
