use std::collections::HashMap;
use std::fmt;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

use hmac::{Hmac, KeyInit, Mac};
use iroh::endpoint::{presets, AfterHandshakeOutcome, Connection, Endpoint, EndpointHooks};
use iroh::{EndpointAddr, EndpointId, RelayMode, SecretKey, TransportAddr};
use rustls::crypto::{aws_lc_rs, CryptoProvider};
use sha2::Sha256;
use subtle::ConstantTimeEq;
use tokio::sync::{mpsc, Mutex as AsyncMutex};
use tracing::{info, warn};
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

use crate::keyring::SecretStore;
use keynear_protocol::error::KeynearError;
use keynear_protocol::model::{sanitize_device_id, PairedDevice, QUIC_ALPN};
use keynear_protocol::proto::{
    read_control_envelope, read_stream_kind, v1, write_control_envelope, write_stream_kind,
    QuicStreamKind,
};

type HmacSha256 = Hmac<Sha256>;

const HANDSHAKE_AUTH_DOMAIN: &[u8] = b"keynear-orion-handshake-v1";
const QUIC_UNAUTHORIZED_CODE: u32 = 403;

mod auth;
mod config;
mod connect;
mod handshake;
mod node;
mod util;

pub use auth::{
    build_orion_crypto_provider, compute_handshake_auth_proof, verify_handshake_auth_proof,
};
pub use config::OrionNodeConfig;
pub use node::OrionQuicNode;
pub use util::{parse_endpoint_id_bytes, parse_endpoint_id_hex};

#[cfg(test)]
#[allow(clippy::too_many_lines, clippy::cognitive_complexity)]
pub mod tests;
