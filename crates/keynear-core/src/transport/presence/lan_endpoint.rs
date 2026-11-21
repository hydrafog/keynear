use super::*;

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
