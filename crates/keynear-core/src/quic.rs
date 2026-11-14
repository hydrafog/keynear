use std::collections::HashMap;
use std::fmt;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

use hmac::{Hmac, KeyInit, Mac};
use iroh::endpoint::{presets, AfterHandshakeOutcome, Connection, Endpoint, EndpointHooks};
use iroh::{EndpointAddr, EndpointId, RelayMode, SecretKey, TransportAddr};
use rustls::crypto::{aws_lc_rs, CryptoProvider};
use sha2::Sha256;
use subtle::ConstantTimeEq;
use tokio::sync::Mutex as AsyncMutex;
use tracing::{info, warn};
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

use crate::error::KeynearError;
use crate::keyring::SecretStore;
use crate::model::{sanitize_device_id, PairedDevice, QUIC_ALPN};
use crate::proto::{
    read_control_envelope, read_stream_kind, v1, write_control_envelope, write_stream_kind,
    QuicStreamKind,
};

type HmacSha256 = Hmac<Sha256>;

const HANDSHAKE_AUTH_DOMAIN: &[u8] = b"keynear-orion-handshake-v1";
const QUIC_UNAUTHORIZED_CODE: u32 = 403;

pub fn build_orion_crypto_provider() -> Arc<CryptoProvider> {
    let mut provider = aws_lc_rs::default_provider();
    provider.kx_groups = vec![aws_lc_rs::kx_group::X25519MLKEM768];
    provider.cipher_suites = vec![
        aws_lc_rs::cipher_suite::TLS13_AES_256_GCM_SHA384,
        aws_lc_rs::cipher_suite::TLS13_CHACHA20_POLY1305_SHA256,
        aws_lc_rs::cipher_suite::TLS13_AES_128_GCM_SHA256,
    ];
    Arc::new(provider)
}

pub fn compute_handshake_auth_proof(
    reconnect_secret: &[u8; 32],
    local_endpoint_id: &EndpointId,
    remote_endpoint_id: &EndpointId,
    local_device_id: &str,
) -> Result<[u8; 32], KeynearError> {
    let clean_id = sanitize_device_id(local_device_id)?;
    let mut mac = <HmacSha256 as KeyInit>::new_from_slice(reconnect_secret)
        .map_err(|err| KeynearError::Crypto(format!("invalid hmac key: {err}")))?;
    mac.update(HANDSHAKE_AUTH_DOMAIN);
    mac.update(local_endpoint_id.as_bytes());
    mac.update(remote_endpoint_id.as_bytes());
    mac.update(clean_id.as_bytes());
    let digest = mac.finalize().into_bytes();
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest);
    Ok(out)
}

pub fn verify_handshake_auth_proof(
    reconnect_secret: &[u8; 32],
    sender_endpoint_id: &EndpointId,
    receiver_endpoint_id: &EndpointId,
    sender_device_id: &str,
    proof: &[u8],
) -> Result<(), KeynearError> {
    if proof.len() != 32 {
        return Err(KeynearError::UnauthorizedPeer(
            "invalid handshake auth proof length".to_string(),
        ));
    }
    let expected = compute_handshake_auth_proof(
        reconnect_secret,
        sender_endpoint_id,
        receiver_endpoint_id,
        sender_device_id,
    )?;
    if !bool::from(proof.ct_eq(&expected)) {
        return Err(KeynearError::UnauthorizedPeer(
            "handshake auth proof verification failed".to_string(),
        ));
    }
    Ok(())
}

#[derive(Clone)]
pub struct OrionNodeConfig {
    pub device_id: String,
    pub display_name: String,
    pub capability_flags: u32,
    pub quic_secret_key: Zeroizing<[u8; 32]>,
}

impl fmt::Debug for OrionNodeConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OrionNodeConfig")
            .field("device_id", &self.device_id)
            .field("display_name", &self.display_name)
            .field("capability_flags", &self.capability_flags)
            .field("quic_secret_key", &"[REDACTED]")
            .finish()
    }
}

