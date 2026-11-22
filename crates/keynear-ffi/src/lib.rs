#![deny(
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    clippy::too_many_arguments
)]

pub mod callbacks;
pub mod carina;
pub mod clipboard;
pub mod pairing;
pub mod perseus;
pub mod transfer;
pub mod vega;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use keynear_core::{
    parse_endpoint_id_bytes, ClipboardEngine, ClipboardPolicy, OrionNodeConfig, OrionQuicNode,
    TransferEngine,
};
use keynear_protocol::{read_stream_kind, KeynearError, QuicStreamKind};

pub use callbacks::{
    ble_service_uuid16, ble_service_uuid_string, default_config_summary,
    everyone_nearby_timeout_secs, get_core_status, lan_endpoint_char_uuid16,
    lan_endpoint_char_uuid_string, pairing_timeout_secs, CoreStatus, FfiDaemonConfig,
    FfiDeviceRole, FfiError, FfiLanEndpoint, FfiPairedDevice, FfiVisibilityMode,
};
pub use carina::{
    carina_call_action_allowed, carina_capability_flags, carina_media_command_allowed,
    carina_mirror_notification, carina_validate_reply, FfiCallAction, FfiCallPhase,
    FfiMediaCommand, FfiMediaState, FfiMirroredNotification, FfiNotificationAction,
    FfiNotificationDescriptor, FfiPlayback,
};
pub use clipboard::FfiClipboardItem;
pub use pairing::{
    FfiEveryoneNearbyBeacon, FfiNoisePairingSession, FfiPairedBeaconState, FfiPairingCompleted,
    FfiPairingRole, FfiPairingState, FfiPairingStep, FfiStaticKeypair,
};
pub use perseus::{
    perseus_capability_flags, perseus_read_plan, perseus_resume_decision, perseus_root_node,
    provide_vfs_read_fd, FfiChunkCheckpoint, FfiReadPlan, FfiResumeDecision, FfiVfsNode,
};
pub use transfer::{FfiTransferDirection, FfiTransferItem, FfiTransferSession, FfiTransferState};
pub use vega::{
    negotiate_vega_capabilities, select_vega_fallback, vega_capability_flags,
    FfiCapabilityNegotiation, FfiFallbackMedia, FfiFallbackSelection, FfiFallbackStatus,
    FfiRangingMethod, FfiVegaCapabilities,
};

use callbacks::{
    cycle_visibility_mode_impl, generate_static_keypair_impl, load_daemon_config_impl,
    load_paired_devices_impl, remaining_secs_impl, remove_paired_device_impl,
    rename_paired_device_impl, save_daemon_config_impl, save_paired_device_impl,
    set_auto_accept_files_impl, set_visibility_mode_impl, to_key_32, validate_config_toml_impl,
    validate_control_frame_impl, visibility_mode_impl, VisibilityStateInner,
};
use clipboard::{
    fetch_clipboard_offer_impl, poll_incoming_clipboard_impl, send_clipboard_blob_impl,
    send_clipboard_text_impl, set_clipboard_config_impl,
};
use pairing::{
    decode_everyone_nearby_beacon_impl, decode_lan_endpoint_impl, decode_paired_beacon_impl,
    derive_presence_token_bytes_impl, encode_everyone_nearby_beacon_impl, encode_lan_endpoint_impl,
    encode_paired_beacon_impl,
};
use transfer::send_paths_impl;
#[cfg(unix)]
use transfer::{provide_receive_fd_impl, send_fd_impl, SendFdParams};

uniffi::setup_scaffolding!();

mod core_handle;
mod node;
mod node_export;

pub use core_handle::CoreHandle;
pub use node::OrionNodeHandle;

#[cfg(test)]
#[allow(clippy::too_many_lines, clippy::cognitive_complexity)]
mod tests;
