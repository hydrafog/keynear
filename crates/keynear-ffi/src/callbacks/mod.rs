use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use keynear_core::{
    StaticKeypairRecord, KEYNEAR_BLE_SERVICE_UUID, KEYNEAR_BLE_SERVICE_UUID16,
    LAN_ENDPOINT_CHAR_UUID, LAN_ENDPOINT_CHAR_UUID16,
};
use keynear_protocol::{
    control_frame_to_envelope, decode_control_frame, v1, DaemonConfig, DeviceRole, KeynearError,
    PairedDevice, PairedDeviceStore, VisibilityMode, EVERYONE_NEARBY_TIMEOUT_SECS,
    MAX_CONTROL_FRAME_BYTES, PAIRING_TIMEOUT_SECS,
};

mod exports;
mod state;
mod types;

pub use exports::{
    ble_service_uuid16, ble_service_uuid_string, default_config_summary,
    everyone_nearby_timeout_secs, get_core_status, lan_endpoint_char_uuid16,
    lan_endpoint_char_uuid_string, pairing_timeout_secs,
};
pub(crate) use state::{
    cycle_visibility_mode_impl, generate_static_keypair_impl, load_daemon_config_impl,
    load_paired_devices_impl, remaining_secs_impl, remove_paired_device_impl,
    rename_paired_device_impl, save_daemon_config_impl, save_paired_device_impl,
    set_auto_accept_files_impl, set_visibility_mode_impl, to_key_32, validate_config_toml_impl,
    validate_control_frame_impl, visibility_mode_impl, VisibilityStateInner,
};
pub use types::{
    CoreStatus, FfiDaemonConfig, FfiDeviceRole, FfiError, FfiLanEndpoint, FfiPairedDevice,
    FfiVisibilityMode,
};
