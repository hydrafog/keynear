#![deny(
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    clippy::too_many_arguments
)]

pub mod error;
pub mod model;
pub mod proto;

pub use error::KeynearError;
pub use model::{
    default_config_dir, default_config_path, default_devices_path, sanitize_device_id,
    validate_display_name, DaemonConfig, DeviceRole, NearbyPeer, PairedDevice, PairedDeviceStore,
    TransferDirection, TransferItem, TransferSession, TransferState, VisibilityMode,
    EVERYONE_NEARBY_TIMEOUT_SECS, MAX_DISPLAY_NAME_LEN, PAIRING_TIMEOUT_SECS, QUIC_ALPN,
    STREAM_CHUNK_BYTES, ZIP_IO_BUFFER_BYTES,
};
pub use proto::{
    control_frame_to_envelope, decode_control_frame, decode_envelope, encode_control_frame,
    encode_envelope, envelope_to_control_frame, read_control_envelope, read_stream_kind, v1,
    write_control_envelope, write_stream_kind, ControlFrame, QuicStreamKind,
    MAX_CONTROL_FRAME_BYTES,
};
