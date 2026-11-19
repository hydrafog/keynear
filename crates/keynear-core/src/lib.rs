pub mod clipboard;
pub mod keyring;
pub mod pairing;
pub mod transfer;
pub mod transport;

pub use clipboard::{
    is_image_mime, normalize_mime_type, pick_preferred_image_mime, read_clipboard_blob,
    should_ignore_mime_offers, write_clipboard_blob, ClipboardEngine, ClipboardItem,
    ClipboardPolicy, DelayedClipboardOffer, DelayedRenderCallback, BUILT_IN_IGNORED_MIMES,
    CLIPBOARD_OFFER_TTL_SECS, DEFAULT_TEXT_MIME, MAX_CLIPBOARD_IMAGE_BYTES, MAX_CLIPBOARD_QUEUE,
    MAX_INLINE_CLIPBOARD_BYTES,
};
pub use keynear_protocol::{
    control_frame_to_envelope, decode_control_frame, decode_envelope, default_config_dir,
    default_config_path, default_devices_path, encode_control_frame, encode_envelope,
    envelope_to_control_frame, read_control_envelope, read_stream_kind, sanitize_device_id, v1,
    validate_display_name, write_control_envelope, write_stream_kind, ControlFrame, DaemonConfig,
    DeviceRole, KeynearError, NearbyPeer, PairedDevice, PairedDeviceStore, QuicStreamKind,
    TransferDirection, TransferItem, TransferSession, TransferState, VisibilityMode,
    EVERYONE_NEARBY_TIMEOUT_SECS, MAX_CONTROL_FRAME_BYTES, MAX_DISPLAY_NAME_LEN,
    PAIRING_TIMEOUT_SECS, QUIC_ALPN, STREAM_CHUNK_BYTES, ZIP_IO_BUFFER_BYTES,
};
pub use keyring::{
    commit_paired_device, remove_paired_device, InMemorySecretStore, PeerSecrets, SecretStore,
    StaticKeypairRecord,
};
pub use pairing::{
    NoisePairingSession, PairingCompleted, PairingPeerMetadata, PairingRole, PairingState,
    EXPORT_CONTRIBUTION_LEN, MAX_GATT_PAIRING_FRAME_BYTES, PAIRING_PROTOCOL,
};
pub use transfer::{
    sanitize_relative_path, TransferEngine, TransferRegistry, MAX_ACTIVE_TRANSFERS,
};
pub use transport::{
    build_orion_crypto_provider, compute_handshake_auth_proof, derive_presence_token,
    parse_endpoint_id_bytes, parse_endpoint_id_hex, verify_handshake_auth_proof, DiscoveredLanPeer,
    EveryoneNearbyBeacon, LanEndpointCodec, MdnsDiscovery, OrionNodeConfig, OrionQuicNode,
    PairedBeacon, PairedBeaconState, KEYNEAR_BLE_SERVICE_UUID, KEYNEAR_BLE_SERVICE_UUID16,
    KEYNEAR_MDNS_SERVICE_TYPE, LAN_ENDPOINT_CHAR_UUID, LAN_ENDPOINT_CHAR_UUID16,
    MAX_BLE_SERVICE_DATA_BYTES, MAX_COUNTER_SKEW, MAX_LEGACY_ADV_BYTES, PAIRED_BEACON_LEN,
    PRESENCE_WINDOW_SECS,
};
