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

use crate::error::KeynearError;
use crate::model::{sanitize_device_id, DeviceRole};
use crate::proto::{v1, MAX_CONTROL_FRAME_BYTES};

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

fn build_beacon_nonce(counter_be: [u8; 4]) -> [u8; 12] {
    let mut nonce = [0u8; 12];
    nonce[..8].copy_from_slice(&NONCE_PREFIX);
    nonce[8..12].copy_from_slice(&counter_be);
    nonce
}

fn build_beacon_aad(token: &[u8; PRESENCE_TOKEN_LEN], counter_be: [u8; 4]) -> [u8; 14] {
    let mut aad = [0u8; 14];
    aad[..3].copy_from_slice(&AAD_PREFIX);
    aad[3] = BEACON_VERSION_PAIRED;
    aad[4..10].copy_from_slice(token);
    aad[10..14].copy_from_slice(&counter_be);
    aad
}

pub struct PairedBeacon;

impl PairedBeacon {
    pub fn encode(
        device_id: &str,
        counter: u32,
        reconnect_key: &[u8; 32],
        available: bool,
        battery_level: Option<u8>,
    ) -> Result<[u8; PAIRED_BEACON_LEN], KeynearError> {
        let token = derive_presence_token(device_id, counter, reconnect_key)?;
        let counter_be = counter.to_be_bytes();
        let nonce_bytes = build_beacon_nonce(counter_be);
        let aad = build_beacon_aad(&token, counter_be);

        let battery_byte = match battery_level {
            Some(level) => level.min(MAX_BATTERY_PERCENT),
            None => BATTERY_UNKNOWN,
        };
        let mut buf = [
            if available { AVAILABILITY_ONLINE } else { 0 },
            battery_byte,
        ];

        let key = Key::<BleAes256Gcm>::from(*reconnect_key);
        let cipher = BleAes256Gcm::new(&key);
        let tag = cipher
            .encrypt_inout_detached(
                &Nonce::<U12>::from(nonce_bytes),
                &aad,
                buf.as_mut_slice().into(),
            )
            .map_err(|_| KeynearError::Crypto("paired beacon encryption failed".to_string()))?;

        let mut out = [0u8; PAIRED_BEACON_LEN];
        out[0..6].copy_from_slice(&token);
        out[6..10].copy_from_slice(&counter_be);
        out[10..12].copy_from_slice(&buf);
        out[12..24].copy_from_slice(tag.as_slice());
        Ok(out)
    }

    pub fn decode_and_verify(
        service_data: &[u8],
        device_id: &str,
        expected_counter: u32,
        last_accepted_counter: Option<u32>,
        reconnect_key: &[u8; 32],
    ) -> Result<PairedBeaconState, KeynearError> {
        if service_data.len() != PAIRED_BEACON_LEN {
            return Err(KeynearError::Validation {
                field: "serviceData",
                message: format!(
                    "paired beacon length {} must equal {} bytes",
                    service_data.len(),
                    PAIRED_BEACON_LEN
                ),
            });
        }

        let mut counter_be = [0u8; 4];
        counter_be.copy_from_slice(&service_data[6..10]);
        let counter = u32::from_be_bytes(counter_be);

        let min_counter = expected_counter.saturating_sub(MAX_COUNTER_SKEW);
        let max_counter = expected_counter.saturating_add(MAX_COUNTER_SKEW);
        if counter < min_counter || counter > max_counter {
            return Err(KeynearError::Crypto(
                "presence counter outside skew window".to_string(),
            ));
        }
        if let Some(last) = last_accepted_counter {
            if counter <= last {
                return Err(KeynearError::Crypto(
                    "replayed or stale presence counter".to_string(),
                ));
            }
        }

        let expected_token = derive_presence_token(device_id, counter, reconnect_key)?;
        if !bool::from(service_data[0..6].ct_eq(&expected_token)) {
            return Err(KeynearError::Crypto("presence token mismatch".to_string()));
        }

        let nonce_bytes = build_beacon_nonce(counter_be);
        let aad = build_beacon_aad(&expected_token, counter_be);
        let mut plaintext = [service_data[10], service_data[11]];
        let mut tag_bytes = [0u8; 12];
        tag_bytes.copy_from_slice(&service_data[12..24]);

        let key = Key::<BleAes256Gcm>::from(*reconnect_key);
        let cipher = BleAes256Gcm::new(&key);
        cipher
            .decrypt_inout_detached(
                &Nonce::<U12>::from(nonce_bytes),
                &aad,
                plaintext.as_mut_slice().into(),
                &Tag::<U12>::from(tag_bytes),
            )
            .map_err(|_| KeynearError::Crypto("paired beacon authentication failed".to_string()))?;

        let available = (plaintext[0] & AVAILABILITY_ONLINE) != 0;
        let battery_level = if plaintext[1] == BATTERY_UNKNOWN {
            None
        } else {
            Some(plaintext[1].min(MAX_BATTERY_PERCENT))
        };

        Ok(PairedBeaconState {
            counter,
            available,
            battery_level,
        })
    }
}

