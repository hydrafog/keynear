use std::net::{IpAddr, SocketAddr};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use mdns_sd::{DaemonEvent, ResolvedService, ScopedIp, ServiceDaemon, ServiceEvent, ServiceInfo};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use keynear_protocol::error::KeynearError;
use keynear_protocol::model::sanitize_device_id;
use keynear_protocol::proto::v1;

pub const KEYNEAR_MDNS_SERVICE_TYPE: &str = "_keynear._udp.local.";
const TXT_DEVICE_ID: &str = "id";
const TXT_ENDPOINT_ID: &str = "eid";
const TXT_CAPABILITY_FLAGS: &str = "cap";
const MDNS_CHANNEL_CAPACITY: usize = 32;

mod discovery;
mod events;
mod parse;

pub use discovery::{DiscoveredLanPeer, MdnsDiscovery};
pub use parse::parse_resolved_service;

#[cfg(test)]
#[allow(clippy::too_many_lines, clippy::cognitive_complexity)]
pub mod tests;
