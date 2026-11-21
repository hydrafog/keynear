use crate::*;

pub(crate) fn encode_visibility(mode: VisibilityMode) -> u8 {
    match mode {
        VisibilityMode::Hidden => 1,
        VisibilityMode::PairedOnly => 2,
        VisibilityMode::EveryoneNearby => 3,
    }
}

pub(crate) fn decode_visibility(raw: u8) -> Result<VisibilityMode, KeynearError> {
    match raw {
        1 => Ok(VisibilityMode::Hidden),
        2 => Ok(VisibilityMode::PairedOnly),
        3 => Ok(VisibilityMode::EveryoneNearby),
        other => Err(KeynearError::Protocol(format!(
            "invalid IPC visibility mode {other}"
        ))),
    }
}

pub(crate) fn encode_device_role(role: DeviceRole) -> u8 {
    match role {
        DeviceRole::Desktop => 1,
        DeviceRole::Mobile => 2,
    }
}

pub(crate) fn decode_device_role(raw: u8) -> Result<DeviceRole, KeynearError> {
    match raw {
        1 => Ok(DeviceRole::Desktop),
        2 => Ok(DeviceRole::Mobile),
        other => Err(KeynearError::Protocol(format!(
            "invalid IPC device role {other}"
        ))),
    }
}
