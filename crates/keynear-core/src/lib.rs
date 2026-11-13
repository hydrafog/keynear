pub mod error;
pub mod model;
pub mod proto;
pub mod uniffi_bridge;

uniffi::setup_scaffolding!();

pub use error::KeynearError;
pub use model::{
    default_config_path, DaemonConfig, DeviceRole, NearbyPeer, PairedDevice, TransferDirection,
    TransferItem, TransferSession, TransferState, VisibilityMode,
};
pub use proto::{
    control_frame_to_envelope, decode_control_frame, decode_envelope, encode_control_frame,
    encode_envelope, envelope_to_control_frame, v1, ControlFrame, MAX_CONTROL_FRAME_BYTES,
};
pub use uniffi_bridge::{
    default_config_summary, get_core_status, CoreHandle, CoreStatus, FfiError, FfiVisibilityMode,
};
