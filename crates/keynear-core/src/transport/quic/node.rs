use std::time::Duration;

use iroh::endpoint::QuicTransportConfig;

use super::config::{PairedPeerHooks, PeerAuthEntry};
use super::util::now_millis;
use super::*;

const QUIC_KEEP_ALIVE_SECS: u64 = 15;
const QUIC_MAX_IDLE_TIMEOUT_SECS: u64 = 180;

#[derive(Clone)]
pub struct OrionQuicNode {
    pub(crate) endpoint: Endpoint,
    pub(crate) config: OrionNodeConfig,
    pub(crate) allowlist: Arc<RwLock<HashMap<EndpointId, String>>>,
    pub(crate) peers_by_device: Arc<RwLock<HashMap<String, PeerAuthEntry>>>,
    pub(crate) connections: Arc<AsyncMutex<HashMap<EndpointId, Connection>>>,
    pub(crate) outbound_tx: mpsc::UnboundedSender<(String, Connection)>,
    pub(crate) outbound_rx: Arc<AsyncMutex<mpsc::UnboundedReceiver<(String, Connection)>>>,
}

impl OrionQuicNode {
    pub async fn bind(
        mut config: OrionNodeConfig,
        bind_addr: Option<SocketAddr>,
    ) -> Result<Self, KeynearError> {
        config.device_id = sanitize_device_id(&config.device_id)?;
        let allowlist = Arc::new(RwLock::new(HashMap::new()));
        let peers_by_device = Arc::new(RwLock::new(HashMap::new()));

        let idle_timeout = Duration::from_secs(QUIC_MAX_IDLE_TIMEOUT_SECS)
            .try_into()
            .map_err(|err| {
                KeynearError::Transport(format!("invalid QUIC idle timeout: {err}"))
            })?;
        let transport_config = QuicTransportConfig::builder()
            .keep_alive_interval(Duration::from_secs(QUIC_KEEP_ALIVE_SECS))
            .max_idle_timeout(Some(idle_timeout))
            .build();

        let mut builder = Endpoint::builder(presets::Empty)
            .secret_key(SecretKey::from_bytes(&config.quic_secret_key))
            .crypto_provider(build_orion_crypto_provider())
            .transport_config(transport_config)
            .relay_mode(RelayMode::Disabled)
            .alpns(vec![QUIC_ALPN.to_vec()])
            .hooks(PairedPeerHooks {
                allowlist: Arc::clone(&allowlist),
            });

        if let Some(addr) = bind_addr {
            builder = builder.bind_addr(addr).map_err(|err| {
                KeynearError::Transport(format!("invalid QUIC bind address: {err}"))
            })?;
        }

        let endpoint = builder
            .bind()
            .await
            .map_err(|err| KeynearError::Transport(format!("QUIC endpoint bind failed: {err}")))?;

        info!(
            event = "quic_endpoint_bound",
            device_id = %config.device_id,
            endpoint_id = %endpoint.id()
        );

        let (outbound_tx, outbound_rx) = mpsc::unbounded_channel();

        Ok(Self {
            endpoint,
            config,
            allowlist,
            peers_by_device,
            connections: Arc::new(AsyncMutex::new(HashMap::new())),
            outbound_tx,
            outbound_rx: Arc::new(AsyncMutex::new(outbound_rx)),
        })
    }

