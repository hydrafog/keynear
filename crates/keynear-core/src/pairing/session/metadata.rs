use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairingRole {
    Initiator,
    Responder,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairingState {
    HandshakeM1,
    HandshakeM2,
    HandshakeM3,
    AwaitingConfirmation,
    Approved,
    Expired,
    Rejected,
    Completed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairingPeerMetadata {
    pub device_id: String,
    pub display_name: String,
    pub device_role: DeviceRole,
    pub capability_flags: u32,
}

impl PairingPeerMetadata {
    pub fn validate(&self) -> Result<Self, KeynearError> {
        let device_id = sanitize_device_id(&self.device_id)?;
        let display_name = self.display_name.trim().to_string();
        if display_name.is_empty() || display_name.len() > MAX_DISPLAY_NAME_BYTES {
            return Err(KeynearError::Validation {
                field: "displayName",
                message: "display_name must be between 1 and 64 bytes".to_string(),
            });
        }
        Ok(Self {
            device_id,
            display_name,
            device_role: self.device_role,
            capability_flags: self.capability_flags,
        })
    }

    pub fn encode(&self) -> Result<Vec<u8>, KeynearError> {
        let valid = self.validate()?;
        let id_bytes = valid.device_id.as_bytes();
        let name_bytes = valid.display_name.as_bytes();
        let role_byte = match valid.device_role {
            DeviceRole::Desktop => 0x01,
            DeviceRole::Mobile => 0x02,
        };
        let mut out = Vec::with_capacity(8 + id_bytes.len() + name_bytes.len());
        out.push(METADATA_FORMAT_VERSION);
        out.push(role_byte);
        out.extend_from_slice(&valid.capability_flags.to_le_bytes());
        out.push(id_bytes.len() as u8);
        out.extend_from_slice(id_bytes);
        out.push(name_bytes.len() as u8);
        out.extend_from_slice(name_bytes);
        Ok(out)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, KeynearError> {
        if bytes.len() < 8 {
            return Err(KeynearError::Protocol(
                "pairing metadata payload too short".to_string(),
            ));
        }
        if bytes[0] != METADATA_FORMAT_VERSION {
            return Err(KeynearError::Protocol(format!(
                "unsupported pairing metadata version {}",
                bytes[0]
            )));
        }
        let device_role = decode_device_role(bytes[1])?;
        let mut cap_bytes = [0u8; 4];
        cap_bytes.copy_from_slice(&bytes[2..6]);
        let capability_flags = u32::from_le_bytes(cap_bytes);

        let id_len = bytes[6] as usize;
        let id_end = 7 + id_len;
        if id_len == 0 || id_end >= bytes.len() {
            return Err(KeynearError::Protocol(
                "invalid device_id length in pairing metadata".to_string(),
            ));
        }
        let device_id = std::str::from_utf8(&bytes[7..id_end])
            .map_err(|_| KeynearError::Protocol("invalid utf-8 device_id".to_string()))?
            .to_string();

        let name_len = bytes[id_end] as usize;
        let name_start = id_end + 1;
        let name_end = name_start + name_len;
        if name_len == 0 || name_end != bytes.len() {
            return Err(KeynearError::Protocol(
                "invalid display_name length in pairing metadata".to_string(),
            ));
        }
        let display_name = std::str::from_utf8(&bytes[name_start..name_end])
            .map_err(|_| KeynearError::Protocol("invalid utf-8 display_name".to_string()))?
            .to_string();

        Self {
            device_id,
            display_name,
            device_role,
            capability_flags,
        }
        .validate()
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
