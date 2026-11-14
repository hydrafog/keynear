use std::net::{IpAddr, SocketAddr};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use mdns_sd::{DaemonEvent, ResolvedService, ScopedIp, ServiceDaemon, ServiceEvent, ServiceInfo};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use crate::error::KeynearError;
use crate::model::sanitize_device_id;
use crate::proto::v1;

pub const KEYNEAR_MDNS_SERVICE_TYPE: &str = "_keynear._udp.local.";
const TXT_DEVICE_ID: &str = "id";
const TXT_ENDPOINT_ID: &str = "eid";
const TXT_CAPABILITY_FLAGS: &str = "cap";
const MDNS_CHANNEL_CAPACITY: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredLanPeer {
    pub device_id: String,
    pub endpoint_id: Vec<u8>,
    pub socket_addrs: Vec<SocketAddr>,
    pub capability_flags: u32,
}

impl DiscoveredLanPeer {
    pub fn to_lan_endpoint(&self) -> v1::LanEndpoint {
        let issued_at_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        v1::LanEndpoint {
            device_id: self.device_id.clone(),
            endpoint_id: self.endpoint_id.clone(),
            socket_addrs: self.socket_addrs.iter().map(ToString::to_string).collect(),
            capability_flags: self.capability_flags,
            issued_at_ms,
        }
    }
}

pub struct MdnsDiscovery {
    daemon: Option<ServiceDaemon>,
    degraded: Arc<AtomicBool>,
    logged_degraded: Arc<AtomicBool>,
}

impl MdnsDiscovery {
    pub fn new() -> Self {
        Self::from_result(ServiceDaemon::new())
    }

    pub fn new_with_port(port: u16) -> Self {
        Self::from_result(ServiceDaemon::new_with_port(port))
    }

    fn from_result(result: mdns_sd::Result<ServiceDaemon>) -> Self {
        let degraded = Arc::new(AtomicBool::new(false));
        let logged_degraded = Arc::new(AtomicBool::new(false));
        match result {
            Ok(daemon) => {
                if let Ok(monitor) = daemon.monitor() {
                    let degraded_flag = Arc::clone(&degraded);
                    let logged_flag = Arc::clone(&logged_degraded);
                    if let Ok(handle) = tokio::runtime::Handle::try_current() {
                        handle.spawn(async move {
                            while let Ok(event) = monitor.recv_async().await {
                                record_daemon_event(&event, &degraded_flag, &logged_flag);
                            }
                        });
                    }
                }
                Self {
                    daemon: Some(daemon),
                    degraded,
                    logged_degraded,
                }
            }
            Err(err) => {
                mark_degraded(&degraded, &logged_degraded, &err.to_string());
                Self {
                    daemon: None,
                    degraded,
                    logged_degraded,
                }
            }
        }
    }

    pub fn is_degraded(&self) -> bool {
        self.degraded.load(Ordering::Relaxed) || self.daemon.is_none()
    }

    pub fn handle_monitor_event(&self, event: &DaemonEvent) -> bool {
        record_daemon_event(event, &self.degraded, &self.logged_degraded)
    }

    pub fn register(
        &self,
        device_id: &str,
        endpoint_id: &[u8; 32],
        port: u16,
        addrs: &[IpAddr],
        capability_flags: u32,
    ) -> Result<Option<String>, KeynearError> {
        let clean_id = sanitize_device_id(device_id)?;
        let Some(daemon) = &self.daemon else {
            return Ok(None);
        };
        if port == 0 || addrs.is_empty() {
            return Ok(None);
        }

        let eid_hex = hex::encode(endpoint_id);
        let cap_str = capability_flags.to_string();
        let instance_name = format!("keynear-{clean_id}");
        let host_name = format!("{clean_id}.local.");
        let props = [
            (TXT_DEVICE_ID, clean_id.as_str()),
            (TXT_ENDPOINT_ID, eid_hex.as_str()),
            (TXT_CAPABILITY_FLAGS, cap_str.as_str()),
        ];

        let service_info = match ServiceInfo::new(
            KEYNEAR_MDNS_SERVICE_TYPE,
            &instance_name,
            &host_name,
            addrs,
            port,
            &props[..],
        ) {
            Ok(info) => info,
            Err(err) => {
                mark_degraded(&self.degraded, &self.logged_degraded, &err.to_string());
                return Ok(None);
            }
        };

        let fullname = service_info.get_fullname().to_string();
        match daemon.register(service_info) {
            Ok(()) => {
                info!(
                    event = "mdns_service_registered",
                    device_id = %clean_id,
                    fullname = %fullname,
                    port
                );
                Ok(Some(fullname))
            }
            Err(err) => {
                mark_degraded(&self.degraded, &self.logged_degraded, &err.to_string());
                Ok(None)
            }
        }
    }

