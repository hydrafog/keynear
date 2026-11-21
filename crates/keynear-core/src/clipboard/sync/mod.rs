use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use iroh::endpoint::{RecvStream, SendStream};
use sha2::{Digest, Sha256};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::mpsc;
use uuid::Uuid;

use keynear_protocol::{
    read_control_envelope, read_stream_kind, v1, write_control_envelope, write_stream_kind,
    KeynearError, QuicStreamKind, STREAM_CHUNK_BYTES,
};

use crate::transport::OrionQuicNode;

use super::history::{
    check_policy, is_image_mime, normalize_mime_type, pick_preferred_image_mime,
    prune_incoming_offers, prune_outgoing_offers, validate_mime_type, ClipboardItem,
    ClipboardPolicy, DelayedClipboardOffer, DelayedRenderCallback, IncomingOfferEntry,
    OutgoingOfferEntry, DEFAULT_TEXT_MIME, MAX_CLIPBOARD_IMAGE_BYTES, MAX_CLIPBOARD_MIME_BYTES,
    MAX_CLIPBOARD_QUEUE, MAX_INCOMING_OFFERS, MAX_INLINE_CLIPBOARD_BYTES, MAX_OFFER_MIME_COUNT,
    MAX_OUTGOING_OFFERS,
};

mod ack;
mod blob;
mod incoming;
mod incoming_helpers;
mod offers;
mod send;
mod state;

use ack::{await_control_ack, send_control_ack};
pub use blob::{read_clipboard_blob, write_clipboard_blob};
use incoming_helpers::{choose_fetch_mime, clean_offer_mimes, offer_metadata_valid};
pub use state::ClipboardEngine;
