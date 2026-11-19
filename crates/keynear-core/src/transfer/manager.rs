use std::collections::HashMap;
use std::fs::File;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use iroh::endpoint::{Connection, RecvStream, SendStream};
use tracing::{info, warn};
use uuid::Uuid;

use keynear_protocol::{
    read_control_envelope, read_stream_kind, sanitize_device_id, v1, write_control_envelope,
    KeynearError, QuicStreamKind, TransferItem, TransferSession, TransferState,
};

use crate::transport::OrionQuicNode;

use super::framing::{send_cancel_frame, send_consent_frame, wrap_payload, CONSENT_TIMEOUT_SECS};
use super::io::{
    expand_tilde, prepare_send_plan, receive_raw_file_stream, receive_zip_stream,
    sanitize_relative_path, send_file_handle_stream, send_zip_stream, OutboundPayload,
};
use super::state::{PeerTransferPolicy, TransferRegistry};

pub struct TransferEngine {
    registry: Arc<TransferRegistry>,
    incoming_directory: Mutex<PathBuf>,
    auto_accept_from_paired: AtomicBool,
    peer_policies: Mutex<HashMap<String, PeerTransferPolicy>>,
    require_explicit_fd_for_single_file: AtomicBool,
}

impl TransferEngine {
    pub fn new(incoming_directory: PathBuf, auto_accept_from_paired: bool) -> Self {
        Self {
            registry: Arc::new(TransferRegistry::new()),
            incoming_directory: Mutex::new(expand_tilde(&incoming_directory)),
            auto_accept_from_paired: AtomicBool::new(auto_accept_from_paired),
            peer_policies: Mutex::new(HashMap::new()),
            require_explicit_fd_for_single_file: AtomicBool::new(false),
        }
    }

    pub fn set_require_explicit_fd_for_single_file(&self, required: bool) {
        self.require_explicit_fd_for_single_file
            .store(required, Ordering::Release);
    }

    pub fn set_incoming_directory(&self, dir: PathBuf) -> Result<(), KeynearError> {
        if dir.as_os_str().is_empty() {
            return Err(KeynearError::Validation {
                field: "incomingDirectory",
                message: "incoming directory must not be empty".to_string(),
            });
        }
        let mut guard = self
            .incoming_directory
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        *guard = expand_tilde(&dir);
        Ok(())
    }

    pub fn incoming_directory(&self) -> PathBuf {
        self.incoming_directory
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clone()
    }

    pub fn set_auto_accept_from_paired(&self, enabled: bool) {
        self.auto_accept_from_paired
            .store(enabled, Ordering::Release);
    }

    pub fn set_peer_auto_accept(&self, device_id: &str, enabled: bool) -> Result<(), KeynearError> {
        let clean_id = sanitize_device_id(device_id)?;
        let mut guard = self.peer_policies.lock().unwrap_or_else(|p| p.into_inner());
        guard.insert(
            clean_id,
            PeerTransferPolicy {
                auto_accept_files: enabled,
            },
        );
        Ok(())
    }

    pub fn remove_peer_policy(&self, device_id: &str) {
        if let Ok(clean_id) = sanitize_device_id(device_id) {
            let mut guard = self.peer_policies.lock().unwrap_or_else(|p| p.into_inner());
            guard.remove(&clean_id);
        }
    }

    pub fn is_auto_accept_allowed(&self, device_id: &str) -> bool {
        if self.auto_accept_from_paired.load(Ordering::Acquire) {
            return true;
        }
        let Ok(clean_id) = sanitize_device_id(device_id) else {
            return false;
        };
        let guard = self.peer_policies.lock().unwrap_or_else(|p| p.into_inner());
        guard
            .get(&clean_id)
            .map(|p| p.auto_accept_files)
            .unwrap_or(false)
    }

    pub fn registry(&self) -> &Arc<TransferRegistry> {
        &self.registry
    }

    pub fn list_transfers(&self) -> Vec<TransferSession> {
        self.registry.list_sessions()
    }

    pub fn respond_consent(
        &self,
        transfer_id: &str,
        accepted: bool,
    ) -> Result<TransferSession, KeynearError> {
        self.registry.respond_consent(transfer_id, accepted)
    }

    pub fn cancel_transfer(&self, transfer_id: &str) -> Result<TransferSession, KeynearError> {
        self.registry.cancel_transfer(transfer_id)
    }

    pub fn provide_receive_fd(&self, transfer_id: &str, file: File) -> Result<(), KeynearError> {
        self.registry.provide_receive_fd(transfer_id, file)
    }

    pub async fn send_paths(
        &self,
        node: &OrionQuicNode,
        peer_device_id: &str,
        paths: &[PathBuf],
    ) -> Result<TransferSession, KeynearError> {
        if paths.is_empty() {
            return Err(KeynearError::Validation {
                field: "paths",
                message: "at least one path is required for transfer".to_string(),
            });
        }
        let conn = node.connect_peer(peer_device_id).await?;
        self.send_paths_on_connection(&conn, peer_device_id, paths)
            .await
    }

