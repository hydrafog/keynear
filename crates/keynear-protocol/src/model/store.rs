use super::*;

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
            normalized.display_name = validate_display_name(&normalized.display_name)?;
            deduped.insert(normalized.device_id.clone(), normalized);
        }

        let payload = PairedDevicesFile {
            devices: deduped.into_values().collect(),
        };
        let encoded = toml::to_string_pretty(&payload)
            .map_err(|err| KeynearError::Config(err.to_string()))?;
        super::io::write_toml_atomic(&self.path, &encoded, "devices.toml")
    }

    pub fn upsert(&self, mut device: PairedDevice) -> Result<Vec<PairedDevice>, KeynearError> {
        device.device_id = sanitize_device_id(&device.device_id)?;
        device.display_name = validate_display_name(&device.display_name)?;
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

    pub fn set_auto_accept_files(
        &self,
        device_id: &str,
        enabled: bool,
    ) -> Result<PairedDevice, KeynearError> {
        let clean_id = sanitize_device_id(device_id)?;
        let mut devices = self.load()?;
        let updated = {
            let device = devices
                .iter_mut()
                .find(|d| d.device_id == clean_id)
                .ok_or_else(|| KeynearError::Validation {
                    field: "deviceId",
                    message: format!("paired device '{clean_id}' not found"),
                })?;
            device.auto_accept_files = enabled;
            device.clone()
        };
        self.save(&devices)?;
        Ok(updated)
    }

    pub fn update_lan_endpoint(
        &self,
        device_id: &str,
        endpoint_id: &str,
        lan_address: &str,
    ) -> Result<PairedDevice, KeynearError> {
        let clean_id = sanitize_device_id(device_id)?;
        let mut devices = self.load()?;
        let updated = {
            let device = devices
                .iter_mut()
                .find(|d| d.device_id == clean_id)
                .ok_or_else(|| KeynearError::Validation {
                    field: "deviceId",
                    message: format!("paired device '{clean_id}' not found"),
                })?;
            if !endpoint_id.trim().is_empty() {
                device.last_endpoint_id = endpoint_id.trim().to_string();
            }
            if !lan_address.trim().is_empty() {
                device.last_lan_address = lan_address.trim().to_string();
            }
            device.clone()
        };
        self.save(&devices)?;
        Ok(updated)
    }

    pub fn rename(
        &self,
        device_id: &str,
        new_display_name: &str,
    ) -> Result<PairedDevice, KeynearError> {
        let clean_id = sanitize_device_id(device_id)?;
        let normalized_name = validate_display_name(new_display_name)?;
        let mut devices = self.load()?;
        let updated = {
            let device = devices
                .iter_mut()
                .find(|d| d.device_id == clean_id)
                .ok_or_else(|| KeynearError::Validation {
                    field: "deviceId",
                    message: format!("paired device '{clean_id}' not found"),
                })?;
            device.display_name = normalized_name;
            device.clone()
        };
        self.save(&devices)?;
        Ok(updated)
    }
}
