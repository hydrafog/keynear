use super::*;

use super::events::{mark_degraded, record_daemon_event};

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

    #[allow(clippy::too_many_arguments)]
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
