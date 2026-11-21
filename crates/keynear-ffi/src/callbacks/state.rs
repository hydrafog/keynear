use crate::callbacks::*;

pub(crate) fn to_key_32(bytes: &[u8], field: &'static str) -> Result<[u8; 32], KeynearError> {
    if bytes.len() != 32 {
        return Err(KeynearError::Validation {
            field,
            message: format!("expected 32 bytes, got {}", bytes.len()),
        });
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(bytes);
    Ok(out)
}

#[derive(Debug, Default)]
pub(crate) struct VisibilityStateInner {
    pub(crate) mode: VisibilityMode,
    deadline: Option<Instant>,
}

impl VisibilityStateInner {
    pub(crate) fn refresh(&mut self, now: Instant) {
        if self.mode == VisibilityMode::EveryoneNearby {
            if let Some(deadline) = self.deadline {
                if now >= deadline {
                    self.mode = VisibilityMode::PairedOnly;
                    self.deadline = None;
                }
            }
        }
    }

    pub(crate) fn set_mode(&mut self, mode: VisibilityMode, now: Instant) -> VisibilityMode {
        self.mode = mode;
        self.deadline = if mode == VisibilityMode::EveryoneNearby {
            Some(now + Duration::from_secs(EVERYONE_NEARBY_TIMEOUT_SECS))
        } else {
            None
        };
        mode
    }

    pub(crate) fn remaining_secs(&mut self, now: Instant) -> u64 {
        self.refresh(now);
        if self.mode != VisibilityMode::EveryoneNearby {
            return 0;
        }
        self.deadline
            .and_then(|deadline| deadline.checked_duration_since(now))
            .map(|duration| duration.as_secs())
            .unwrap_or(0)
    }
}

pub(crate) fn visibility_mode_impl(lock: &Mutex<VisibilityStateInner>) -> FfiVisibilityMode {
    let mut guard = lock.lock().unwrap_or_else(|p| p.into_inner());
    guard.refresh(Instant::now());
    FfiVisibilityMode::from(guard.mode)
}

pub(crate) fn set_visibility_mode_impl(
    lock: &Mutex<VisibilityStateInner>,
    mode: FfiVisibilityMode,
) -> FfiVisibilityMode {
    let mut guard = lock.lock().unwrap_or_else(|p| p.into_inner());
    let updated = guard.set_mode(VisibilityMode::from(mode), Instant::now());
    FfiVisibilityMode::from(updated)
}

pub(crate) fn cycle_visibility_mode_impl(lock: &Mutex<VisibilityStateInner>) -> FfiVisibilityMode {
    let mut guard = lock.lock().unwrap_or_else(|p| p.into_inner());
    let now = Instant::now();
    guard.refresh(now);
    let next = match guard.mode {
        VisibilityMode::Hidden => VisibilityMode::PairedOnly,
        VisibilityMode::PairedOnly => VisibilityMode::EveryoneNearby,
        VisibilityMode::EveryoneNearby => VisibilityMode::Hidden,
    };
    guard.set_mode(next, now);
    FfiVisibilityMode::from(next)
}

pub(crate) fn remaining_secs_impl(lock: &Mutex<VisibilityStateInner>) -> u64 {
    let mut guard = lock.lock().unwrap_or_else(|p| p.into_inner());
    guard.remaining_secs(Instant::now())
}

pub(crate) fn validate_config_toml_impl(raw_toml: String) -> Result<FfiVisibilityMode, FfiError> {
    let parsed: DaemonConfig =
        toml::from_str(&raw_toml).map_err(|err| KeynearError::Config(err.to_string()))?;
    parsed.validate()?;
    Ok(FfiVisibilityMode::from(parsed.visibility_mode))
}

pub(crate) fn load_daemon_config_impl(
    lock: &Mutex<VisibilityStateInner>,
    config_toml_path: String,
) -> Result<FfiDaemonConfig, FfiError> {
    let cfg = DaemonConfig::load_or_default(&PathBuf::from(config_toml_path))?;
    let ffi_cfg = FfiDaemonConfig::from(cfg);
    set_visibility_mode_impl(lock, ffi_cfg.visibility_mode);
    Ok(ffi_cfg)
}

pub(crate) fn save_daemon_config_impl(
    lock: &Mutex<VisibilityStateInner>,
    config_toml_path: String,
    config: FfiDaemonConfig,
) -> Result<FfiDaemonConfig, FfiError> {
    let daemon_cfg = DaemonConfig::from(config);
    daemon_cfg.save_to_path(&PathBuf::from(config_toml_path))?;
    let saved = FfiDaemonConfig::from(daemon_cfg);
    set_visibility_mode_impl(lock, saved.visibility_mode);
    Ok(saved)
}

pub(crate) fn validate_control_frame_impl(bytes: Vec<u8>) -> Result<String, FfiError> {
    let frame = decode_control_frame(&bytes)?;
    control_frame_to_envelope(&frame)?;
    Ok(frame.kind)
}

pub(crate) fn generate_static_keypair_impl() -> (Vec<u8>, String) {
    let kp = StaticKeypairRecord::generate();
    (kp.private_key.to_vec(), hex::encode(kp.public_key))
}

pub(crate) fn load_paired_devices_impl(
    devices_toml_path: String,
) -> Result<Vec<FfiPairedDevice>, FfiError> {
    let store = PairedDeviceStore::new(PathBuf::from(devices_toml_path));
    let devices = store.load()?;
    Ok(devices.into_iter().map(FfiPairedDevice::from).collect())
}

pub(crate) fn save_paired_device_impl(
    devices_toml_path: String,
    device: FfiPairedDevice,
) -> Result<(), FfiError> {
    let store = PairedDeviceStore::new(PathBuf::from(devices_toml_path));
    store.upsert(PairedDevice::from(device))?;
    Ok(())
}

pub(crate) fn remove_paired_device_impl(
    devices_toml_path: String,
    device_id: String,
) -> Result<bool, FfiError> {
    let store = PairedDeviceStore::new(PathBuf::from(devices_toml_path));
    store.remove(&device_id).map_err(FfiError::from)
}

pub(crate) fn set_auto_accept_files_impl(
    devices_toml_path: String,
    device_id: String,
    enabled: bool,
) -> Result<FfiPairedDevice, FfiError> {
    let store = PairedDeviceStore::new(PathBuf::from(devices_toml_path));
    let updated = store.set_auto_accept_files(&device_id, enabled)?;
    Ok(FfiPairedDevice::from(updated))
}

pub(crate) fn rename_paired_device_impl(
    devices_toml_path: String,
    device_id: String,
    new_display_name: String,
) -> Result<FfiPairedDevice, FfiError> {
    let store = PairedDeviceStore::new(PathBuf::from(devices_toml_path));
    let updated = store.rename(&device_id, &new_display_name)?;
    Ok(FfiPairedDevice::from(updated))
}
