use aes_gcm::{
    aead::{Aead, AeadInOut, Key, KeyInit, Payload},
    aes::{cipher::consts::U12, Aes256},
    Aes256Gcm, AesGcm, Nonce, Tag,
};
use hmac::{Hmac, KeyInit as HmacKeyInit, Mac};
use prost::Message;
use rand::Rng;
use sha2::Sha256;
use subtle::ConstantTimeEq;
use uuid::Uuid;

use keynear_protocol::error::KeynearError;
use keynear_protocol::model::{sanitize_device_id, DeviceRole};
use keynear_protocol::proto::{v1, MAX_CONTROL_FRAME_BYTES};

type HmacSha256 = Hmac<Sha256>;
type BleAes256Gcm = AesGcm<Aes256, U12, U12>;

pub const KEYNEAR_BLE_SERVICE_UUID16: u16 = 0xFE6D;
pub const KEYNEAR_BLE_SERVICE_UUID: Uuid =
    Uuid::from_u128(0x0000_FE6D_0000_1000_8000_0080_5F9B_34FB);
pub const LAN_ENDPOINT_CHAR_UUID16: u16 = 0xFE70;
pub const LAN_ENDPOINT_CHAR_UUID: Uuid = Uuid::from_u128(0x0000_FE70_0000_1000_8000_0080_5F9B_34FB);
pub const MAX_BLE_SERVICE_DATA_BYTES: usize = 24;
pub const MAX_LEGACY_ADV_BYTES: usize = 31;
pub const LEGACY_ADV_OVERHEAD_BYTES: usize = 7;
pub const MAX_COUNTER_SKEW: u32 = 8;
pub const PRESENCE_WINDOW_SECS: u64 = 900;
pub const PRESENCE_TOKEN_LEN: usize = 6;
pub const PAIRED_BEACON_LEN: usize = 24;
pub const EVERYONE_NEARBY_HEADER_LEN: usize = 7;
pub const MAX_EVERYONE_NEARBY_NAME_BYTES: usize =
    MAX_BLE_SERVICE_DATA_BYTES - EVERYONE_NEARBY_HEADER_LEN;

const BEACON_VERSION_PAIRED: u8 = 0x01;
const BEACON_VERSION_EVERYONE_NEARBY: u8 = 0x02;
const AVAILABILITY_ONLINE: u8 = 0x01;
const BATTERY_UNKNOWN: u8 = 0xFF;
const MAX_BATTERY_PERCENT: u8 = 100;
const PRESENCE_DOMAIN: &[u8] = b"keynear-presence-v1";
const NONCE_PREFIX: [u8; 8] = *b"KN_BLE1_";
const AAD_PREFIX: [u8; 3] = *b"KN1";
const LAN_ENDPOINT_VERSION: u8 = 0x01;
const LAN_ENDPOINT_AAD_PREFIX: &[u8] = b"KN_LAN1";
const LAN_ENDPOINT_NONCE_BYTES: usize = 12;
const LAN_ENDPOINT_TAG_BYTES: usize = 16;
const LAN_ENDPOINT_MIN_LEN: usize = 1 + LAN_ENDPOINT_NONCE_BYTES + LAN_ENDPOINT_TAG_BYTES;
const ED25519_PUBKEY_BYTES: usize = 32;

const _: () = assert!(PAIRED_BEACON_LEN == MAX_BLE_SERVICE_DATA_BYTES);
const _: () =
    assert!(MAX_BLE_SERVICE_DATA_BYTES + LEGACY_ADV_OVERHEAD_BYTES <= MAX_LEGACY_ADV_BYTES);
const _: () = assert!(
    EVERYONE_NEARBY_HEADER_LEN + MAX_EVERYONE_NEARBY_NAME_BYTES <= MAX_BLE_SERVICE_DATA_BYTES
);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairedBeaconState {
    pub counter: u32,
    pub available: bool,
    pub battery_level: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EveryoneNearbyBeacon {
    pub role: DeviceRole,
    pub capability_flags: u32,
    pub display_name: String,
}

pub fn derive_presence_token(
    device_id: &str,
    counter: u32,
    reconnect_key: &[u8; 32],
) -> Result<[u8; PRESENCE_TOKEN_LEN], KeynearError> {
    let clean_id = sanitize_device_id(device_id)?;
    let mut mac = <HmacSha256 as HmacKeyInit>::new_from_slice(reconnect_key)
        .map_err(|err| KeynearError::Crypto(format!("invalid hmac key: {err}")))?;
    mac.update(PRESENCE_DOMAIN);
    mac.update(clean_id.as_bytes());
    mac.update(&counter.to_be_bytes());
    let digest = mac.finalize().into_bytes();
    let mut token = [0u8; PRESENCE_TOKEN_LEN];
    token.copy_from_slice(&digest[..PRESENCE_TOKEN_LEN]);
    Ok(token)
}

pub struct PairedBeacon;
pub struct LanEndpointCodec;

mod beacon;
mod everyone;
mod lan_endpoint;

#[cfg(test)]
#[allow(clippy::too_many_lines, clippy::cognitive_complexity)]
mod tests;
