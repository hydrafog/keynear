use std::io::{Seek, SeekFrom};

use super::*;

impl TransferEngine {
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
        let res = self
            .send_paths_on_connection(&conn, peer_device_id, paths)
            .await;
        if !matches!(res, Err(KeynearError::Transport(_))) {
            return res;
        }
        node.invalidate_peer_connection(peer_device_id).await;
        let fresh_conn = node.connect_peer(peer_device_id).await?;
        let retry = self
            .send_paths_on_connection(&fresh_conn, peer_device_id, paths)
            .await;
        if matches!(retry, Err(KeynearError::Transport(_))) {
            node.invalidate_peer_connection(peer_device_id).await;
        }
        retry
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

    #[allow(clippy::too_many_arguments)]
    pub async fn send_file_handle(
        &self,
        node: &OrionQuicNode,
        peer_device_id: &str,
        file_name: String,
        mime_type: String,
        size_bytes: u64,
        file: File,
    ) -> Result<TransferSession, KeynearError> {
        let retry_file = file.try_clone().ok();
        let conn = node.connect_peer(peer_device_id).await?;
        let res = self
            .send_file_handle_on_connection(
                &conn,
                peer_device_id,
                file_name.clone(),
                mime_type.clone(),
                size_bytes,
                file,
            )
            .await;
        if !matches!(res, Err(KeynearError::Transport(_))) {
            return res;
        }
        node.invalidate_peer_connection(peer_device_id).await;
        let Some(mut rewindable) = retry_file else {
            return res;
        };
        if rewindable.seek(SeekFrom::Start(0)).is_err() {
            return res;
        }
        let fresh_conn = node.connect_peer(peer_device_id).await?;
        let retry = self
            .send_file_handle_on_connection(
                &fresh_conn,
                peer_device_id,
                file_name,
                mime_type,
                size_bytes,
                rewindable,
            )
            .await;
        if matches!(retry, Err(KeynearError::Transport(_))) {
            node.invalidate_peer_connection(peer_device_id).await;
        }
        retry
    }

    #[allow(clippy::too_many_arguments)]
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
}
