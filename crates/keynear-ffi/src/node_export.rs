use crate::node::OrionNodeInit;
use crate::*;
use keynear_protocol::{write_stream_kind, STREAM_CHUNK_BYTES};

#[uniffi::export]
impl OrionNodeHandle {
    #[uniffi::constructor]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        device_id: String,
        display_name: String,
        capability_flags: u32,
        quic_secret_key: Vec<u8>,
        incoming_directory: String,
        auto_accept_from_paired: bool,
        require_explicit_fd_for_single_file: bool,
    ) -> Result<Self, FfiError> {
        Self::build_with_init(OrionNodeInit {
            device_id,
            display_name,
            capability_flags,
            quic_secret_key,
            incoming_directory,
            auto_accept_from_paired,
            require_explicit_fd_for_single_file,
            bind_addr: None,
        })
    }

    pub fn local_lan_endpoint(&self) -> FfiLanEndpoint {
        FfiLanEndpoint::from(self.node.local_lan_endpoint())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn sync_peer(
        &self,
        device_id: String,
        endpoint_id: Vec<u8>,
        socket_addrs: Vec<String>,
        reconnect_secret: Vec<u8>,
        auto_accept_files: bool,
    ) -> Result<(), FfiError> {
        let eid = parse_endpoint_id_bytes(&endpoint_id)?;
        let secret = to_key_32(&reconnect_secret, "reconnectSecret")?;
        let parsed_addrs: Vec<SocketAddr> = socket_addrs
            .iter()
            .filter_map(|s| s.trim().parse::<SocketAddr>().ok())
            .collect();
        self.node
            .register_peer(&device_id, eid, parsed_addrs, secret)?;
        self.engine
            .set_peer_auto_accept(&device_id, auto_accept_files)?;
        let mut peers = self.synced_peers.lock().unwrap_or_else(|p| p.into_inner());
        if !peers.contains(&device_id) {
            peers.push(device_id);
        }
        Ok(())
    }

    pub fn remove_peer(&self, device_id: String) -> Result<(), FfiError> {
        self.node.remove_peer(&device_id)?;
        self.engine.remove_peer_policy(&device_id);
        let mut peers = self.synced_peers.lock().unwrap_or_else(|p| p.into_inner());
        peers.retain(|id| id != &device_id);
        Ok(())
    }

    pub fn set_auto_accept_from_paired(&self, enabled: bool) {
        self.engine.set_auto_accept_from_paired(enabled);
    }

    pub fn set_incoming_directory(&self, dir: String) -> Result<(), FfiError> {
        self.engine
            .set_incoming_directory(PathBuf::from(dir))
            .map_err(FfiError::from)
    }

    pub fn sync_proximity_config(&self, lock_enabled: bool, distance_meters: f64) {
        let peers = self
            .synced_peers
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clone();
        if peers.is_empty() {
            return;
        }
        let sequence = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(1)
            .max(1);
        let status = if lock_enabled {
            keynear_protocol::v1::ProximityStatus::ProximityInRange as i32
        } else {
            keynear_protocol::v1::ProximityStatus::ProximityUnspecified as i32
        };
        let state = keynear_protocol::v1::ProximityState {
            device_id: self.node.local_lan_endpoint().device_id,
            status,
            distance_meters: distance_meters as f32,
            session_locked: false,
            sequence,
        };
        self.runtime.block_on(async {
            for peer_id in peers {
                let _ = self
                    .node
                    .send_proximity_state(&peer_id, state.clone())
                    .await;
            }
        });
    }

    pub fn list_transfers(&self) -> Vec<FfiTransferSession> {
        self.engine
            .list_transfers()
            .into_iter()
            .map(FfiTransferSession::from)
            .collect()
    }

    pub fn respond_consent(
        &self,
        transfer_id: String,
        accepted: bool,
    ) -> Result<FfiTransferSession, FfiError> {
        let session = self.engine.respond_consent(&transfer_id, accepted)?;
        Ok(FfiTransferSession::from(session))
    }

    pub fn cancel_transfer(&self, transfer_id: String) -> Result<FfiTransferSession, FfiError> {
        let session = self.engine.cancel_transfer(&transfer_id)?;
        Ok(FfiTransferSession::from(session))
    }

    pub fn send_paths(
        &self,
        peer_device_id: String,
        paths: Vec<String>,
    ) -> Result<FfiTransferSession, FfiError> {
        send_paths_impl(
            &self.runtime,
            &self.engine,
            &self.node,
            peer_device_id,
            paths,
        )
    }

    #[cfg(unix)]
    #[allow(clippy::too_many_arguments)]
    pub fn send_fd(
        &self,
        peer_device_id: String,
        file_name: String,
        mime_type: String,
        size_bytes: u64,
        raw_fd: i32,
    ) -> Result<FfiTransferSession, FfiError> {
        send_fd_impl(
            &self.runtime,
            &self.engine,
            &self.node,
            SendFdParams {
                peer_device_id,
                file_name,
                mime_type,
                size_bytes,
                raw_fd,
            },
        )
    }

    #[cfg(unix)]
    pub fn provide_receive_fd(&self, transfer_id: String, raw_fd: i32) -> Result<(), FfiError> {
        provide_receive_fd_impl(&self.engine, transfer_id, raw_fd)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn set_clipboard_config(
        &self,
        clipboard_sync_enabled: bool,
        clipboard_images_enabled: bool,
        otp_passthrough_enabled: bool,
        ignored_mime_types: Vec<String>,
    ) {
        set_clipboard_config_impl(
            &self.clipboard,
            clipboard_sync_enabled,
            clipboard_images_enabled,
            otp_passthrough_enabled,
            ignored_mime_types,
        );
    }

    pub fn send_clipboard_text(&self, text: String, is_otp: bool) -> Result<(), FfiError> {
        send_clipboard_text_impl(
            &self.runtime,
            &self.clipboard,
            &self.node,
            &self.synced_peers,
            text,
            is_otp,
        )
    }

    pub fn send_clipboard_blob(&self, mime_type: String, data: Vec<u8>) -> Result<(), FfiError> {
        send_clipboard_blob_impl(
            &self.runtime,
            &self.clipboard,
            &self.node,
            &self.synced_peers,
            mime_type,
            data,
        )
    }

    pub fn poll_incoming_clipboard(&self) -> Vec<FfiClipboardItem> {
        poll_incoming_clipboard_impl(&self.clipboard)
    }

    pub fn fetch_clipboard_offer(
        &self,
        peer_device_id: String,
        offer_id: String,
        mime_type: String,
    ) -> Result<FfiClipboardItem, FfiError> {
        fetch_clipboard_offer_impl(
            &self.runtime,
            &self.clipboard,
            &self.node,
            peer_device_id,
            offer_id,
            mime_type,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn read_remote_vfs_chunk(
        &self,
        peer_device_id: String,
        relative_path: String,
        offset: u64,
        max_bytes: u32,
    ) -> Result<Vec<u8>, FfiError> {
        let path = crate::vfs_read::validate_relative_path(&relative_path)?;
        if max_bytes == 0 || max_bytes > STREAM_CHUNK_BYTES as u32 {
            return Err(FfiError::Validation {
                field: "maxBytes".to_string(),
                detail: "max_bytes must be 1..=STREAM_CHUNK_BYTES".to_string(),
            });
        }
        self.runtime
            .block_on(async {
                let conn = self.node.connect_peer(&peer_device_id).await?;
                let (mut send, mut recv) = conn.open_bi().await.map_err(|err| {
                    KeynearError::Transport(format!("open vfs read stream failed: {err}"))
                })?;
                write_stream_kind(&mut send, QuicStreamKind::VfsRead).await?;
                crate::vfs_read::write_vfs_read_request(&mut send, &path, offset, max_bytes)
                    .await?;
                send.finish().map_err(|err| {
                    KeynearError::Transport(format!("finish vfs read stream failed: {err}"))
                })?;
                crate::vfs_read::read_vfs_read_reply(&mut recv, max_bytes).await
            })
            .map_err(FfiError::from)
    }

    pub fn shutdown(&self) {
        if let Some(task) = self
            .accept_task
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .take()
        {
            task.abort();
        }
        self.runtime.block_on(self.node.close());
    }
}

impl Drop for OrionNodeHandle {
    fn drop(&mut self) {
        if let Some(task) = self
            .accept_task
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .take()
        {
            task.abort();
        }
    }
}