    pub async fn send_paths_on_connection(
        &self,
        conn: &Connection,
        peer_device_id: &str,
        paths: &[PathBuf],
    ) -> Result<TransferSession, KeynearError> {
        let plan = prepare_send_plan(paths)?;
        let transfer_id = Uuid::new_v4().to_string();
        let cancel_flag = self.registry.register_outbound(
            transfer_id.clone(),
            peer_device_id.to_string(),
            plan.items.clone(),
            plan.total_bytes,
        )?;

        let offer = v1::TransferOffer {
            transfer_id: transfer_id.clone(),
            items: plan
                .items
                .iter()
                .map(|i| v1::TransferItemMeta {
                    name: i.name.clone(),
                    mime_type: i.mime_type.clone(),
                    size_bytes: i.size_bytes,
                    is_directory: i.is_directory,
                })
                .collect(),
            total_bytes: plan.total_bytes,
        };

        let res = match plan.payload {
            OutboundPayload::SingleFile(path) => {
                let file = File::open(&path)?;
                send_file_handle_stream(&self.registry, conn, &offer, file, &cancel_flag).await
            }
            OutboundPayload::ArchiveEntries(entries) => {
                send_zip_stream(&self.registry, conn, &offer, entries, &cancel_flag).await
            }
        };

        self.finalize_outbound_result(&transfer_id, plan.total_bytes, res)
    }

    pub async fn send_file_handle(
        &self,
        node: &OrionQuicNode,
        peer_device_id: &str,
        file_name: String,
        mime_type: String,
        size_bytes: u64,
        file: File,
    ) -> Result<TransferSession, KeynearError> {
        let conn = node.connect_peer(peer_device_id).await?;
        self.send_file_handle_on_connection(
            &conn,
            peer_device_id,
            file_name,
            mime_type,
            size_bytes,
            file,
        )
        .await
    }

    pub async fn send_file_handle_on_connection(
        &self,
        conn: &Connection,
        peer_device_id: &str,
        file_name: String,
        mime_type: String,
        size_bytes: u64,
        file: File,
    ) -> Result<TransferSession, KeynearError> {
        let clean_name = sanitize_relative_path(&file_name)?;
        let name_str = clean_name.to_string_lossy().to_string();
        let transfer_id = Uuid::new_v4().to_string();
        let items = vec![TransferItem {
            name: name_str.clone(),
            mime_type: mime_type.clone(),
            size_bytes,
            is_directory: false,
            target_path: None,
            checksum: None,
        }];
        let cancel_flag = self.registry.register_outbound(
            transfer_id.clone(),
            peer_device_id.to_string(),
            items,
            size_bytes,
        )?;

        let offer = v1::TransferOffer {
            transfer_id: transfer_id.clone(),
            items: vec![v1::TransferItemMeta {
                name: name_str,
                mime_type,
                size_bytes,
                is_directory: false,
            }],
            total_bytes: size_bytes,
        };

        let res = send_file_handle_stream(&self.registry, conn, &offer, file, &cancel_flag).await;
        self.finalize_outbound_result(&transfer_id, size_bytes, res)
    }

    fn finalize_outbound_result(
        &self,
        transfer_id: &str,
        total_bytes: u64,
        res: Result<String, KeynearError>,
    ) -> Result<TransferSession, KeynearError> {
        match res {
            Ok(digest_hex) => {
                self.registry.finish_transfer(
                    transfer_id,
                    TransferState::Completed,
                    total_bytes,
                    None,
                    Some(&digest_hex),
                );
                self.registry.get_session(transfer_id).ok_or_else(|| {
                    KeynearError::Transfer("completed transfer session missing".to_string())
                })
            }
            Err(KeynearError::TransferCancelled(reason)) => {
                self.registry
                    .finish_transfer(transfer_id, TransferState::Cancelled, 0, None, None);
                Err(KeynearError::TransferCancelled(reason))
            }
            Err(err) => {
                self.registry
                    .finish_transfer(transfer_id, TransferState::Failed, 0, None, None);
                Err(err)
            }
        }
    }

    pub async fn handle_incoming_connection(
        &self,
        peer_device_id: String,
        conn: Connection,
    ) -> Result<(), KeynearError> {
        loop {
            let Ok((send, recv)) = conn.accept_bi().await else {
                break;
            };
            if let Err(err) = self
                .handle_incoming_stream(&peer_device_id, send, recv)
                .await
            {
                warn!(
                    event = "incoming_transfer_failed",
                    peer_device_id = %peer_device_id,
                    error = %err
                );
            }
        }
        Ok(())
    }

    pub async fn handle_incoming_stream(
        &self,
        peer_device_id: &str,
        send: SendStream,
        mut recv: RecvStream,
    ) -> Result<TransferSession, KeynearError> {
        let kind = read_stream_kind(&mut recv).await?;
        self.handle_incoming_stream_with_kind(peer_device_id, kind, send, recv)
            .await
    }