fn truncate_utf8(input: &str, max_bytes: usize) -> &str {
    if input.len() <= max_bytes {
        return input;
    }
    let mut end = max_bytes;
    while end > 0 && !input.is_char_boundary(end) {
        end -= 1;
    }
    &input[..end]
}

impl EveryoneNearbyBeacon {
    pub fn encode(&self) -> Result<Vec<u8>, KeynearError> {
        let trimmed = self.display_name.trim();
        if trimmed.is_empty() {
            return Err(KeynearError::Validation {
                field: "displayName",
                message: "display_name must not be empty".to_string(),
            });
        }
        let truncated = truncate_utf8(trimmed, MAX_EVERYONE_NEARBY_NAME_BYTES);
        if truncated.is_empty() {
            return Err(KeynearError::Validation {
                field: "displayName",
                message: "display_name does not fit within BLE beacon budget".to_string(),
            });
        }

        let role_byte = match self.role {
            DeviceRole::Desktop => 0x01,
            DeviceRole::Mobile => 0x02,
        };
        let name_bytes = truncated.as_bytes();
        let mut out = Vec::with_capacity(EVERYONE_NEARBY_HEADER_LEN + name_bytes.len());
        out.push(BEACON_VERSION_EVERYONE_NEARBY);
        out.push(role_byte);
        out.extend_from_slice(&self.capability_flags.to_le_bytes());
        out.push(name_bytes.len() as u8);
        out.extend_from_slice(name_bytes);
        debug_assert!(out.len() <= MAX_BLE_SERVICE_DATA_BYTES);
        Ok(out)
    }

    pub fn decode(service_data: &[u8]) -> Result<Self, KeynearError> {
        if service_data.len() < EVERYONE_NEARBY_HEADER_LEN
            || service_data.len() > MAX_BLE_SERVICE_DATA_BYTES
        {
            return Err(KeynearError::Validation {
                field: "serviceData",
                message: format!(
                    "everyone-nearby beacon length {} out of bounds",
                    service_data.len()
                ),
            });
        }
        if service_data[0] != BEACON_VERSION_EVERYONE_NEARBY {
            return Err(KeynearError::Protocol(format!(
                "unsupported everyone-nearby beacon version {}",
                service_data[0]
            )));
        }
        let role = match service_data[1] {
            0x01 => DeviceRole::Desktop,
            0x02 => DeviceRole::Mobile,
            other => {
                return Err(KeynearError::Protocol(format!(
                    "unknown device role byte {other}"
                )));
            }
        };
        let mut cap_bytes = [0u8; 4];
        cap_bytes.copy_from_slice(&service_data[2..6]);
        let capability_flags = u32::from_le_bytes(cap_bytes);
        let name_len = service_data[6] as usize;
        if name_len == 0 || EVERYONE_NEARBY_HEADER_LEN + name_len != service_data.len() {
            return Err(KeynearError::Validation {
                field: "displayName",
                message: "invalid everyone-nearby beacon display_name length".to_string(),
            });
        }
        let display_name = std::str::from_utf8(&service_data[7..])
            .map_err(|_| {
                KeynearError::Protocol("invalid utf-8 in everyone-nearby beacon".to_string())
            })?
            .trim()
            .to_string();
        if display_name.is_empty() {
            return Err(KeynearError::Validation {
                field: "displayName",
                message: "display_name must not be empty".to_string(),
            });
        }
        Ok(Self {
            role,
            capability_flags,
            display_name,
        })
    }
}

pub struct LanEndpointCodec;

