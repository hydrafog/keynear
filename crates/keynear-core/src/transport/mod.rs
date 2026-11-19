pub mod lan_discovery;
pub mod presence;
pub mod quic;

pub use lan_discovery::{DiscoveredLanPeer, MdnsDiscovery, KEYNEAR_MDNS_SERVICE_TYPE};
pub use presence::{
    derive_presence_token, EveryoneNearbyBeacon, LanEndpointCodec, PairedBeacon, PairedBeaconState,
    KEYNEAR_BLE_SERVICE_UUID, KEYNEAR_BLE_SERVICE_UUID16, LAN_ENDPOINT_CHAR_UUID,
    LAN_ENDPOINT_CHAR_UUID16, MAX_BLE_SERVICE_DATA_BYTES, MAX_COUNTER_SKEW, MAX_LEGACY_ADV_BYTES,
    PAIRED_BEACON_LEN, PRESENCE_WINDOW_SECS,
};
pub use quic::{
    build_orion_crypto_provider, compute_handshake_auth_proof, parse_endpoint_id_bytes,
    parse_endpoint_id_hex, verify_handshake_auth_proof, OrionNodeConfig, OrionQuicNode,
};
