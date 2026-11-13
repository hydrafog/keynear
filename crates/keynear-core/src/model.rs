use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

use crate::error::KeynearError;

pub const EVERYONE_NEARBY_TIMEOUT_SECS: u64 = 600;
pub const PAIRING_TIMEOUT_SECS: u64 = 120;
const MAX_DEVICE_ID_BYTES: usize = 64;
static TEMP_WRITE_SEQ: AtomicU64 = AtomicU64::new(0);

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
}

pub fn sanitize_device_id(raw: &str) -> Result<String, KeynearError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.len() > MAX_DEVICE_ID_BYTES {
        return Err(KeynearError::Validation {
            field: "deviceId",
            message: "device_id must be between 1 and 64 bytes".to_string(),
        });
    }
    if trimmed.contains("..")
        || !trimmed
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
    {
        return Err(KeynearError::Validation {
            field: "deviceId",
            message: "device_id contains invalid characters".to_string(),
        });
    }
    Ok(trimmed.to_string())
}

pub fn default_config_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("KEYNEAR_CONFIG_DIR") {
        let trimmed = dir.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        let trimmed = xdg.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed).join("keynear");
        }
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".config").join("keynear")
}

pub fn default_config_path() -> PathBuf {
    default_config_dir().join("config.toml")
}