impl LanEndpointCodec {
    pub fn encode(
        endpoint: &v1::LanEndpoint,
        reconnect_key: &[u8; 32],
    ) -> Result<Vec<u8>, KeynearError> {
        let clean_id = sanitize_device_id(&endpoint.device_id)?;
        if endpoint.endpoint_id.len() != ED25519_PUBKEY_BYTES {
            return Err(KeynearError::Validation {
                field: "endpointId",
                message: "endpoint_id must be 32 bytes".to_string(),
            });
        }
        let mut normalized = endpoint.clone();
        normalized.device_id = clean_id.clone();

        let plaintext = normalized.encode_to_vec();
        if plaintext.len() > MAX_CONTROL_FRAME_BYTES {
            return Err(KeynearError::FrameTooLarge {
                size: plaintext.len(),
                max: MAX_CONTROL_FRAME_BYTES,
            });
        }

        let mut nonce_bytes = [0u8; LAN_ENDPOINT_NONCE_BYTES];
        rand::rng().fill_bytes(&mut nonce_bytes);

        let mut aad = Vec::with_capacity(LAN_ENDPOINT_AAD_PREFIX.len() + clean_id.len());
        aad.extend_from_slice(LAN_ENDPOINT_AAD_PREFIX);
        aad.extend_from_slice(clean_id.as_bytes());

        let cipher = Aes256Gcm::new(&Key::<Aes256Gcm>::from(*reconnect_key));
        let ciphertext = cipher
            .encrypt(
                &Nonce::from(nonce_bytes),
                Payload {
                    msg: &plaintext,
                    aad: &aad,
                },
            )
            .map_err(|_| KeynearError::Crypto("lan endpoint encryption failed".to_string()))?;

        let mut out = Vec::with_capacity(1 + LAN_ENDPOINT_NONCE_BYTES + ciphertext.len());
        out.push(LAN_ENDPOINT_VERSION);
        out.extend_from_slice(&nonce_bytes);
        out.extend_from_slice(&ciphertext);
        Ok(out)
    }

    pub fn decode(
        encoded: &[u8],
        expected_device_id: &str,
        reconnect_key: &[u8; 32],
    ) -> Result<v1::LanEndpoint, KeynearError> {
        let clean_id = sanitize_device_id(expected_device_id)?;
        if encoded.len() < LAN_ENDPOINT_MIN_LEN {
            return Err(KeynearError::Validation {
                field: "lanEndpoint",
                message: "encrypted lan endpoint payload too short".to_string(),
            });
        }
        if encoded.len() > MAX_CONTROL_FRAME_BYTES {
            return Err(KeynearError::FrameTooLarge {
                size: encoded.len(),
                max: MAX_CONTROL_FRAME_BYTES,
            });
        }
        if encoded[0] != LAN_ENDPOINT_VERSION {
            return Err(KeynearError::Protocol(format!(
                "unsupported lan endpoint codec version {}",
                encoded[0]
            )));
        }

        let mut nonce_bytes = [0u8; LAN_ENDPOINT_NONCE_BYTES];
        nonce_bytes.copy_from_slice(&encoded[1..1 + LAN_ENDPOINT_NONCE_BYTES]);
        let ciphertext = &encoded[1 + LAN_ENDPOINT_NONCE_BYTES..];

        let mut aad = Vec::with_capacity(LAN_ENDPOINT_AAD_PREFIX.len() + clean_id.len());
        aad.extend_from_slice(LAN_ENDPOINT_AAD_PREFIX);
        aad.extend_from_slice(clean_id.as_bytes());

        let cipher = Aes256Gcm::new(&Key::<Aes256Gcm>::from(*reconnect_key));
        let plaintext = cipher
            .decrypt(
                &Nonce::from(nonce_bytes),
                Payload {
                    msg: ciphertext,
                    aad: &aad,
                },
            )
            .map_err(|_| KeynearError::Crypto("lan endpoint authentication failed".to_string()))?;

        let decoded = v1::LanEndpoint::decode(plaintext.as_slice())
            .map_err(|err| KeynearError::Protocol(err.to_string()))?;
        if decoded.device_id != clean_id {
            return Err(KeynearError::Crypto(
                "lan endpoint device_id mismatch".to_string(),
            ));
        }
        if decoded.endpoint_id.len() != ED25519_PUBKEY_BYTES {
            return Err(KeynearError::Validation {
                field: "endpointId",
                message: "decoded endpoint_id must be 32 bytes".to_string(),
            });
        }
        Ok(decoded)
    }

