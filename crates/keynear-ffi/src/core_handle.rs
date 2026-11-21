use crate::*;

#[derive(Debug, Default, uniffi::Object)]
pub struct CoreHandle {
    visibility: Mutex<VisibilityStateInner>,
}

#[uniffi::export]
impl CoreHandle {
    #[uniffi::constructor]
    pub fn new() -> Self {
        Self {
            visibility: Mutex::new(VisibilityStateInner::default()),
        }
    }

    pub fn status(&self) -> CoreStatus {
        get_core_status()
    }

    pub fn visibility_mode(&self) -> FfiVisibilityMode {
        visibility_mode_impl(&self.visibility)
    }

    pub fn set_visibility_mode(&self, mode: FfiVisibilityMode) -> FfiVisibilityMode {
        set_visibility_mode_impl(&self.visibility, mode)
    }

    pub fn cycle_visibility_mode(&self) -> FfiVisibilityMode {
        cycle_visibility_mode_impl(&self.visibility)
    }

    pub fn everyone_nearby_remaining_secs(&self) -> u64 {
        remaining_secs_impl(&self.visibility)
    }

    pub fn validate_config_toml(&self, raw_toml: String) -> Result<FfiVisibilityMode, FfiError> {
        validate_config_toml_impl(raw_toml)
    }

    pub fn load_daemon_config(
        &self,
        config_toml_path: String,
    ) -> Result<FfiDaemonConfig, FfiError> {
        load_daemon_config_impl(&self.visibility, config_toml_path)
    }

    pub fn save_daemon_config(
        &self,
        config_toml_path: String,
        config: FfiDaemonConfig,
    ) -> Result<FfiDaemonConfig, FfiError> {
        save_daemon_config_impl(&self.visibility, config_toml_path, config)
    }

    pub fn validate_control_frame(&self, bytes: Vec<u8>) -> Result<String, FfiError> {
        validate_control_frame_impl(bytes)
    }

    pub fn generate_static_keypair(&self) -> FfiStaticKeypair {
        let (private_key, public_key_hex) = generate_static_keypair_impl();
        FfiStaticKeypair {
            private_key,
            public_key_hex,
        }
    }

    pub fn encode_everyone_nearby_beacon(
        &self,
        role: FfiDeviceRole,
        capability_flags: u32,
        display_name: String,
    ) -> Result<Vec<u8>, FfiError> {
        encode_everyone_nearby_beacon_impl(role, capability_flags, display_name)
    }

    pub fn decode_everyone_nearby_beacon(
        &self,
        service_data: Vec<u8>,
    ) -> Result<FfiEveryoneNearbyBeacon, FfiError> {
        decode_everyone_nearby_beacon_impl(service_data)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn encode_paired_beacon(
        &self,
        device_id: String,
        counter: u32,
        reconnect_key: Vec<u8>,
        available: bool,
        battery_level: Option<u8>,
    ) -> Result<Vec<u8>, FfiError> {
        encode_paired_beacon_impl(device_id, counter, reconnect_key, available, battery_level)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn decode_paired_beacon(
        &self,
        service_data: Vec<u8>,
        device_id: String,
        expected_counter: u32,
        last_accepted_counter: Option<u32>,
        reconnect_key: Vec<u8>,
    ) -> Result<FfiPairedBeaconState, FfiError> {
        decode_paired_beacon_impl(
            service_data,
            device_id,
            expected_counter,
            last_accepted_counter,
            reconnect_key,
        )
    }

    pub fn derive_presence_token_bytes(
        &self,
        device_id: String,
        counter: u32,
        reconnect_key: Vec<u8>,
    ) -> Result<Vec<u8>, FfiError> {
        derive_presence_token_bytes_impl(device_id, counter, reconnect_key)
    }

    pub fn load_paired_devices(
        &self,
        devices_toml_path: String,
    ) -> Result<Vec<FfiPairedDevice>, FfiError> {
        load_paired_devices_impl(devices_toml_path)
    }

    pub fn save_paired_device(
        &self,
        devices_toml_path: String,
        device: FfiPairedDevice,
    ) -> Result<(), FfiError> {
        save_paired_device_impl(devices_toml_path, device)
    }

    pub fn remove_paired_device(
        &self,
        devices_toml_path: String,
        device_id: String,
    ) -> Result<bool, FfiError> {
        remove_paired_device_impl(devices_toml_path, device_id)
    }

    pub fn set_auto_accept_files(
        &self,
        devices_toml_path: String,
        device_id: String,
        enabled: bool,
    ) -> Result<FfiPairedDevice, FfiError> {
        set_auto_accept_files_impl(devices_toml_path, device_id, enabled)
    }

    pub fn rename_paired_device(
        &self,
        devices_toml_path: String,
        device_id: String,
        new_display_name: String,
    ) -> Result<FfiPairedDevice, FfiError> {
        rename_paired_device_impl(devices_toml_path, device_id, new_display_name)
    }

    pub fn encode_lan_endpoint(
        &self,
        endpoint: FfiLanEndpoint,
        reconnect_key: Vec<u8>,
    ) -> Result<Vec<u8>, FfiError> {
        encode_lan_endpoint_impl(endpoint, reconnect_key)
    }

    pub fn decode_lan_endpoint(
        &self,
        encoded: Vec<u8>,
        expected_device_id: String,
        reconnect_key: Vec<u8>,
    ) -> Result<FfiLanEndpoint, FfiError> {
        decode_lan_endpoint_impl(encoded, expected_device_id, reconnect_key)
    }
}
