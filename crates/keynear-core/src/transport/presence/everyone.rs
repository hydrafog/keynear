use super::*;

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
        let role = decode_device_role(service_data[1])?;
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

fn decode_device_role(byte: u8) -> Result<DeviceRole, KeynearError> {
    match byte {
        0x01 => Ok(DeviceRole::Desktop),
        0x02 => Ok(DeviceRole::Mobile),
        other => Err(KeynearError::Protocol(format!(
            "unknown device role byte {other}"
        ))),
    }
}
