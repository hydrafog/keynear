pub mod error;
pub mod keyring;
pub mod model;
#[cfg(feature = "linux-oo7")]
pub mod oo7_keyring;
pub mod pairing;
pub mod presence;
pub mod proto;
pub mod uniffi_bridge;

uniffi::setup_scaffolding!();

pub use error::KeynearError;
pub use keyring::{
    commit_paired_device, remove_paired_device, InMemorySecretStore, PeerSecrets, SecretStore,
    StaticKeypairRecord,
};
pub use model::{
    default_config_dir, default_config_path, default_devices_path, sanitize_device_id,
    DaemonConfig, DeviceRole, NearbyPeer, PairedDevice, PairedDeviceStore, TransferDirection,
    TransferItem, TransferSession, TransferState, VisibilityMode, EVERYONE_NEARBY_TIMEOUT_SECS,
    PAIRING_TIMEOUT_SECS,
};
#[cfg(feature = "linux-oo7")]
pub use oo7_keyring::Oo7SecretStore;
pub use pairing::{
    NoisePairingSession, PairingCompleted, PairingPeerMetadata, PairingRole, PairingState,
    EXPORT_CONTRIBUTION_LEN, MAX_GATT_PAIRING_FRAME_BYTES, PAIRING_PROTOCOL,
};
pub use presence::{
    derive_presence_token, EveryoneNearbyBeacon, PairedBeacon, PairedBeaconState,
    KEYNEAR_BLE_SERVICE_UUID, KEYNEAR_BLE_SERVICE_UUID16, MAX_BLE_SERVICE_DATA_BYTES,
    MAX_COUNTER_SKEW, MAX_LEGACY_ADV_BYTES, PAIRED_BEACON_LEN, PRESENCE_WINDOW_SECS,
};
pub use proto::{
    control_frame_to_envelope, decode_control_frame, decode_envelope, encode_control_frame,
    encode_envelope, envelope_to_control_frame, v1, ControlFrame, MAX_CONTROL_FRAME_BYTES,
};
pub use uniffi_bridge::{
    default_config_summary, get_core_status, CoreHandle, CoreStatus, FfiError, FfiVisibilityMode,
};
