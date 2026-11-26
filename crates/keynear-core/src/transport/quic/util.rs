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

#[cfg(test)]
mod tests {
    use super::*;

    const PIXEL_PHONE_EID_HEX: &str =
        "2df04125f0015afb47ce853aef8772094ff9498c14cb1b9e12973c2927da0fa6";

    #[test]
    fn parse_endpoint_id_hex_accepts_valid_compressed_ed25519_key() {
        let parsed = parse_endpoint_id_hex(PIXEL_PHONE_EID_HEX).expect("valid endpoint id");
        assert_eq!(hex::encode(parsed.as_bytes()), PIXEL_PHONE_EID_HEX);
    }

    #[test]
    fn parse_endpoint_id_hex_rejects_malformed_and_wrong_length_input() {
        assert!(parse_endpoint_id_hex("abc").is_none());
        assert!(parse_endpoint_id_hex("zzzz").is_none());
        assert!(parse_endpoint_id_hex("aabb").is_none());
    }

    #[test]
    fn parse_endpoint_id_hex_round_trips_any_32_byte_value() {
        let raw = "ff".repeat(32);
        let parsed = parse_endpoint_id_hex(&raw).expect("32 bytes parse");
        assert_eq!(hex::encode(parsed.as_bytes()), raw);
    }

    #[test]
    fn parse_endpoint_id_bytes_accepts_valid_32_bytes() {
        let bytes: [u8; 32] = hex::decode(PIXEL_PHONE_EID_HEX)
            .expect("valid hex")
            .try_into()
            .expect("32 bytes");
        let parsed = parse_endpoint_id_bytes(&bytes).expect("valid endpoint id");
        assert_eq!(parsed.as_bytes(), &bytes);
    }

    #[test]
    fn parse_endpoint_id_bytes_rejects_wrong_length() {
        for len in [31, 33] {
            let err = parse_endpoint_id_bytes(&vec![0u8; len]).expect_err("wrong length");
            assert!(matches!(
                err,
                KeynearError::Validation {
                    field: "endpointId",
                    ..
                }
            ));
        }
    }

    #[test]
    fn parse_endpoint_id_bytes_round_trips_any_32_byte_value() {
        let bytes = [0xff; 32];
        let parsed = parse_endpoint_id_bytes(&bytes).expect("32 bytes parse");
        assert_eq!(parsed.as_bytes(), &bytes);
    }
}