    pub async fn handle_incoming_stream_with_kind(
        &self,
        peer_device_id: &str,
        kind: QuicStreamKind,
        mut send: SendStream,
        mut recv: RecvStream,
    ) -> Result<TransferSession, KeynearError> {
        if !matches!(
            kind,
            QuicStreamKind::RawFileChunk | QuicStreamKind::StoredZipArchive
        ) {
            return Err(KeynearError::Protocol(
                "unexpected stream kind on incoming transfer stream".to_string(),
            ));
        }

        let env = read_control_envelope(&mut recv).await?;
        let Some(v1::envelope::Payload::TransferOffer(offer)) = env.payload else {
            return Err(KeynearError::Protocol(
                "expected TransferOffer on incoming transfer stream".to_string(),
            ));
        };

        let auto_accept = self.is_auto_accept_allowed(peer_device_id);
        let require_fd = kind == QuicStreamKind::RawFileChunk
            && self
                .require_explicit_fd_for_single_file
                .load(Ordering::Acquire);

        let reg = self.registry.register_inbound(
            peer_device_id.to_string(),
            &offer,
            auto_accept,
            require_fd,
        )?;

        if let Some(consent_rx) = reg.consent_rx {
            let accepted =
                match tokio::time::timeout(Duration::from_secs(CONSENT_TIMEOUT_SECS), consent_rx)
                    .await
                {
                    Ok(Ok(true)) => true,
                    Ok(Ok(false)) | Ok(Err(_)) | Err(_) => false,
                };
            if !accepted {
                self.registry.finish_transfer(
                    &offer.transfer_id,
                    TransferState::Cancelled,
                    0,
                    None,
                    None,
                );
                let _ = send_consent_frame(&mut send, &offer.transfer_id, false, "declined").await;
                let _ = send.finish();
                return Err(KeynearError::TransferCancelled(
                    "incoming transfer declined or timed out".to_string(),
                ));
            }
        }

        let explicit_file = if let Some(fd_rx) = reg.fd_rx {
            match tokio::time::timeout(Duration::from_secs(CONSENT_TIMEOUT_SECS), fd_rx).await {
                Ok(Ok(file)) => Some(file),
                _ => {
                    self.registry.finish_transfer(
                        &offer.transfer_id,
                        TransferState::Cancelled,
                        0,
                        None,
                        None,
                    );
                    let _ =
                        send_consent_frame(&mut send, &offer.transfer_id, false, "no_output_fd")
                            .await;
                    let _ = send.finish();
                    return Err(KeynearError::TransferCancelled(
                        "no output file descriptor supplied".to_string(),
                    ));
                }
            }
        } else {
            None
        };

        send_consent_frame(&mut send, &offer.transfer_id, true, "accepted").await?;

        let result = match kind {
            QuicStreamKind::RawFileChunk => {
                receive_raw_file_stream(
                    &self.registry,
                    self.incoming_directory(),
                    &offer,
                    &mut recv,
                    explicit_file,
                    &reg.cancel_flag,
                )
                .await
            }
            QuicStreamKind::StoredZipArchive => {
                receive_zip_stream(
                    &self.registry,
                    self.incoming_directory(),
                    &offer,
                    &mut recv,
                    &reg.cancel_flag,
                )
                .await
            }
            QuicStreamKind::Control | QuicStreamKind::ClipboardBlob => unreachable!(),
        };

        match result {
            Ok((bytes_received, target_paths, digest_hex)) => {
                let ack = wrap_payload(v1::envelope::Payload::ControlAck(v1::ControlAck {
                    acked_sequence: 1,
                    accepted: true,
                    reason: String::new(),
                }));
                write_control_envelope(&mut send, &ack).await?;
                let _ = send.finish();
                self.registry.finish_transfer(
                    &offer.transfer_id,
                    TransferState::Completed,
                    bytes_received,
                    Some(&target_paths),
                    Some(&digest_hex),
                );
                info!(
                    event = "incoming_transfer_completed",
                    transfer_id = %offer.transfer_id,
                    bytes = bytes_received
                );
                self.registry
                    .get_session(&offer.transfer_id)
                    .ok_or_else(|| {
                        KeynearError::Transfer("completed transfer session missing".to_string())
                    })
            }
            Err(KeynearError::TransferCancelled(reason)) => {
                let _ = send_cancel_frame(&mut send, &offer.transfer_id, &reason).await;
                let _ = send.finish();
                self.registry.finish_transfer(
                    &offer.transfer_id,
                    TransferState::Cancelled,
                    0,
                    None,
                    None,
                );
                Err(KeynearError::TransferCancelled(reason))
            }
            Err(err) => {
                let _ = send_cancel_frame(&mut send, &offer.transfer_id, &err.to_string()).await;
                let _ = send.finish();
                self.registry.finish_transfer(
                    &offer.transfer_id,
                    TransferState::Failed,
                    0,
                    None,
                    None,
                );
                Err(err)
            }
        }
    }
}