pub fn default_devices_path() -> PathBuf {
    default_config_dir().join("devices.toml")
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct PairedDevicesFile {
    devices: Vec<PairedDevice>,
}

#[derive(Debug, Clone)]
pub struct PairedDeviceStore {
    path: PathBuf,
}

impl PairedDeviceStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn default_location() -> Self {
        Self::new(default_devices_path())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load(&self) -> Result<Vec<PairedDevice>, KeynearError> {
        match fs::read_to_string(&self.path) {
            Ok(raw) => {
                let parsed: PairedDevicesFile =
                    toml::from_str(&raw).map_err(|err| KeynearError::Config(err.to_string()))?;
                let mut deduped = BTreeMap::new();
                for mut device in parsed.devices {
                    device.device_id = sanitize_device_id(&device.device_id)?;
                    if device.display_name.trim().is_empty() {
                        return Err(KeynearError::Validation {
                            field: "displayName",
                            message: "display_name must not be empty".to_string(),
                        });
                    }
                    deduped.insert(device.device_id.clone(), device);
                }
                Ok(deduped.into_values().collect())
            }
            Err(err) if err.kind() == ErrorKind::NotFound => Ok(Vec::new()),
            Err(err) => Err(KeynearError::Io(err)),
        }
    }

    pub fn save(&self, devices: &[PairedDevice]) -> Result<(), KeynearError> {
        let mut deduped = BTreeMap::new();
        for device in devices {
            let mut normalized = device.clone();
            normalized.device_id = sanitize_device_id(&normalized.device_id)?;
            if normalized.display_name.trim().is_empty() {
                return Err(KeynearError::Validation {
                    field: "displayName",
                    message: "display_name must not be empty".to_string(),
                });
            }
            deduped.insert(normalized.device_id.clone(), normalized);
        }

        let payload = PairedDevicesFile {
            devices: deduped.into_values().collect(),
        };
        let encoded = toml::to_string_pretty(&payload)
            .map_err(|err| KeynearError::Config(err.to_string()))?;

        if let Some(parent) = self.path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let _ = fs::set_permissions(parent, fs::Permissions::from_mode(0o700));
                }
            }
        }

        let seq = TEMP_WRITE_SEQ.fetch_add(1, Ordering::Relaxed);
        let file_name = self
            .path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("devices.toml");
        let tmp_path = self
            .path
            .with_file_name(format!(".{file_name}.tmp.{}.{seq}", std::process::id()));

        let write_result = (|| -> Result<(), KeynearError> {
            let mut opts = OpenOptions::new();
            opts.create_new(true).write(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                opts.mode(0o600);
            }
            let mut file = opts.open(&tmp_path)?;
            file.write_all(encoded.as_bytes())?;
            file.sync_all()?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&tmp_path, fs::Permissions::from_mode(0o600))?;
            }
            fs::rename(&tmp_path, &self.path)?;
            Ok(())
        })();

        if write_result.is_err() {
            let _ = fs::remove_file(&tmp_path);
        }
        write_result
    }

    pub fn upsert(&self, mut device: PairedDevice) -> Result<Vec<PairedDevice>, KeynearError> {
        device.device_id = sanitize_device_id(&device.device_id)?;
        if device.display_name.trim().is_empty() {
            return Err(KeynearError::Validation {
                field: "displayName",
                message: "display_name must not be empty".to_string(),
            });
        }
        let mut devices = self.load()?;
        if let Some(existing) = devices.iter_mut().find(|d| d.device_id == device.device_id) {
            *existing = device;
        } else {
            devices.push(device);
        }
        devices.sort_by(|a, b| a.device_id.cmp(&b.device_id));
        self.save(&devices)?;
        Ok(devices)
    }

    pub fn remove(&self, device_id: &str) -> Result<bool, KeynearError> {
        let clean_id = sanitize_device_id(device_id)?;
        let mut devices = self.load()?;
        let original_len = devices.len();
        devices.retain(|d| d.device_id != clean_id);
        if devices.len() == original_len {
            return Ok(false);
        }
        self.save(&devices)?;
        Ok(true)
    }

    pub fn get(&self, device_id: &str) -> Result<Option<PairedDevice>, KeynearError> {
        let clean_id = sanitize_device_id(device_id)?;
        let devices = self.load()?;
        Ok(devices.into_iter().find(|d| d.device_id == clean_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_path(name: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock is after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("keynear-{name}-{nanos}.toml"))
    }

    fn sample_device(id: &str, name: &str) -> PairedDevice {
        PairedDevice {
            device_id: id.to_string(),
            display_name: name.to_string(),
            device_role: DeviceRole::Desktop,
            static_x25519_pubkey: "pubkey-hex".to_string(),
            last_ble_address: "AA:BB:CC:DD:EE:FF".to_string(),
            last_lan_address: "192.168.1.10:5000".to_string(),
            capability_flags: 1,
            auto_accept_files: true,
            paired_at: "2026-11-13T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn config_missing_returns_default() {
        let missing = unique_temp_path("missing");
        let loaded = DaemonConfig::load_or_default(&missing).expect("missing file uses default");
        assert_eq!(loaded, DaemonConfig::default());
    }

    #[test]
    fn config_malformed_returns_error() {
        let path = unique_temp_path("malformed");
        fs::write(&path, "visibilityMode = [not valid toml").expect("write temp file");
        let err = DaemonConfig::load_or_default(&path).expect_err("malformed toml must fail");
        let _ = fs::remove_file(&path);
        assert!(matches!(err, KeynearError::Config(_)));
    }

    #[test]
    fn config_empty_incoming_directory_fails_validation() {
        let path = unique_temp_path("empty-dir");
        fs::write(&path, "incomingDirectory = \"   \"\n").expect("write temp file");
        let err = DaemonConfig::load_or_default(&path).expect_err("empty dir must fail");
        let _ = fs::remove_file(&path);
        assert!(matches!(
            err,
            KeynearError::Validation {
                field: "incomingDirectory",
                ..
            }
        ));
    }

    #[test]
    fn secret_fields_absent_from_serialized_paired_device() {
        let device = sample_device("dev-1", "Workstation");
        let encoded = toml::to_string(&device).expect("serialize paired device");
        assert!(!encoded.contains("irk"));
        assert!(!encoded.contains("reconnectSecret"));
        assert!(!encoded.contains("reconnect_secret"));
        assert!(!encoded.contains("private_key"));
    }

    #[test]
    fn paired_device_store_roundtrip_and_malformed_rejection() {
        let path = unique_temp_path("devices");
        let store = PairedDeviceStore::new(&path);

        assert!(store
            .load()
            .expect("missing devices file returns empty")
            .is_empty());

        let updated = store
            .upsert(sample_device("dev-b", "Pixel"))
            .expect("upsert first device");
        assert_eq!(updated.len(), 1);

        let updated = store
            .upsert(sample_device("dev-a", "ThinkPad"))
            .expect("upsert second device");
        assert_eq!(updated.len(), 2);
        assert_eq!(updated[0].device_id, "dev-a");
        assert_eq!(updated[1].device_id, "dev-b");

        let raw = fs::read_to_string(&path).expect("read saved devices.toml");
        assert!(!raw.contains("irk"));
        assert!(!raw.contains("reconnectSecret"));
        assert!(!raw.contains("private_key"));

        assert!(store.remove("dev-a").expect("remove existing device"));
        assert!(!store
            .remove("dev-a")
            .expect("remove missing device is false"));
        assert_eq!(store.load().expect("reload after remove").len(), 1);

        fs::write(&path, "devices = [invalid").expect("corrupt devices.toml");
        assert!(matches!(store.load(), Err(KeynearError::Config(_))));
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn sanitize_device_id_rejects_traversal_and_empty() {
        assert!(sanitize_device_id("").is_err());
        assert!(sanitize_device_id("   ").is_err());
        assert!(sanitize_device_id("../etc/passwd").is_err());
        assert!(sanitize_device_id("a/b").is_err());
        assert!(sanitize_device_id("a..b").is_err());
        assert_eq!(
            sanitize_device_id(" pixel-9_pro.1 ").unwrap(),
            "pixel-9_pro.1"
        );
    }
}