#[derive(Clone, Zeroize, ZeroizeOnDrop)]
struct PeerAuthEntry {
    #[zeroize(skip)]
    device_id: String,
    #[zeroize(skip)]
    endpoint_id: EndpointId,
    #[zeroize(skip)]
    socket_addrs: Vec<SocketAddr>,
    reconnect_secret: [u8; 32],
}

#[derive(Debug, Clone)]
struct PairedPeerHooks {
    allowlist: Arc<RwLock<HashMap<EndpointId, String>>>,
}

impl EndpointHooks for PairedPeerHooks {
    async fn after_handshake(&self, conn: &Connection) -> AfterHandshakeOutcome {
        let remote_id = conn.remote_id();
        let allowed = self
            .allowlist
            .read()
            .map(|guard| guard.contains_key(&remote_id))
            .unwrap_or(false);
        if allowed {
            AfterHandshakeOutcome::Accept
        } else {
            warn!(
                event = "quic_unpaired_peer_rejected",
                remote_id = %remote_id
            );
            AfterHandshakeOutcome::Reject {
                error_code: QUIC_UNAUTHORIZED_CODE.into(),
                reason: b"unpaired_peer".to_vec(),
            }
        }
    }
}

#[derive(Clone)]
pub struct OrionQuicNode {
    endpoint: Endpoint,
    config: OrionNodeConfig,
    allowlist: Arc<RwLock<HashMap<EndpointId, String>>>,
    peers_by_device: Arc<RwLock<HashMap<String, PeerAuthEntry>>>,
    connections: Arc<AsyncMutex<HashMap<EndpointId, Connection>>>,
}

