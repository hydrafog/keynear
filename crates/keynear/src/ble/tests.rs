mod advertisement;
mod endpoint;
mod pairing;

use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use keynear_core::PairedDeviceStore;

use super::*;

fn temp_devices_store(label: &str) -> (std::path::PathBuf, PairedDeviceStore) {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("keynear-{label}-{nonce}"));
    fs::create_dir_all(&dir).expect("tempdir");
    let store = PairedDeviceStore::new(dir.join("devices.toml"));
    (dir, store)
}
