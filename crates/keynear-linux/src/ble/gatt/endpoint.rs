use keynear_core::{LanEndpointCodec, PairedDeviceStore, SecretStore, MAX_CONTROL_FRAME_BYTES};
use keynear_protocol::{v1, KeynearError};

fn match_peer<'a>(
    devices: &'a [keynear_protocol::PairedDevice],
    known_device_id: Option<&str>,
    peer_ble_address: Option<&str>,
) -> Option<&'a keynear_protocol::PairedDevice> {
    known_device_id
        .and_then(|id| devices.iter().find(|d| d.device_id == id))
        .or_else(|| {
            peer_ble_address
                .map(str::trim)
                .filter(|addr| !addr.is_empty())
                .and_then(|addr| {
                    devices.iter().find(|d| {
                        !d.last_ble_address.trim().is_empty()
                            && d.last_ble_address.eq_ignore_ascii_case(addr)
                    })
                })
        })
        .or_else(|| {
            if devices.len() == 1 {
                devices.first()
            } else {
                None
            }
        })
}

pub struct EncodeEndpointParams<'a> {
    pub endpoint: &'a v1::LanEndpoint,
    pub known_device_id: Option<&'a str>,
    pub peer_ble_address: Option<&'a str>,
    pub device_store: &'a PairedDeviceStore,
    pub secret_store: &'a dyn SecretStore,
}

pub fn encode_lan_endpoint_for_peer(
    params: EncodeEndpointParams<'_>,
) -> Result<Vec<u8>, KeynearError> {
    let EncodeEndpointParams {
        endpoint,
        known_device_id,
        peer_ble_address,
        device_store,
        secret_store,
    } = params;
    let devices = device_store.load()?;
    if devices.is_empty() {
        return Err(KeynearError::UnauthorizedPeer(
            "no paired devices registered for LAN endpoint exchange".to_string(),
        ));
    }

    let matched = match_peer(&devices, known_device_id, peer_ble_address).ok_or_else(|| {
        KeynearError::UnauthorizedPeer(
            "unrecognized BLE peer address for LAN endpoint read".to_string(),
        )
    })?;

    let secrets = secret_store
        .get_peer_secrets(&matched.device_id)?
        .ok_or_else(|| {
            KeynearError::UnauthorizedPeer(format!(
                "missing pairwise reconnect secret for '{}'",
                matched.device_id
            ))
        })?;

    LanEndpointCodec::encode(endpoint, &secrets.reconnect_secret)
}

pub fn decode_lan_endpoint_from_known_peers(
    encoded: &[u8],
    device_store: &PairedDeviceStore,
    secret_store: &dyn SecretStore,
) -> Result<v1::LanEndpoint, KeynearError> {
    if encoded.is_empty() || encoded.len() > MAX_CONTROL_FRAME_BYTES {
        return Err(KeynearError::Validation {
            field: "lanEndpoint",
            message: format!(
                "encrypted LAN endpoint length {} must be within 1..={MAX_CONTROL_FRAME_BYTES}",
                encoded.len()
            ),
        });
    }
    let devices = device_store.load()?;
    let mut keys: Vec<(String, [u8; 32])> = Vec::new();
    for dev in devices {
        if let Some(secrets) = secret_store.get_peer_secrets(&dev.device_id)? {
            keys.push((dev.device_id, secrets.reconnect_secret));
        }
    }
    LanEndpointCodec::decode_any(
        encoded,
        keys.iter().map(|(id, secret)| (id.as_str(), secret)),
    )
}
