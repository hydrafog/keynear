use super::*;

impl OrionQuicNode {
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
        let _ = self
            .outbound_tx
            .send((entry.device_id.clone(), conn.clone()));
        Ok(conn)
    }

    pub async fn invalidate_peer_connection(&self, device_id: &str) {
        let Ok(clean_id) = sanitize_device_id(device_id) else {
            return;
        };
        let endpoint_id = {
            let Ok(peers) = self.peers_by_device.read() else {
                return;
            };
            peers.get(&clean_id).map(|p| p.endpoint_id)
        };
        if let Some(eid) = endpoint_id {
            let mut cache = self.connections.lock().await;
            cache.remove(&eid);
        }
    }

    pub async fn accept_authenticated(&self) -> Result<Option<(String, Connection)>, KeynearError> {
        let mut outbound_rx = self.outbound_rx.lock().await;
        tokio::select! {
            incoming = self.endpoint.accept() => {
                drop(outbound_rx);
                let Some(incoming) = incoming else {
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
            outbound = outbound_rx.recv() => {
                Ok(outbound)
            }
        }
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
}