impl OrionQuicNode {
    pub async fn bind(
        mut config: OrionNodeConfig,
        bind_addr: Option<SocketAddr>,
    ) -> Result<Self, KeynearError> {
        config.device_id = sanitize_device_id(&config.device_id)?;
        let allowlist = Arc::new(RwLock::new(HashMap::new()));
        let peers_by_device = Arc::new(RwLock::new(HashMap::new()));

        let mut builder = Endpoint::builder(presets::Empty)
            .secret_key(SecretKey::from_bytes(&config.quic_secret_key))
            .crypto_provider(build_orion_crypto_provider())
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

        Ok(Self {
            endpoint,
            config,
            allowlist,
            peers_by_device,
            connections: Arc::new(AsyncMutex::new(HashMap::new())),
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

    pub async fn connect_peer(&self, device_id: &str) -> Result<Connection, KeynearError> {
        let clean_id = sanitize_device_id(device_id)?;
        let entry = {
            let peers = self.peers_by_device.read().map_err(|_| {
                KeynearError::Transport("peer auth registry lock poisoned".to_string())
            })?;
            peers.get(&clean_id).cloned().ok_or_else(|| {
                KeynearError::UnauthorizedPeer(format!(
                    "peer '{clean_id}' has no registered LAN endpoint"
                ))
            })?
        };

        {
            let mut cache = self.connections.lock().await;
            if let Some(existing) = cache.get(&entry.endpoint_id) {
                if existing.close_reason().is_none() {
                    return Ok(existing.clone());
                }
            }
            cache.remove(&entry.endpoint_id);
        }

        if entry.socket_addrs.is_empty() {
            return Err(KeynearError::Transport(format!(
                "peer '{clean_id}' has no direct LAN socket addresses"
            )));
        }

        let addr = EndpointAddr::from_parts(
            entry.endpoint_id,
            entry.socket_addrs.iter().copied().map(TransportAddr::Ip),
        );
        let conn = self
            .endpoint
            .connect(addr, QUIC_ALPN)
            .await
            .map_err(|err| KeynearError::Transport(format!("QUIC connect failed: {err}")))?;

        if let Err(err) = self
            .authenticate_outbound(&conn, &entry.device_id, &entry.reconnect_secret)
            .await
        {
            conn.close(QUIC_UNAUTHORIZED_CODE.into(), b"auth_failed");
            return Err(err);
        }

        let mut cache = self.connections.lock().await;
        cache.insert(entry.endpoint_id, conn.clone());
        Ok(conn)
    }

    pub async fn accept_authenticated(&self) -> Result<Option<(String, Connection)>, KeynearError> {
        let Some(incoming) = self.endpoint.accept().await else {
            return Ok(None);
        };
        let connecting = incoming
            .accept()
            .map_err(|err| KeynearError::Transport(format!("QUIC accept failed: {err}")))?;
        let conn = connecting
            .await
            .map_err(|err| KeynearError::Transport(format!("QUIC handshake failed: {err}")))?;

        match self.authenticate_inbound(&conn).await {
            Ok(device_id) => {
                let mut cache = self.connections.lock().await;
                cache.insert(conn.remote_id(), conn.clone());
                Ok(Some((device_id, conn)))
            }
            Err(err) => {
                conn.close(QUIC_UNAUTHORIZED_CODE.into(), b"auth_failed");
                Err(err)
            }
        }
    }

    pub async fn authenticate_outbound(
        &self,
        conn: &Connection,
        expected_peer_id: &str,
        reconnect_secret: &[u8; 32],
    ) -> Result<(), KeynearError> {
        let remote_eid = conn.remote_id();
        let (mut send, mut recv) = conn
            .open_bi()
            .await
            .map_err(|err| KeynearError::Transport(format!("open auth stream failed: {err}")))?;

        write_stream_kind(&mut send, QuicStreamKind::Control).await?;
        let local_handshake = self.build_handshake_envelope(&remote_eid, reconnect_secret)?;
        write_control_envelope(&mut send, &local_handshake).await?;
        send.finish()
            .map_err(|err| KeynearError::Transport(format!("finish auth stream failed: {err}")))?;

        let kind = read_stream_kind(&mut recv).await?;
        if kind != QuicStreamKind::Control {
            return Err(KeynearError::Protocol(
                "expected control stream for handshake".to_string(),
            ));
        }
        let reply = read_control_envelope(&mut recv).await?;
        let Some(v1::envelope::Payload::Handshake(peer_hs)) = reply.payload else {
            return Err(KeynearError::Protocol(
                "expected HandshakePayload in auth reply".to_string(),
            ));
        };
        if peer_hs.device_id != expected_peer_id {
            return Err(KeynearError::UnauthorizedPeer(
                "peer device_id mismatch during QUIC handshake".to_string(),
            ));
        }
        verify_handshake_auth_proof(
            reconnect_secret,
            &remote_eid,
            &self.endpoint.id(),
            &peer_hs.device_id,
            &peer_hs.auth_proof,
        )
    }

    pub async fn authenticate_inbound(&self, conn: &Connection) -> Result<String, KeynearError> {
        let remote_eid = conn.remote_id();
        let entry = {
            let allowlist = self
                .allowlist
                .read()
                .map_err(|_| KeynearError::Transport("allowlist lock poisoned".to_string()))?;
            let Some(device_id) = allowlist.get(&remote_eid) else {
                return Err(KeynearError::UnauthorizedPeer(
                    "remote endpoint is not paired".to_string(),
                ));
            };
            let peers = self.peers_by_device.read().map_err(|_| {
                KeynearError::Transport("peer auth registry lock poisoned".to_string())
            })?;
            peers.get(device_id).cloned().ok_or_else(|| {
                KeynearError::UnauthorizedPeer("paired peer secret missing".to_string())
            })?
        };

        let (mut send, mut recv) = conn
            .accept_bi()
            .await
            .map_err(|err| KeynearError::Transport(format!("accept auth stream failed: {err}")))?;

        let kind = read_stream_kind(&mut recv).await?;
        if kind != QuicStreamKind::Control {
            return Err(KeynearError::Protocol(
                "expected control stream for inbound handshake".to_string(),
            ));
        }
        let env = read_control_envelope(&mut recv).await?;
        let Some(v1::envelope::Payload::Handshake(peer_hs)) = env.payload else {
            return Err(KeynearError::Protocol(
                "expected HandshakePayload on inbound connection".to_string(),
            ));
        };
        if peer_hs.device_id != entry.device_id {
            return Err(KeynearError::UnauthorizedPeer(
                "inbound handshake device_id mismatch".to_string(),
            ));
        }
        verify_handshake_auth_proof(
            &entry.reconnect_secret,
            &remote_eid,
            &self.endpoint.id(),
            &peer_hs.device_id,
            &peer_hs.auth_proof,
        )?;

        write_stream_kind(&mut send, QuicStreamKind::Control).await?;
        let reply = self.build_handshake_envelope(&remote_eid, &entry.reconnect_secret)?;
        write_control_envelope(&mut send, &reply).await?;
        send.finish()
            .map_err(|err| KeynearError::Transport(format!("finish auth reply failed: {err}")))?;

        Ok(entry.device_id.clone())
    }

    pub async fn close(&self) {
        let conns: Vec<Connection> = {
            let mut guard = self.connections.lock().await;
            guard.drain().map(|(_, c)| c).collect()
        };
        for conn in conns {
            conn.close(0u32.into(), b"shutdown");
        }
        self.endpoint.close().await;
    }

    fn build_handshake_envelope(
        &self,
        remote_endpoint_id: &EndpointId,
        reconnect_secret: &[u8; 32],
    ) -> Result<v1::Envelope, KeynearError> {
        let proof = compute_handshake_auth_proof(
            reconnect_secret,
            &self.endpoint.id(),
            remote_endpoint_id,
            &self.config.device_id,
        )?;
        Ok(v1::Envelope {
            version: 1,
            sequence: 1,
            timestamp_ms: now_millis(),
            payload: Some(v1::envelope::Payload::Handshake(v1::HandshakePayload {
                device_id: self.config.device_id.clone(),
                display_name: self.config.display_name.clone(),
                ephemeral_pubkey: self.endpoint.id().as_bytes().to_vec(),
                capability_flags: self.config.capability_flags,
                auth_proof: proof.to_vec(),
            })),
        })
    }
}

pub fn parse_endpoint_id_hex(raw: &str) -> Option<EndpointId> {
    let bytes = hex::decode(raw.trim()).ok()?;
    let arr: [u8; 32] = bytes.try_into().ok()?;
    EndpointId::from_bytes(&arr).ok()
}

pub fn parse_endpoint_id_bytes(bytes: &[u8]) -> Result<EndpointId, KeynearError> {
    let arr: [u8; 32] = bytes.try_into().map_err(|_| KeynearError::Validation {
        field: "endpointId",
        message: "endpoint_id must be 32 bytes".to_string(),
    })?;
    EndpointId::from_bytes(&arr).map_err(|err| KeynearError::Validation {
        field: "endpointId",
        message: format!("invalid Ed25519 endpoint_id: {err}"),
    })
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    fn loopback_addr() -> SocketAddr {
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0)
    }

    fn node_config(device_id: &str, seed_byte: u8) -> OrionNodeConfig {
        OrionNodeConfig {
            device_id: device_id.to_string(),
            display_name: format!("Node {device_id}"),
            capability_flags: 0x03,
            quic_secret_key: Zeroizing::new([seed_byte; 32]),
        }
    }

    #[tokio::test]
    async fn endpoint_disables_relays_and_negotiates_x25519mlkem768() {
        let provider = build_orion_crypto_provider();
        assert_eq!(provider.kx_groups.len(), 1);
        assert_eq!(
            provider.kx_groups[0].name(),
            aws_lc_rs::kx_group::X25519MLKEM768.name()
        );

        let server = OrionQuicNode::bind(node_config("linux-host", 0x11), Some(loopback_addr()))
            .await
            .expect("bind server");
        let client = OrionQuicNode::bind(node_config("pixel-phone", 0x22), Some(loopback_addr()))
            .await
            .expect("bind client");

        assert!(server.endpoint_addr().relay_urls().next().is_none());
        assert!(client.endpoint_addr().relay_urls().next().is_none());

        let secret = [0x99; 32];
        let server_addrs: Vec<SocketAddr> = server.endpoint_addr().ip_addrs().copied().collect();
        let client_addrs: Vec<SocketAddr> = client.endpoint_addr().ip_addrs().copied().collect();
        assert!(!server_addrs.is_empty());

        server
            .register_peer("pixel-phone", client.endpoint_id(), client_addrs, secret)
            .expect("register client");
        client
            .register_peer(
                "linux-host",
                server.endpoint_id(),
                server_addrs.clone(),
                secret,
            )
            .expect("register server");

        let server_clone = server.clone();
        let accept_task = tokio::spawn(async move { server_clone.accept_authenticated().await });

        let conn = client
            .connect_peer("linux-host")
            .await
            .expect("client connects over PQ QUIC");
        assert_eq!(conn.alpn(), QUIC_ALPN);

        let accepted = accept_task
            .await
            .expect("server join")
            .expect("server accept")
            .expect("some conn");
        assert_eq!(accepted.0, "pixel-phone");

        // Classic-only X25519 peer must fail TLS handshake because server requires X25519MLKEM768.
        let mut classic_provider = aws_lc_rs::default_provider();
        classic_provider.kx_groups = vec![aws_lc_rs::kx_group::X25519];
        let classic_client = Endpoint::builder(presets::Empty)
            .secret_key(SecretKey::from_bytes(&[0x33; 32]))
            .crypto_provider(Arc::new(classic_provider))
            .relay_mode(RelayMode::Disabled)
            .bind_addr(loopback_addr())
            .expect("bind addr")
            .bind()
            .await
            .expect("bind classic client");
        server
            .register_peer("classic-peer", classic_client.id(), vec![], [0x44; 32])
            .expect("register classic peer");
        let server_addr = EndpointAddr::from_parts(
            server.endpoint_id(),
            server_addrs.into_iter().map(TransportAddr::Ip),
        );
        assert!(
            classic_client
                .connect(server_addr, QUIC_ALPN)
                .await
                .is_err(),
            "classic-only peer without X25519MLKEM768 must be rejected"
        );

        classic_client.close().await;
        client.close().await;
        server.close().await;
    }

    #[tokio::test]
    async fn rejects_unpaired_and_invalid_hmac() {
        let server = OrionQuicNode::bind(node_config("linux-host", 0x51), Some(loopback_addr()))
            .await
            .expect("bind server");
        let rogue = OrionQuicNode::bind(node_config("rogue-node", 0x52), Some(loopback_addr()))
            .await
            .expect("bind rogue");
        let paired = OrionQuicNode::bind(node_config("pixel-phone", 0x53), Some(loopback_addr()))
            .await
            .expect("bind paired");

        let server_addrs: Vec<SocketAddr> = server.endpoint_addr().ip_addrs().copied().collect();

        // Unpaired peer is rejected by PairedPeerHooks::after_handshake.
        rogue
            .register_peer(
                "linux-host",
                server.endpoint_id(),
                server_addrs.clone(),
                [0xAA; 32],
            )
            .expect("rogue registers server");
        let server_for_rogue = server.clone();
        let rogue_accept = tokio::spawn(async move {
            if let Some(incoming) = server_for_rogue.endpoint().accept().await {
                if let Ok(connecting) = incoming.accept() {
                    return connecting.await.is_ok();
                }
            }
            false
        });
        let rogue_res = rogue.connect_peer("linux-host").await;
        assert!(rogue_res.is_err(), "unpaired peer must be rejected");
        let _ = rogue_accept.await;

        // Peer in allowlist with mismatched reconnect_secret fails HMAC verification.
        server
            .register_peer("pixel-phone", paired.endpoint_id(), vec![], [0x11; 32])
            .expect("server registers paired");
        paired
            .register_peer("linux-host", server.endpoint_id(), server_addrs, [0x22; 32])
            .expect("paired registers server with wrong secret");

        let server_for_hmac = server.clone();
        let hmac_accept = tokio::spawn(async move { server_for_hmac.accept_authenticated().await });
        let client_res = paired.connect_peer("linux-host").await;
        assert!(
            client_res.is_err(),
            "mismatched reconnect_secret must fail handshake auth, got {client_res:?}"
        );
        let server_res = hmac_accept.await.expect("join");
        assert!(
            matches!(server_res, Err(KeynearError::UnauthorizedPeer(_))),
            "server must reject invalid HMAC auth_proof"
        );

        rogue.close().await;
        paired.close().await;
        server.close().await;
    }
}
