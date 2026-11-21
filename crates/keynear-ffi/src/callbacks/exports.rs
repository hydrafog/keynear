use crate::callbacks::*;

#[uniffi::export]
pub fn get_core_status() -> CoreStatus {
    CoreStatus {
        version: env!("CARGO_PKG_VERSION").to_string(),
        max_control_frame_bytes: MAX_CONTROL_FRAME_BYTES as u32,
        ready: true,
    }
}

#[uniffi::export]
pub fn default_config_summary() -> String {
    let config = DaemonConfig::default();
    format!(
        "visibility={:?},incoming_dir={}",
        config.visibility_mode, config.incoming_directory
    )
}

#[uniffi::export]
pub fn ble_service_uuid_string() -> String {
    KEYNEAR_BLE_SERVICE_UUID.to_string()
}

#[uniffi::export]
pub fn ble_service_uuid16() -> u16 {
    KEYNEAR_BLE_SERVICE_UUID16
}

#[uniffi::export]
pub fn lan_endpoint_char_uuid_string() -> String {
    LAN_ENDPOINT_CHAR_UUID.to_string()
}

#[uniffi::export]
pub fn lan_endpoint_char_uuid16() -> u16 {
    LAN_ENDPOINT_CHAR_UUID16
}

#[uniffi::export]
pub fn everyone_nearby_timeout_secs() -> u64 {
    EVERYONE_NEARBY_TIMEOUT_SECS
}

#[uniffi::export]
pub fn pairing_timeout_secs() -> u64 {
    PAIRING_TIMEOUT_SECS
}
