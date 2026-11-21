use super::*;

pub fn parse_endpoint_id_hex(raw: &str) -> Option<EndpointId> {
    let bytes = hex::decode(raw.trim()).ok()?;
    let arr: [u8; 32] = bytes.try_into().ok()?;
    EndpointId::from_bytes(&arr).ok()
}

pub fn parse_endpoint_id_bytes(bytes: &[u8]) -> Result<EndpointId, KeynearError> {
    let arr: [u8; 32] = bytes.try_into().map_err(|_| KeynearError::Validation {
        field: "endpointId",
        message: "endpoint_id must be 32 bytes".to_string(),
    })?;
    EndpointId::from_bytes(&arr).map_err(|err| KeynearError::Validation {
        field: "endpointId",
        message: format!("invalid Ed25519 endpoint_id: {err}"),
    })
}

pub(super) fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