    pub fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }

    pub fn endpoint_id(&self) -> EndpointId {
        self.endpoint.id()
    }

    pub fn endpoint_addr(&self) -> EndpointAddr {
        self.endpoint.addr()
    }

    pub fn local_lan_endpoint(&self) -> v1::LanEndpoint {
        let addr = self.endpoint.addr();
        let socket_addrs = addr.ip_addrs().map(|a| a.to_string()).collect();
        v1::LanEndpoint {
            device_id: self.config.device_id.clone(),
            endpoint_id: self.endpoint.id().as_bytes().to_vec(),
            socket_addrs,
            capability_flags: self.config.capability_flags,
            issued_at_ms: now_millis(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn register_peer(
        &self,
        device_id: &str,
        endpoint_id: EndpointId,
        socket_addrs: Vec<SocketAddr>,
        reconnect_secret: [u8; 32],
    ) -> Result<(), KeynearError> {
        let clean_id = sanitize_device_id(device_id)?;
        let entry = PeerAuthEntry {
            device_id: clean_id.clone(),
            endpoint_id,
            socket_addrs,
            reconnect_secret,
        };
        {
            let mut peers = self.peers_by_device.write().map_err(|_| {
                KeynearError::Transport("peer auth registry lock poisoned".to_string())
            })?;
            let previous_eid = peers.get(&clean_id).map(|p| p.endpoint_id);
            peers.insert(clean_id.clone(), entry);

            let mut allowlist = self
                .allowlist
                .write()
                .map_err(|_| KeynearError::Transport("allowlist lock poisoned".to_string()))?;
            if let Some(prev) = previous_eid {
                if prev != endpoint_id {
                    allowlist.remove(&prev);
                }
            }
            allowlist.insert(endpoint_id, clean_id);
        }
        Ok(())
    }

    pub fn remove_peer(&self, device_id: &str) -> Result<(), KeynearError> {
        let clean_id = sanitize_device_id(device_id)?;
        let removed_eid = {
            let mut peers = self.peers_by_device.write().map_err(|_| {
                KeynearError::Transport("peer auth registry lock poisoned".to_string())
            })?;
            peers.remove(&clean_id).map(|p| p.endpoint_id)
        };
        if let Some(eid) = removed_eid {
            let mut allowlist = self
                .allowlist
                .write()
                .map_err(|_| KeynearError::Transport("allowlist lock poisoned".to_string()))?;
            allowlist.remove(&eid);
        }
        Ok(())
    }

    pub fn sync_paired_devices(
        &self,
        devices: &[PairedDevice],
        secrets: &dyn SecretStore,
    ) -> Result<(), KeynearError> {
        for dev in devices {
            if dev.last_endpoint_id.trim().is_empty() {
                continue;
            }
            let Some(eid) = parse_endpoint_id_hex(&dev.last_endpoint_id) else {
                continue;
            };
            let Some(peer_secrets) = secrets.get_peer_secrets(&dev.device_id)? else {
                continue;
            };
            let addrs: Vec<SocketAddr> = dev
                .last_lan_address
                .split(',')
                .filter_map(|s| s.trim().parse::<SocketAddr>().ok())
                .collect();
            self.register_peer(&dev.device_id, eid, addrs, peer_secrets.reconnect_secret)?;
        }
        Ok(())
    }

    pub fn peer_device_id(&self, endpoint_id: &EndpointId) -> Option<String> {
        self.allowlist
            .read()
            .ok()
            .and_then(|guard| guard.get(endpoint_id).cloned())
    }

    pub async fn send_proximity_state(
        &self,
        peer_device_id: &str,
        state: v1::ProximityState,
    ) -> Result<(), KeynearError> {
        let conn = self.connect_peer(peer_device_id).await?;
        let (mut send, mut recv) = conn.open_bi().await.map_err(|err| {
            KeynearError::Transport(format!("open proximity control stream failed: {err}"))
        })?;
        write_stream_kind(&mut send, QuicStreamKind::Control).await?;
        let env = v1::Envelope {
            version: 1,
            sequence: state.sequence.max(1),
            timestamp_ms: now_millis(),
            payload: Some(v1::envelope::Payload::ProximityState(state)),
        };
        write_control_envelope(&mut send, &env).await?;
        send.finish().map_err(|err| {
            KeynearError::Transport(format!("finish proximity control stream failed: {err}"))
        })?;
        let reply = read_control_envelope(&mut recv).await?;
        match reply.payload {
            Some(v1::envelope::Payload::ControlAck(ack)) if ack.accepted => Ok(()),
            Some(v1::envelope::Payload::ControlAck(ack)) => Err(KeynearError::Protocol(format!(
                "peer rejected proximity state: {}",
                ack.reason
            ))),
            _ => Err(KeynearError::Protocol(
                "expected ControlAck for proximity state".to_string(),
            )),
        }
    }
}
