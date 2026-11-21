use super::*;

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

impl PairedBeacon {
    #[allow(clippy::too_many_arguments)]
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

    #[allow(clippy::too_many_arguments)]
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

        validate_presence_counter(counter, expected_counter, last_accepted_counter)?;

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

        Ok(parse_paired_plaintext(counter, &plaintext))
    }
}

fn validate_presence_counter(
    counter: u32,
    expected_counter: u32,
    last_accepted_counter: Option<u32>,
) -> Result<(), KeynearError> {
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
    Ok(())
}

fn parse_paired_plaintext(counter: u32, plaintext: &[u8; 2]) -> PairedBeaconState {
    let available = (plaintext[0] & AVAILABILITY_ONLINE) != 0;
    let battery_level = if plaintext[1] == BATTERY_UNKNOWN {
        None
    } else {
        Some(plaintext[1].min(MAX_BATTERY_PERCENT))
    };
    PairedBeaconState {
        counter,
        available,
        battery_level,
    }
}
