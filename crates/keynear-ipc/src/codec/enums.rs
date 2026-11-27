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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_visibility_raw_returns_protocol_error() {
        for raw in [0, 4, 255] {
            assert!(matches!(
                decode_visibility(raw),
                Err(KeynearError::Protocol(_))
            ));
        }
    }

    #[test]
    fn valid_visibility_values_round_trip() {
        for mode in [
            VisibilityMode::Hidden,
            VisibilityMode::PairedOnly,
            VisibilityMode::EveryoneNearby,
        ] {
            assert_eq!(
                decode_visibility(encode_visibility(mode)).expect("decode"),
                mode
            );
        }
    }

    #[test]
    fn invalid_device_role_raw_returns_protocol_error() {
        for raw in [0, 3, 255] {
            assert!(matches!(
                decode_device_role(raw),
                Err(KeynearError::Protocol(_))
            ));
        }
    }

    #[test]
    fn valid_device_role_values_round_trip() {
        for role in [DeviceRole::Desktop, DeviceRole::Mobile] {
            assert_eq!(
                decode_device_role(encode_device_role(role)).expect("decode"),
                role
            );
        }
    }
}
