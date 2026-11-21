use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum VisibilityMode {
    Hidden,
    #[default]
    PairedOnly,
    EveryoneNearby,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DeviceRole {
    Desktop,
    Mobile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TransferDirection {
    Send,
    Receive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TransferState {
    PendingConsent,
    Active,
    Completed,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PairedDevice {
    pub device_id: String,
    pub display_name: String,
    pub device_role: DeviceRole,
    pub static_x25519_pubkey: String,
    pub last_ble_address: String,
    pub last_lan_address: String,
    #[serde(default)]
    pub last_endpoint_id: String,
    pub capability_flags: u32,
    pub auto_accept_files: bool,
    pub paired_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NearbyPeer {
    pub presence_token: String,
    pub display_name: String,
    pub device_role: DeviceRole,
    pub rssi: i16,
    pub battery_level: Option<u8>,
    pub is_paired: bool,
    pub last_seen_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferItem {
    pub name: String,
    pub mime_type: String,
    pub size_bytes: u64,
    pub is_directory: bool,
    pub target_path: Option<String>,
    pub checksum: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferSession {
    pub transfer_id: String,
    pub peer_device_id: String,
    pub direction: TransferDirection,
    pub items: Vec<TransferItem>,
    pub total_bytes: u64,
    pub transferred_bytes: u64,
    pub state: TransferState,
    pub started_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DaemonConfig {
    pub visibility_mode: VisibilityMode,
    pub incoming_directory: String,
    pub auto_accept_from_paired: bool,
    pub clipboard_sync_enabled: bool,
    pub clipboard_images_enabled: bool,
    pub otp_passthrough_enabled: bool,
    pub audio_handoff_enabled: bool,
    pub ignored_mime_types: Vec<String>,
    pub notify_auto_accept_complete: bool,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            visibility_mode: VisibilityMode::PairedOnly,
            incoming_directory: "~/Downloads/Keynear".to_string(),
            auto_accept_from_paired: false,
            clipboard_sync_enabled: true,
            clipboard_images_enabled: true,
            otp_passthrough_enabled: true,
            audio_handoff_enabled: true,
            ignored_mime_types: vec![
                "x-kde-passwordManagerHint".to_string(),
                "application/x-password".to_string(),
            ],
            notify_auto_accept_complete: true,
        }
    }
}

impl DaemonConfig {
    pub fn validate(&self) -> Result<(), KeynearError> {
        if self.incoming_directory.trim().is_empty() {
            return Err(KeynearError::Validation {
                field: "incomingDirectory",
                message: "incoming directory must not be empty".to_string(),
            });
        }
        Ok(())
    }

    pub fn load_or_default(path: &Path) -> Result<Self, KeynearError> {
        match fs::read_to_string(path) {
            Ok(raw) => {
                let parsed: Self =
                    toml::from_str(&raw).map_err(|err| KeynearError::Config(err.to_string()))?;
                parsed.validate()?;
                Ok(parsed)
            }
            Err(err) if err.kind() == ErrorKind::NotFound => {
                let default = Self::default();
                default.validate()?;
                Ok(default)
            }
            Err(err) => Err(KeynearError::Io(err)),
        }
    }

    pub fn save_to_path(&self, path: &Path) -> Result<(), KeynearError> {
        self.validate()?;
        let encoded =
            toml::to_string_pretty(self).map_err(|err| KeynearError::Config(err.to_string()))?;
        super::io::write_toml_atomic(path, &encoded, "config.toml")
    }
}