    pub fn decode_any<'a, I>(encoded: &[u8], candidates: I) -> Result<v1::LanEndpoint, KeynearError>
    where
        I: IntoIterator<Item = (&'a str, &'a [u8; 32])>,
    {
        for (device_id, reconnect_key) in candidates {
            if let Ok(decoded) = Self::decode(encoded, device_id, reconnect_key) {
                return Ok(decoded);
            }
        }
        Err(KeynearError::UnauthorizedPeer(
            "no paired peer authenticated encrypted lan endpoint".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_key() -> [u8; 32] {
        [0x42; 32]
    }

    #[test]
    fn beacon_fits_31_byte_ble_budget_and_rejects_oversized() {
        let key = test_key();
        let encoded =
            PairedBeacon::encode("pixel-9", 1, &key, true, Some(84)).expect("encode paired beacon");
        assert_eq!(encoded.len(), 24);
        assert!(encoded.len() + LEGACY_ADV_OVERHEAD_BYTES <= MAX_LEGACY_ADV_BYTES);

        let decoded = PairedBeacon::decode_and_verify(&encoded, "pixel-9", 1, None, &key)
            .expect("decode valid paired beacon");
        assert_eq!(
            decoded,
            PairedBeaconState {
                counter: 1,
                available: true,
                battery_level: Some(84),
            }
        );

        let mut oversized = vec![0u8; MAX_BLE_SERVICE_DATA_BYTES + 1];
        oversized[..24].copy_from_slice(&encoded);
        assert!(matches!(
            PairedBeacon::decode_and_verify(&oversized, "pixel-9", 1, None, &key),
            Err(KeynearError::Validation {
                field: "serviceData",
                ..
            })
        ));

        let long_name = "Workstation-Desktop-UTF8-😀😀😀-VeryLongName";
        let open = EveryoneNearbyBeacon {
            role: DeviceRole::Desktop,
            capability_flags: 0x07,
            display_name: long_name.to_string(),
        };
        let open_bytes = open.encode().expect("encode everyone-nearby beacon");
        assert!(open_bytes.len() <= MAX_BLE_SERVICE_DATA_BYTES);
        assert!(open_bytes.len() + LEGACY_ADV_OVERHEAD_BYTES <= MAX_LEGACY_ADV_BYTES);
        let decoded_open =
            EveryoneNearbyBeacon::decode(&open_bytes).expect("decode everyone-nearby beacon");
        assert_eq!(decoded_open.role, DeviceRole::Desktop);
        assert_eq!(decoded_open.capability_flags, 0x07);
        assert!(long_name.starts_with(&decoded_open.display_name));

        assert!(EveryoneNearbyBeacon::decode(&oversized).is_err());
    }

    #[test]
    fn rejects_stale_replayed_and_tampered_tokens() {
        let key = test_key();
        let encoded = PairedBeacon::encode("thinkpad-x1", 10, &key, true, None)
            .expect("encode paired beacon");

        assert!(matches!(
            PairedBeacon::decode_and_verify(&encoded, "thinkpad-x1", 10, Some(10), &key),
            Err(KeynearError::Crypto(_))
        ));

        assert!(matches!(
            PairedBeacon::decode_and_verify(&encoded, "thinkpad-x1", 30, None, &key),
            Err(KeynearError::Crypto(_))
        ));

        let mut tampered_token = encoded;
        tampered_token[0] ^= 0x01;
        assert!(matches!(
            PairedBeacon::decode_and_verify(&tampered_token, "thinkpad-x1", 10, None, &key),
            Err(KeynearError::Crypto(_))
        ));

        let mut tampered_ciphertext = encoded;
        tampered_ciphertext[10] ^= 0xFF;
        assert!(matches!(
            PairedBeacon::decode_and_verify(&tampered_ciphertext, "thinkpad-x1", 10, None, &key),
            Err(KeynearError::Crypto(_))
        ));

        let mut tampered_tag = encoded;
        tampered_tag[23] ^= 0x80;
        assert!(matches!(
            PairedBeacon::decode_and_verify(&tampered_tag, "thinkpad-x1", 10, None, &key),
            Err(KeynearError::Crypto(_))
        ));
    }

    #[test]
    fn lan_endpoint_codec_roundtrips_and_rejects_tampering() {
        let key = [0x77; 32];
        let payload = v1::LanEndpoint {
            device_id: "pixel-9".to_string(),
            endpoint_id: vec![0x33; 32],
            socket_addrs: vec!["192.168.1.50:49000".to_string()],
            capability_flags: 1,
            issued_at_ms: 1_763_000_000_123,
        };

        let encoded = LanEndpointCodec::encode(&payload, &key).expect("encode lan endpoint");
        let encoded_2 = LanEndpointCodec::encode(&payload, &key).expect("encode again");
        assert_ne!(
            encoded[1..13],
            encoded_2[1..13],
            "fresh random nonce must be used per encryption"
        );

        let decoded = LanEndpointCodec::decode(&encoded, "pixel-9", &key).expect("decode");
        assert_eq!(decoded, payload);

        let wrong_key = [0x88; 32];
        assert!(matches!(
            LanEndpointCodec::decode(&encoded, "pixel-9", &wrong_key),
            Err(KeynearError::Crypto(_))
        ));
        assert!(matches!(
            LanEndpointCodec::decode(&encoded, "other-device", &key),
            Err(KeynearError::Crypto(_))
        ));

        let mut tampered = encoded.clone();
        let last = tampered.len() - 1;
        tampered[last] ^= 0x01;
        assert!(matches!(
            LanEndpointCodec::decode(&tampered, "pixel-9", &key),
            Err(KeynearError::Crypto(_))
        ));

        let matched = LanEndpointCodec::decode_any(
            &encoded,
            [("other-device", &wrong_key), ("pixel-9", &key)],
        )
        .expect("decode_any matches pixel-9");
        assert_eq!(matched, payload);
    }
}
