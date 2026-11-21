#![deny(
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    clippy::too_many_arguments
)]

use std::fs;
use std::io::{Read, Write};
use std::os::unix::fs::{FileTypeExt, PermissionsExt};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

use keynear_protocol::{
    default_config_dir, sanitize_device_id, validate_display_name, DaemonConfig, DeviceRole,
    KeynearError, PairedDevice, TransferSession, VisibilityMode,
};

mod codec;
mod config;
mod request;
mod response;
mod socket;
mod types;

pub use codec::validate_transfer_id;
pub use config::MAX_IPC_FRAME_BYTES;
pub use request::{decode_request, encode_request};
pub use response::{decode_response, encode_response};
pub use socket::{
    daemon_socket_path, prepare_daemon_socket_dir, send_request, send_request_to,
    set_daemon_socket_permissions,
};
pub use types::{DaemonStatus, IpcRequest, IpcResponse};

#[cfg(test)]
#[allow(clippy::too_many_lines, clippy::cognitive_complexity)]
mod tests;
