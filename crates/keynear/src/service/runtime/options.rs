use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use keynear_protocol::EVERYONE_NEARBY_TIMEOUT_SECS;

#[derive(Debug, Clone)]
pub struct RuntimeOptions {
    pub local_device_id: String,
    pub display_name: String,
    pub everyone_nearby_timeout: Duration,
    pub enable_bluez: bool,
    pub enable_mdns: bool,
    pub enable_ipc: bool,
    pub enable_tray: bool,
    pub enable_notifications: bool,
    pub enable_clipboard: bool,
    pub config_path: Option<PathBuf>,
    pub ipc_socket_path: Option<PathBuf>,
    pub quic_bind_addr: Option<SocketAddr>,
    pub mdns_port: Option<u16>,
}

impl Default for RuntimeOptions {
    fn default() -> Self {
        Self {
            local_device_id: default_local_device_id(),
            display_name: default_display_name(),
            everyone_nearby_timeout: Duration::from_secs(EVERYONE_NEARBY_TIMEOUT_SECS),
            enable_bluez: true,
            enable_mdns: true,
            enable_ipc: true,
            enable_tray: true,
            enable_notifications: true,
            enable_clipboard: true,
            config_path: None,
            ipc_socket_path: None,
            quic_bind_addr: None,
            mdns_port: None,
        }
    }
}

fn default_local_device_id() -> String {
    let raw = std::env::var("HOSTNAME")
        .or_else(|_| std::fs::read_to_string("/etc/hostname"))
        .unwrap_or_else(|_| "keynear".to_string());
    let cleaned: String = raw
        .trim()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') {
                c
            } else {
                '-'
            }
        })
        .take(48)
        .collect();
    if cleaned.is_empty() {
        "keynear".to_string()
    } else {
        cleaned
    }
}

fn default_display_name() -> String {
    std::env::var("HOSTNAME")
        .or_else(|_| std::fs::read_to_string("/etc/hostname"))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Keynear Linux".to_string())
}