    pub fn browse(&self, local_device_id: &str) -> Option<mpsc::Receiver<DiscoveredLanPeer>> {
        let daemon = self.daemon.as_ref()?;
        let rx = match daemon.browse(KEYNEAR_MDNS_SERVICE_TYPE) {
            Ok(rx) => rx,
            Err(err) => {
                mark_degraded(&self.degraded, &self.logged_degraded, &err.to_string());
                return None;
            }
        };

        let (tx, out_rx) = mpsc::channel(MDNS_CHANNEL_CAPACITY);
        let local_id = local_device_id.to_string();
        let degraded_flag = Arc::clone(&self.degraded);
        let logged_flag = Arc::clone(&self.logged_degraded);
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                while let Ok(event) = rx.recv_async().await {
                    if let ServiceEvent::ServiceResolved(resolved) = event {
                        if let Some(peer) = parse_resolved_service(&resolved) {
                            if peer.device_id == local_id {
                                continue;
                            }
                            if tx.send(peer).await.is_err() {
                                break;
                            }
                        }
                    }
                }
            });
        } else {
            mark_degraded(
                &degraded_flag,
                &logged_flag,
                "tokio runtime missing for mdns browse",
            );
            return None;
        }
        Some(out_rx)
    }

    pub fn shutdown(&self) {
        if let Some(daemon) = &self.daemon {
            if let Ok(status) = daemon.shutdown() {
                let _ = status.recv();
            }
        }
    }
}

impl Default for MdnsDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

pub fn parse_resolved_service(resolved: &ResolvedService) -> Option<DiscoveredLanPeer> {
    let raw_id = resolved
        .txt_properties
        .get_property_val_str(TXT_DEVICE_ID)?;
    let device_id = sanitize_device_id(raw_id).ok()?;

    let eid_hex = resolved
        .txt_properties
        .get_property_val_str(TXT_ENDPOINT_ID)?;
    let endpoint_id = hex::decode(eid_hex.trim()).ok()?;
    if endpoint_id.len() != 32 {
        return None;
    }

    let capability_flags = resolved
        .txt_properties
        .get_property_val_str(TXT_CAPABILITY_FLAGS)
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(0);

    let mut socket_addrs: Vec<SocketAddr> = resolved
        .addresses
        .iter()
        .map(|scoped| {
            let ip = match scoped {
                ScopedIp::V4(v4) => IpAddr::V4(*v4.addr()),
                ScopedIp::V6(v6) => IpAddr::V6(*v6.addr()),
                other => other.to_ip_addr(),
            };
            SocketAddr::new(ip, resolved.port)
        })
        .collect();
    socket_addrs.sort();
    socket_addrs.dedup();
    if socket_addrs.is_empty() || resolved.port == 0 {
        return None;
    }

    Some(DiscoveredLanPeer {
        device_id,
        endpoint_id,
        socket_addrs,
        capability_flags,
    })
}

fn record_daemon_event(
    event: &DaemonEvent,
    degraded: &AtomicBool,
    logged_degraded: &AtomicBool,
) -> bool {
    match event {
        DaemonEvent::Error(err) => {
            mark_degraded(degraded, logged_degraded, &err.to_string());
            true
        }
        DaemonEvent::IpAdd(ip) => {
            debug!(event = "mdns_ip_added", %ip);
            false
        }
        DaemonEvent::IpDel(ip) => {
            debug!(event = "mdns_ip_removed", %ip);
            false
        }
        _ => false,
    }
}

fn mark_degraded(degraded: &AtomicBool, logged_degraded: &AtomicBool, reason: &str) {
    degraded.store(true, Ordering::Relaxed);
    if !logged_degraded.swap(true, Ordering::AcqRel) {
        warn!(event = "mdns_degraded", reason = %reason);
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use mdns_sd::Error as MdnsError;
    use std::net::Ipv4Addr;

    #[tokio::test]
    async fn handles_isolated_port_and_degraded_monitor() {
        // Use an unprivileged ephemeral port above 1024 to avoid binding 5353 in CI sandboxes.
        let probe = std::net::UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).expect("ephemeral udp");
        let port = probe.local_addr().expect("local addr").port();
        drop(probe);

        let discovery = MdnsDiscovery::new_with_port(port);
        assert!(!discovery.is_degraded());

        let registered = discovery
            .register(
                "linux-1",
                &[0x42; 32],
                49152,
                &[IpAddr::V4(Ipv4Addr::LOCALHOST)],
                0x03,
            )
            .expect("register returns Ok even on sandboxed sockets");
        assert!(registered.is_some());

        // Simulate a daemon monitor error: transitions non-fatally to degraded and logs once.
        assert!(
            discovery.handle_monitor_event(&DaemonEvent::Error(MdnsError::Msg(
                "multicast unavailable".to_string(),
            )))
        );
        assert!(discovery.is_degraded());
        assert!(
            discovery.handle_monitor_event(&DaemonEvent::Error(MdnsError::Msg(
                "repeated multicast error".to_string(),
            )))
        );
        assert!(discovery.is_degraded());

        // Verify ResolvedService parsing produces sanitized DiscoveredLanPeer.
        let eid_hex = hex::encode([0x55; 32]);
        let props = [("id", "pixel-9"), ("eid", eid_hex.as_str()), ("cap", "7")];
        let info = ServiceInfo::new(
            KEYNEAR_MDNS_SERVICE_TYPE,
            "keynear-pixel-9",
            "pixel-9.local.",
            &[IpAddr::V4(Ipv4Addr::new(192, 168, 1, 20))][..],
            45000,
            &props[..],
        )
        .expect("construct service info");
        let resolved = info.as_resolved_service();
        let peer = parse_resolved_service(&resolved).expect("parse valid resolved service");
        assert_eq!(peer.device_id, "pixel-9");
        assert_eq!(peer.endpoint_id, vec![0x55; 32]);
        assert_eq!(peer.capability_flags, 7);
        assert_eq!(
            peer.socket_addrs,
            vec![SocketAddr::new(
                IpAddr::V4(Ipv4Addr::new(192, 168, 1, 20)),
                45000
            )]
        );

        discovery.shutdown();
    }
}
