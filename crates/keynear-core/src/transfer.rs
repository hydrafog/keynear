use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use iroh::endpoint::{Connection, RecvStream, SendStream};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use tokio::sync::{mpsc, oneshot};
use tracing::{info, warn};
use uuid::Uuid;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::error::KeynearError;
use crate::model::{
    TransferDirection, TransferItem, TransferSession, TransferState, sanitize_device_id,
    STREAM_CHUNK_BYTES, ZIP_IO_BUFFER_BYTES,
};
use crate::proto::{
    QuicStreamKind, read_control_envelope, read_stream_kind, v1, write_control_envelope,
    write_stream_kind,
};
use crate::quic::OrionQuicNode;

pub const MAX_ACTIVE_TRANSFERS: usize = 32;
const MAX_HISTORY_TRANSFERS: usize = 128;
const CONSENT_TIMEOUT_SECS: u64 = 120;
const MAX_FILES_IN_ARCHIVE: usize = 10_000;
const MAX_ZIP_STREAM_OVERHEAD_BYTES: u64 = (MAX_FILES_IN_ARCHIVE as u64) * 512 + 65_536;
const PIPE_CHANNEL_SLOTS: usize = 8;

static TEMP_SEQ: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone)]
pub struct PeerTransferPolicy {
    pub auto_accept_files: bool,
}

struct TransferEntry {
    session: TransferSession,
    consent_tx: Option<oneshot::Sender<bool>>,
    fd_tx: Option<oneshot::Sender<File>>,
    cancel_flag: Arc<AtomicBool>,
}

#[derive(Default)]
pub struct TransferRegistry {
    entries: Mutex<HashMap<String, TransferEntry>>,
}

impl TransferRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn list_sessions(&self) -> Vec<TransferSession> {
        let guard = self
            .entries
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut list: Vec<TransferSession> = guard.values().map(|e| e.session.clone()).collect();
        list.sort_by(|a, b| b.started_at.cmp(&a.started_at).then_with(|| a.transfer_id.cmp(&b.transfer_id)));
        list
    }

    pub fn get_session(&self, transfer_id: &str) -> Option<TransferSession> {
        let guard = self
            .entries
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.get(transfer_id).map(|e| e.session.clone())
    }

    pub fn register_outbound(
        &self,
        transfer_id: String,
        peer_device_id: String,
        items: Vec<TransferItem>,
        total_bytes: u64,
    ) -> Result<Arc<AtomicBool>, KeynearError> {
        let clean_peer = sanitize_device_id(&peer_device_id)?;
        let mut guard = self
            .entries
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        ensure_capacity(&mut guard)?;
        let cancel_flag = Arc::new(AtomicBool::new(false));
        let session = TransferSession {
            transfer_id: transfer_id.clone(),
            peer_device_id: clean_peer,
            direction: TransferDirection::Send,
            items,
            total_bytes,
            transferred_bytes: 0,
            state: TransferState::Active,
            started_at: now_timestamp_string(),
        };
        guard.insert(
            transfer_id,
            TransferEntry {
                session,
                consent_tx: None,
                fd_tx: None,
                cancel_flag: Arc::clone(&cancel_flag),
            },
        );
        Ok(cancel_flag)
    }

    pub fn register_inbound(
        &self,
        peer_device_id: String,
        offer: &v1::TransferOffer,
        auto_accept: bool,
        require_explicit_fd: bool,
    ) -> Result<InboundRegistration, KeynearError> {
        let clean_peer = sanitize_device_id(&peer_device_id)?;
        validate_offer(offer)?;

        let items: Vec<TransferItem> = offer
            .items
            .iter()
            .map(|meta| TransferItem {
                name: meta.name.clone(),
                mime_type: meta.mime_type.clone(),
                size_bytes: meta.size_bytes,
                is_directory: meta.is_directory,
                target_path: None,
                checksum: None,
            })
            .collect();

        let mut guard = self
            .entries
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        ensure_capacity(&mut guard)?;
        if guard.contains_key(&offer.transfer_id) {
            return Err(KeynearError::Transfer(format!(
                "duplicate transfer_id '{}'",
                offer.transfer_id
            )));
        }

        let cancel_flag = Arc::new(AtomicBool::new(false));
        let (consent_tx, consent_rx) = if auto_accept {
            (None, None)
        } else {
            let (tx, rx) = oneshot::channel();
            (Some(tx), Some(rx))
        };
        let (fd_tx, fd_rx) = if require_explicit_fd {
            let (tx, rx) = oneshot::channel();
            (Some(tx), Some(rx))
        } else {
            (None, None)
        };

        let initial_state = if auto_accept {
            TransferState::Active
        } else {
            TransferState::PendingConsent
        };

        let session = TransferSession {
            transfer_id: offer.transfer_id.clone(),
            peer_device_id: clean_peer,
            direction: TransferDirection::Receive,
            items,
            total_bytes: offer.total_bytes,
            transferred_bytes: 0,
            state: initial_state,
            started_at: now_timestamp_string(),
        };

        guard.insert(
            offer.transfer_id.clone(),
            TransferEntry {
                session,
                consent_tx,
                fd_tx,
                cancel_flag: Arc::clone(&cancel_flag),
            },
        );

        Ok(InboundRegistration {
            state: initial_state,
            consent_rx,
            fd_rx,
            cancel_flag,
        })
    }

    pub fn respond_consent(
        &self,
        transfer_id: &str,
        accepted: bool,
    ) -> Result<TransferSession, KeynearError> {
        let mut guard = self
            .entries
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let entry = guard.get_mut(transfer_id).ok_or_else(|| {
            KeynearError::Transfer(format!("transfer '{transfer_id}' not found"))
        })?;
        if entry.session.state != TransferState::PendingConsent {
            return Err(KeynearError::Transfer(format!(
                "transfer '{transfer_id}' is not awaiting consent"
            )));
        }
        if accepted {
            entry.session.state = TransferState::Active;
        } else {
            entry.session.state = TransferState::Cancelled;
            entry.cancel_flag.store(true, Ordering::Release);
            entry.fd_tx.take();
        }
        if let Some(tx) = entry.consent_tx.take() {
            let _ = tx.send(accepted);
        }
        Ok(entry.session.clone())
    }

    pub fn provide_receive_fd(&self, transfer_id: &str, file: File) -> Result<(), KeynearError> {
        let mut guard = self
            .entries
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let entry = guard.get_mut(transfer_id).ok_or_else(|| {
            KeynearError::Transfer(format!("transfer '{transfer_id}' not found"))
        })?;
        if matches!(
            entry.session.state,
            TransferState::Cancelled | TransferState::Failed | TransferState::Completed
        ) {
            return Err(KeynearError::TransferCancelled(format!(
                "transfer '{transfer_id}' is already terminated"
            )));
        }
        let tx = entry.fd_tx.take().ok_or_else(|| {
            KeynearError::Transfer(format!(
                "transfer '{transfer_id}' is not awaiting an output file descriptor"
            ))
        })?;
        tx.send(file).map_err(|_| {
            KeynearError::Transfer(format!(
                "transfer '{transfer_id}' receiver dropped before accepting fd"
            ))
        })
    }

    pub fn cancel_transfer(&self, transfer_id: &str) -> Result<TransferSession, KeynearError> {
        let mut guard = self
            .entries
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let entry = guard.get_mut(transfer_id).ok_or_else(|| {
            KeynearError::Transfer(format!("transfer '{transfer_id}' not found"))
        })?;
        entry.cancel_flag.store(true, Ordering::Release);
        if matches!(
            entry.session.state,
            TransferState::PendingConsent | TransferState::Active
        ) {
            entry.session.state = TransferState::Cancelled;
        }
        if let Some(tx) = entry.consent_tx.take() {
            let _ = tx.send(false);
        }
        entry.fd_tx.take();
        Ok(entry.session.clone())
    }

    pub fn update_progress(&self, transfer_id: &str, transferred_bytes: u64) {
        let mut guard = self
            .entries
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(entry) = guard.get_mut(transfer_id) {
            entry.session.transferred_bytes = transferred_bytes;
        }
    }

    pub fn finish_transfer(
        &self,
        transfer_id: &str,
        state: TransferState,
        transferred_bytes: u64,
        target_paths: Option<&[String]>,
        checksum_hex: Option<&str>,
    ) {
        let mut guard = self
            .entries
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(entry) = guard.get_mut(transfer_id) {
            entry.session.state = state;
            entry.session.transferred_bytes = transferred_bytes;
            entry.consent_tx.take();
            entry.fd_tx.take();
            if let Some(paths) = target_paths {
                for (item, path) in entry.session.items.iter_mut().zip(paths.iter()) {
                    item.target_path = Some(path.clone());
                }
            }
            if let Some(digest) = checksum_hex {
                for item in &mut entry.session.items {
                    item.checksum = Some(digest.to_string());
                }
            }
        }
    }
}

pub struct InboundRegistration {
    pub state: TransferState,
    pub consent_rx: Option<oneshot::Receiver<bool>>,
    pub fd_rx: Option<oneshot::Receiver<File>>,
    pub cancel_flag: Arc<AtomicBool>,
}

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
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *guard = expand_tilde(&dir);
        Ok(())
    }

    pub fn incoming_directory(&self) -> PathBuf {
        self.incoming_directory
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    pub fn set_auto_accept_from_paired(&self, enabled: bool) {
        self.auto_accept_from_paired
            .store(enabled, Ordering::Release);
    }

    pub fn set_peer_auto_accept(&self, device_id: &str, enabled: bool) -> Result<(), KeynearError> {
        let clean_id = sanitize_device_id(device_id)?;
        let mut guard = self
            .peer_policies
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
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
            let mut guard = self
                .peer_policies
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
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
        let guard = self
            .peer_policies
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
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
                self.send_file_handle_stream(conn, &offer, file, &cancel_flag)
                    .await
            }
            OutboundPayload::ArchiveEntries(entries) => {
                self.send_zip_stream(conn, &offer, entries, &cancel_flag)
                    .await
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
        self.send_file_handle_on_connection(&conn, peer_device_id, file_name, mime_type, size_bytes, file)
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

        let res = self
            .send_file_handle_stream(conn, &offer, file, &cancel_flag)
            .await;
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
                self.registry
                    .get_session(transfer_id)
                    .ok_or_else(|| {
                        KeynearError::Transfer("completed transfer session missing".to_string())
                    })
            }
            Err(KeynearError::TransferCancelled(reason)) => {
                self.registry.finish_transfer(
                    transfer_id,
                    TransferState::Cancelled,
                    0,
                    None,
                    None,
                );
                Err(KeynearError::TransferCancelled(reason))
            }
            Err(err) => {
                self.registry
                    .finish_transfer(transfer_id, TransferState::Failed, 0, None, None);
                Err(err)
            }
        }
    }

    async fn send_file_handle_stream(
        &self,
        conn: &Connection,
        offer: &v1::TransferOffer,
        file: File,
        cancel_flag: &Arc<AtomicBool>,
    ) -> Result<String, KeynearError> {
        let (mut send, mut recv) = conn
            .open_bi()
            .await
            .map_err(|err| KeynearError::Transport(format!("open transfer stream failed: {err}")))?;

        write_stream_kind(&mut send, QuicStreamKind::RawFileChunk).await?;
        let offer_env = wrap_payload(v1::envelope::Payload::TransferOffer(offer.clone()));
        write_control_envelope(&mut send, &offer_env).await?;

        await_transfer_consent(&mut recv, &offer.transfer_id).await?;

        let (chunk_tx, mut chunk_rx) = mpsc::channel::<Result<Vec<u8>, KeynearError>>(PIPE_CHANNEL_SLOTS);
        let cancel_reader = Arc::clone(cancel_flag);
        let reader_task = tokio::task::spawn_blocking(move || {
            let mut reader = BufReader::with_capacity(ZIP_IO_BUFFER_BYTES, file);
            let mut buf = vec![0u8; STREAM_CHUNK_BYTES];
            loop {
                if cancel_reader.load(Ordering::Acquire) {
                    let _ = chunk_tx.blocking_send(Err(KeynearError::TransferCancelled(
                        "cancelled by sender".to_string(),
                    )));
                    break;
                }
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        if chunk_tx.blocking_send(Ok(buf[..n].to_vec())).is_err() {
                            break;
                        }
                    }
                    Err(err) => {
                        let _ = chunk_tx.blocking_send(Err(KeynearError::Io(err)));
                        break;
                    }
                }
            }
        });

        let mut hasher = Sha256::new();
        let mut bytes_sent: u64 = 0;
        while let Some(item) = chunk_rx.recv().await {
            if cancel_flag.load(Ordering::Acquire) {
                let _ = send_cancel_frame(&mut send, &offer.transfer_id, "cancelled_by_sender").await;
                return Err(KeynearError::TransferCancelled(
                    "cancelled by sender".to_string(),
                ));
            }
            let chunk = item?;
            hasher.update(&chunk);
            write_data_chunk(&mut send, &chunk).await?;
            bytes_sent = bytes_sent.saturating_add(chunk.len() as u64);
            self.registry.update_progress(&offer.transfer_id, bytes_sent);
        }

        reader_task
            .await
            .map_err(|err| KeynearError::Transfer(format!("file reader task failed: {err}")))?;

        let digest: [u8; 32] = hasher.finalize().into();
        send_completion_and_wait_ack(&mut send, &mut recv, &offer.transfer_id, bytes_sent, &digest)
            .await?;

        Ok(hex::encode(digest))
    }

    async fn send_zip_stream(
        &self,
        conn: &Connection,
        offer: &v1::TransferOffer,
        entries: Vec<ArchiveSourceEntry>,
        cancel_flag: &Arc<AtomicBool>,
    ) -> Result<String, KeynearError> {
        let (mut send, mut recv) = conn
            .open_bi()
            .await
            .map_err(|err| KeynearError::Transport(format!("open zip stream failed: {err}")))?;

        write_stream_kind(&mut send, QuicStreamKind::StoredZipArchive).await?;
        let offer_env = wrap_payload(v1::envelope::Payload::TransferOffer(offer.clone()));
        write_control_envelope(&mut send, &offer_env).await?;

        await_transfer_consent(&mut recv, &offer.transfer_id).await?;

        let (chunk_tx, mut chunk_rx) = mpsc::channel::<Result<Vec<u8>, KeynearError>>(PIPE_CHANNEL_SLOTS);
        let cancel_writer = Arc::clone(cancel_flag);
        let writer_task = tokio::task::spawn_blocking(move || -> Result<(), KeynearError> {
            let channel_writer = ChannelChunkWriter::new(chunk_tx.clone(), cancel_writer);
            let buffered = BufWriter::with_capacity(ZIP_IO_BUFFER_BYTES, channel_writer);
            let mut zip = ZipWriter::new_stream(buffered);
            let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);

            for entry in entries {
                if entry.is_dir {
                    let dir_name = if entry.archive_path.ends_with('/') {
                        entry.archive_path
                    } else {
                        format!("{}/", entry.archive_path)
                    };
                    zip.add_directory(dir_name, options).map_err(zip_err)?;
                    continue;
                }
                let Some(src_path) = entry.source_path else {
                    continue;
                };
                let meta = fs::symlink_metadata(&src_path)?;
                if meta.file_type().is_symlink() || !meta.is_file() {
                    continue;
                }
                zip.start_file(&entry.archive_path, options).map_err(zip_err)?;
                let file = File::open(&src_path)?;
                let mut reader = BufReader::with_capacity(ZIP_IO_BUFFER_BYTES, file);
                let mut copy_buf = vec![0u8; STREAM_CHUNK_BYTES];
                loop {
                    let n = reader.read(&mut copy_buf)?;
                    if n == 0 {
                        break;
                    }
                    zip.write_all(&copy_buf[..n])?;
                }
            }

            let stream_writer = zip.finish().map_err(zip_err)?;
            let mut buffered = stream_writer.into_inner();
            buffered.flush()?;
            Ok(())
        });

        let mut hasher = Sha256::new();
        let mut bytes_sent: u64 = 0;
        while let Some(item) = chunk_rx.recv().await {
            if cancel_flag.load(Ordering::Acquire) {
                let _ = send_cancel_frame(&mut send, &offer.transfer_id, "cancelled_by_sender").await;
                return Err(KeynearError::TransferCancelled(
                    "cancelled by sender".to_string(),
                ));
            }
            let chunk = item?;
            hasher.update(&chunk);
            write_data_chunk(&mut send, &chunk).await?;
            bytes_sent = bytes_sent.saturating_add(chunk.len() as u64);
            self.registry.update_progress(&offer.transfer_id, bytes_sent);
        }

        let writer_res = writer_task
            .await
            .map_err(|err| KeynearError::Transfer(format!("zip writer task failed: {err}")))?;
        writer_res?;

        let digest: [u8; 32] = hasher.finalize().into();
        send_completion_and_wait_ack(&mut send, &mut recv, &offer.transfer_id, bytes_sent, &digest)
            .await?;

        Ok(hex::encode(digest))
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
        mut send: SendStream,
        mut recv: RecvStream,
    ) -> Result<TransferSession, KeynearError> {
        let kind = read_stream_kind(&mut recv).await?;
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
            && self.require_explicit_fd_for_single_file.load(Ordering::Acquire);

        let reg = self.registry.register_inbound(
            peer_device_id.to_string(),
            &offer,
            auto_accept,
            require_fd,
        )?;

        if let Some(consent_rx) = reg.consent_rx {
            let accepted = match tokio::time::timeout(
                Duration::from_secs(CONSENT_TIMEOUT_SECS),
                consent_rx,
            )
            .await
            {
                Ok(Ok(true)) => true,
                Ok(Ok(false)) => false,
                Ok(Err(_)) | Err(_) => false,
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
                    let _ = send_consent_frame(&mut send, &offer.transfer_id, false, "no_output_fd")
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
                self.receive_raw_file_stream(
                    &offer,
                    &mut recv,
                    explicit_file,
                    &reg.cancel_flag,
                )
                .await
            }
            QuicStreamKind::StoredZipArchive => {
                self.receive_zip_stream(&offer, &mut recv, &reg.cancel_flag)
                    .await
            }
            QuicStreamKind::Control => unreachable!(),
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
                self.registry
                    .finish_transfer(&offer.transfer_id, TransferState::Failed, 0, None, None);
                Err(err)
            }
        }
    }

    async fn receive_raw_file_stream(
        &self,
        offer: &v1::TransferOffer,
        recv: &mut RecvStream,
        explicit_file: Option<File>,
        cancel_flag: &Arc<AtomicBool>,
    ) -> Result<(u64, Vec<String>, String), KeynearError> {
        let rel_name = sanitize_relative_path(&offer.items[0].name)?;
        let (dest_dir, tmp_path, final_path, out_file) = if let Some(file) = explicit_file {
            (None, None, None, file)
        } else {
            let dir = self.incoming_directory();
            fs::create_dir_all(&dir)?;
            let final_target = unique_target_path(&dir, &rel_name);
            let tmp = unique_temp_path(&dir, &offer.transfer_id);
            if let Some(parent) = tmp.parent() {
                fs::create_dir_all(parent)?;
            }
            let file = open_private_new_file(&tmp)?;
            (Some(dir), Some(tmp), Some(final_target), file)
        };

        let (chunk_tx, mut chunk_rx) = mpsc::channel::<Vec<u8>>(PIPE_CHANNEL_SLOTS);
        let writer_task = tokio::task::spawn_blocking(move || -> Result<(), KeynearError> {
            let mut writer = BufWriter::with_capacity(ZIP_IO_BUFFER_BYTES, out_file);
            while let Some(chunk) = chunk_rx.blocking_recv() {
                writer.write_all(&chunk)?;
            }
            writer.flush()?;
            let inner = writer
                .into_inner()
                .map_err(|err| KeynearError::Io(err.into_error()))?;
            let _ = inner.sync_all();
            Ok(())
        });

        let stream_result: Result<(u64, [u8; 32]), KeynearError> = async {
            let mut hasher = Sha256::new();
            let mut total_received: u64 = 0;

            while let Some(chunk) = read_data_chunk(recv).await? {
                if cancel_flag.load(Ordering::Acquire) {
                    return Err(KeynearError::TransferCancelled(
                        "cancelled by receiver".to_string(),
                    ));
                }
                let next_total = total_received
                    .checked_add(chunk.len() as u64)
                    .ok_or_else(|| KeynearError::Transfer("byte count overflow".to_string()))?;
                if next_total > offer.total_bytes {
                    return Err(KeynearError::Transfer(format!(
                        "received bytes {next_total} exceed TransferOffer.total_bytes {}",
                        offer.total_bytes
                    )));
                }
                hasher.update(&chunk);
                if chunk_tx.send(chunk).await.is_err() {
                    return Err(KeynearError::Transfer(
                        "disk writer task terminated unexpectedly".to_string(),
                    ));
                }
                total_received = next_total;
                self.registry.update_progress(&offer.transfer_id, total_received);
            }

            drop(chunk_tx);
            let write_res = writer_task
                .await
                .map_err(|err| KeynearError::Transfer(format!("disk writer join failed: {err}")))?;
            write_res?;

            let digest: [u8; 32] = hasher.finalize().into();
            verify_completion_envelope(recv, &offer.transfer_id, total_received, &digest).await?;
            Ok((total_received, digest))
        }
        .await;

        match stream_result {
            Ok((total_received, digest)) => {
                let target_paths = if let (Some(tmp), Some(final_target)) = (tmp_path, final_path) {
                    if let Some(parent) = final_target.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    fs::rename(&tmp, &final_target)?;
                    vec![final_target.to_string_lossy().to_string()]
                } else {
                    vec!["mediastore://fd".to_string()]
                };
                let _ = dest_dir;
                Ok((total_received, target_paths, hex::encode(digest)))
            }
            Err(err) => {
                if let Some(tmp) = tmp_path {
                    let _ = fs::remove_file(tmp);
                }
                Err(err)
            }
        }
    }

    async fn receive_zip_stream(
        &self,
        offer: &v1::TransferOffer,
        recv: &mut RecvStream,
        cancel_flag: &Arc<AtomicBool>,
    ) -> Result<(u64, Vec<String>, String), KeynearError> {
        let dir = self.incoming_directory();
        fs::create_dir_all(&dir)?;
        let staging_dir = dir.join(format!(
            ".keynear-staging-{}-{}",
            offer.transfer_id,
            TEMP_SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&staging_dir)?;

        let archive_tmp_path = staging_dir.join(".archive.zip.tmp");
        let max_wire_bytes = offer
            .total_bytes
            .saturating_add(MAX_ZIP_STREAM_OVERHEAD_BYTES);
        let max_extracted = offer.total_bytes;
        let staging_clone = staging_dir.clone();
        let archive_path_clone = archive_tmp_path.clone();

        let stream_result: Result<(u64, [u8; 32], Vec<PathBuf>), KeynearError> = async {
            let mut archive_writer = BufWriter::with_capacity(
                ZIP_IO_BUFFER_BYTES,
                open_private_new_file(&archive_tmp_path)?,
            );
            let mut hasher = Sha256::new();
            let mut wire_bytes: u64 = 0;

            while let Some(chunk) = read_data_chunk(recv).await? {
                if cancel_flag.load(Ordering::Acquire) {
                    return Err(KeynearError::TransferCancelled(
                        "cancelled by receiver".to_string(),
                    ));
                }
                wire_bytes = wire_bytes
                    .checked_add(chunk.len() as u64)
                    .ok_or_else(|| KeynearError::Transfer("wire byte overflow".to_string()))?;
                if wire_bytes > max_wire_bytes {
                    return Err(KeynearError::Transfer(
                        "zip stream exceeded permitted framing overhead".to_string(),
                    ));
                }
                hasher.update(&chunk);
                archive_writer.write_all(&chunk)?;
                self.registry
                    .update_progress(&offer.transfer_id, wire_bytes.min(offer.total_bytes));
            }
            archive_writer.flush()?;
            drop(archive_writer);

            let digest: [u8; 32] = hasher.finalize().into();
            verify_completion_envelope(recv, &offer.transfer_id, wire_bytes, &digest).await?;

            let extracted_paths =
                tokio::task::spawn_blocking(move || -> Result<Vec<PathBuf>, KeynearError> {
                    let archive_file = File::open(&archive_path_clone)?;
                    let reader = BufReader::with_capacity(ZIP_IO_BUFFER_BYTES, archive_file);
                    let mut archive = ZipArchive::new(reader).map_err(zip_err)?;
                    if archive.len() > MAX_FILES_IN_ARCHIVE {
                        return Err(KeynearError::Transfer(
                            "archive exceeds maximum file entry count".to_string(),
                        ));
                    }

                    let mut extracted_bytes: u64 = 0;
                    let mut extracted_rel_paths = Vec::new();
                    let mut buf = vec![0u8; STREAM_CHUNK_BYTES];

                    for idx in 0..archive.len() {
                        let mut entry = archive.by_index(idx).map_err(zip_err)?;
                        if entry.encrypted() {
                            return Err(KeynearError::Transfer(
                                "encrypted zip entries are not allowed".to_string(),
                            ));
                        }
                        if entry.is_symlink() {
                            return Err(KeynearError::Transfer(
                                "symlink zip entries are not allowed".to_string(),
                            ));
                        }
                        if entry.compression() != CompressionMethod::Stored {
                            return Err(KeynearError::Transfer(format!(
                                "unsupported zip compression method {:?}; only STORED is permitted",
                                entry.compression()
                            )));
                        }
                        let Some(enclosed) = entry.enclosed_name() else {
                            return Err(KeynearError::Transfer(
                                "zip entry contains invalid or escaping path".to_string(),
                            ));
                        };
                        let safe_rel = sanitize_relative_path(&enclosed.to_string_lossy())?;
                        if entry.is_dir() {
                            fs::create_dir_all(staging_clone.join(&safe_rel))?;
                            continue;
                        }
                        if entry.compressed_size() != entry.size() {
                            return Err(KeynearError::Transfer(
                                "STORED zip entry has mismatched compressed and uncompressed size"
                                    .to_string(),
                            ));
                        }
                        let target = staging_clone.join(&safe_rel);
                        if let Some(parent) = target.parent() {
                            fs::create_dir_all(parent)?;
                        }
                        let out_file = open_private_new_file(&target)?;
                        let mut writer = BufWriter::with_capacity(ZIP_IO_BUFFER_BYTES, out_file);
                        loop {
                            let n = entry.read(&mut buf)?;
                            if n == 0 {
                                break;
                            }
                            extracted_bytes = extracted_bytes
                                .checked_add(n as u64)
                                .ok_or_else(|| {
                                    KeynearError::Transfer("extracted byte overflow".to_string())
                                })?;
                            if extracted_bytes > max_extracted {
                                return Err(KeynearError::Transfer(format!(
                                    "extracted bytes {extracted_bytes} exceed TransferOffer.total_bytes {max_extracted}"
                                )));
                            }
                            writer.write_all(&buf[..n])?;
                        }
                        writer.flush()?;
                        extracted_rel_paths.push(safe_rel);
                    }

                    Ok(extracted_rel_paths)
                })
                .await
                .map_err(|err| KeynearError::Transfer(format!("zip extract task failed: {err}")))??;

            let _ = fs::remove_file(&archive_tmp_path);
            Ok((wire_bytes, digest, extracted_paths))
        }
        .await;

        match stream_result {
            Ok((wire_bytes, digest, extracted_paths)) => {
                let mut promoted = Vec::with_capacity(extracted_paths.len());
                for rel in extracted_paths {
                    let src = staging_dir.join(&rel);
                    let dest = unique_target_path(&dir, &rel);
                    if let Some(parent) = dest.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    fs::rename(&src, &dest)?;
                    promoted.push(dest.to_string_lossy().to_string());
                }
                let _ = fs::remove_dir_all(&staging_dir);
                Ok((wire_bytes, promoted, hex::encode(digest)))
            }
            Err(err) => {
                let _ = fs::remove_dir_all(&staging_dir);
                Err(err)
            }
        }
    }
}

struct SendPlan {
    items: Vec<TransferItem>,
    total_bytes: u64,
    payload: OutboundPayload,
}

enum OutboundPayload {
    SingleFile(PathBuf),
    ArchiveEntries(Vec<ArchiveSourceEntry>),
}

struct ArchiveSourceEntry {
    archive_path: String,
    source_path: Option<PathBuf>,
    is_dir: bool,
}

fn prepare_send_plan(paths: &[PathBuf]) -> Result<SendPlan, KeynearError> {
    if paths.len() == 1 {
        let path = &paths[0];
        let meta = fs::symlink_metadata(path)?;
        if meta.file_type().is_symlink() {
            return Err(KeynearError::Transfer(
                "symlinks cannot be transferred".to_string(),
            ));
        }
        if meta.is_file() {
            let file_name = path
                .file_name()
                .and_then(|s| s.to_str())
                .ok_or_else(|| KeynearError::Validation {
                    field: "path",
                    message: "source file has invalid filename".to_string(),
                })?;
            let clean_name = sanitize_relative_path(file_name)?;
            let size = meta.len();
            let item = TransferItem {
                name: clean_name.to_string_lossy().to_string(),
                mime_type: "application/octet-stream".to_string(),
                size_bytes: size,
                is_directory: false,
                target_path: None,
                checksum: None,
            };
            return Ok(SendPlan {
                items: vec![item],
                total_bytes: size,
                payload: OutboundPayload::SingleFile(path.clone()),
            });
        }
    }

    let mut entries = Vec::new();
    let mut items = Vec::new();
    let mut total_bytes: u64 = 0;

    for path in paths {
        let meta = fs::symlink_metadata(path)?;
        if meta.file_type().is_symlink() {
            continue;
        }
        let base_name = path
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| KeynearError::Validation {
                field: "path",
                message: "source path has invalid filename".to_string(),
            })?;
        let base_rel = sanitize_relative_path(base_name)?;

        if meta.is_dir() {
            let mut dir_bytes: u64 = 0;
            collect_directory_entries(path, &base_rel, &mut entries, &mut dir_bytes)?;
            total_bytes = total_bytes
                .checked_add(dir_bytes)
                .ok_or_else(|| KeynearError::Transfer("transfer size overflow".to_string()))?;
            items.push(TransferItem {
                name: base_rel.to_string_lossy().to_string(),
                mime_type: "application/zip".to_string(),
                size_bytes: dir_bytes,
                is_directory: true,
                target_path: None,
                checksum: None,
            });
        } else if meta.is_file() {
            let size = meta.len();
            total_bytes = total_bytes
                .checked_add(size)
                .ok_or_else(|| KeynearError::Transfer("transfer size overflow".to_string()))?;
            let rel_str = base_rel.to_string_lossy().replace('\\', "/");
            entries.push(ArchiveSourceEntry {
                archive_path: rel_str.clone(),
                source_path: Some(path.clone()),
                is_dir: false,
            });
            items.push(TransferItem {
                name: rel_str,
                mime_type: "application/octet-stream".to_string(),
                size_bytes: size,
                is_directory: false,
                target_path: None,
                checksum: None,
            });
        }
    }

    if items.is_empty() {
        return Err(KeynearError::Validation {
            field: "paths",
            message: "no transferable regular files or directories found".to_string(),
        });
    }

    Ok(SendPlan {
        items,
        total_bytes,
        payload: OutboundPayload::ArchiveEntries(entries),
    })
}

fn collect_directory_entries(
    fs_dir: &Path,
    archive_rel: &Path,
    entries: &mut Vec<ArchiveSourceEntry>,
    dir_bytes: &mut u64,
) -> Result<(), KeynearError> {
    let rel_str = archive_rel.to_string_lossy().replace('\\', "/");
    entries.push(ArchiveSourceEntry {
        archive_path: rel_str,
        source_path: None,
        is_dir: true,
    });

    let mut children: Vec<fs::DirEntry> = fs::read_dir(fs_dir)?.collect::<Result<Vec<_>, _>>()?;
    children.sort_by_key(|e| e.file_name());

    for child in children {
        let child_path = child.path();
        let meta = fs::symlink_metadata(&child_path)?;
        if meta.file_type().is_symlink() {
            continue;
        }
        let child_name = child.file_name();
        let Some(name_str) = child_name.to_str() else {
            continue;
        };
        let clean_child = sanitize_relative_path(name_str)?;
        let next_rel = archive_rel.join(clean_child);
        if meta.is_dir() {
            collect_directory_entries(&child_path, &next_rel, entries, dir_bytes)?;
        } else if meta.is_file() {
            *dir_bytes = dir_bytes
                .checked_add(meta.len())
                .ok_or_else(|| KeynearError::Transfer("directory size overflow".to_string()))?;
            entries.push(ArchiveSourceEntry {
                archive_path: next_rel.to_string_lossy().replace('\\', "/"),
                source_path: Some(child_path),
                is_dir: false,
            });
        }
    }
    Ok(())
}

pub fn sanitize_relative_path(raw: &str) -> Result<PathBuf, KeynearError> {
    if raw.is_empty() || raw.contains('\0') || raw.contains('\\') {
        return Err(KeynearError::Validation {
            field: "path",
            message: "relative path contains forbidden characters".to_string(),
        });
    }
    if raw.starts_with('/') || raw.as_bytes().get(1) == Some(&b':') {
        return Err(KeynearError::Validation {
            field: "path",
            message: "absolute path is forbidden".to_string(),
        });
    }
    let mut out = PathBuf::new();
    for component in Path::new(raw).components() {
        match component {
            Component::Normal(seg) => {
                if seg.is_empty() || seg == "." || seg == ".." {
                    return Err(KeynearError::Validation {
                        field: "path",
                        message: "path traversal segment is forbidden".to_string(),
                    });
                }
                out.push(seg);
            }
            Component::CurDir
            | Component::ParentDir
            | Component::RootDir
            | Component::Prefix(_) => {
                return Err(KeynearError::Validation {
                    field: "path",
                    message: "path traversal or root component is forbidden".to_string(),
                });
            }
        }
    }
    if out.as_os_str().is_empty() {
        return Err(KeynearError::Validation {
            field: "path",
            message: "relative path must not be empty".to_string(),
        });
    }
    Ok(out)
}

fn validate_offer(offer: &v1::TransferOffer) -> Result<(), KeynearError> {
    if offer.transfer_id.trim().is_empty() || offer.transfer_id.len() > 64 {
        return Err(KeynearError::Validation {
            field: "transferId",
            message: "transfer_id must be 1..64 bytes".to_string(),
        });
    }
    if offer.items.is_empty() {
        return Err(KeynearError::Validation {
            field: "items",
            message: "transfer offer must contain at least one item".to_string(),
        });
    }
    let mut sum: u64 = 0;
    for item in &offer.items {
        sanitize_relative_path(&item.name)?;
        sum = sum
            .checked_add(item.size_bytes)
            .ok_or_else(|| KeynearError::Transfer("transfer offer size overflow".to_string()))?;
    }
    if sum != offer.total_bytes {
        return Err(KeynearError::Validation {
            field: "totalBytes",
            message: "offer total_bytes does not match item sum".to_string(),
        });
    }
    Ok(())
}

fn ensure_capacity(entries: &mut HashMap<String, TransferEntry>) -> Result<(), KeynearError> {
    let active_count = entries
        .values()
        .filter(|e| {
            matches!(
                e.session.state,
                TransferState::PendingConsent | TransferState::Active
            )
        })
        .count();
    if active_count >= MAX_ACTIVE_TRANSFERS {
        return Err(KeynearError::Transfer(
            "maximum active transfers limit reached".to_string(),
        ));
    }
    if entries.len() >= MAX_HISTORY_TRANSFERS {
        let removable: Vec<String> = entries
            .iter()
            .filter(|(_, e)| {
                !matches!(
                    e.session.state,
                    TransferState::PendingConsent | TransferState::Active
                )
            })
            .map(|(k, _)| k.clone())
            .take(entries.len() - MAX_HISTORY_TRANSFERS + 1)
            .collect();
        for key in removable {
            entries.remove(&key);
        }
    }
    Ok(())
}

async fn write_data_chunk(send: &mut SendStream, chunk: &[u8]) -> Result<(), KeynearError> {
    let len = u32::try_from(chunk.len()).map_err(|_| {
        KeynearError::Transfer("data chunk exceeds u32 length".to_string())
    })?;
    if len == 0 || (len as usize) > STREAM_CHUNK_BYTES {
        return Err(KeynearError::Transfer(
            "data chunk length out of bounds".to_string(),
        ));
    }
    send.write_all(&len.to_be_bytes())
        .await
        .map_err(|err| KeynearError::Transport(format!("write chunk header failed: {err}")))?;
    send.write_all(chunk)
        .await
        .map_err(|err| KeynearError::Transport(format!("write chunk body failed: {err}")))?;
    Ok(())
}

async fn read_data_chunk(recv: &mut RecvStream) -> Result<Option<Vec<u8>>, KeynearError> {
    let mut len_buf = [0u8; 4];
    recv.read_exact(&mut len_buf)
        .await
        .map_err(|err| KeynearError::Transport(format!("read chunk header failed: {err}")))?;
    let len = u32::from_be_bytes(len_buf) as usize;
    if len == 0 {
        return Ok(None);
    }
    if len > STREAM_CHUNK_BYTES {
        return Err(KeynearError::Transfer(format!(
            "incoming chunk length {len} exceeds maximum {STREAM_CHUNK_BYTES}"
        )));
    }
    let mut buf = vec![0u8; len];
    recv.read_exact(&mut buf)
        .await
        .map_err(|err| KeynearError::Transport(format!("read chunk body failed: {err}")))?;
    Ok(Some(buf))
}

async fn send_completion_and_wait_ack(
    send: &mut SendStream,
    recv: &mut RecvStream,
    transfer_id: &str,
    bytes_transferred: u64,
    digest: &[u8; 32],
) -> Result<(), KeynearError> {
    send.write_all(&0u32.to_be_bytes())
        .await
        .map_err(|err| KeynearError::Transport(format!("write EOF marker failed: {err}")))?;
    let complete = wrap_payload(v1::envelope::Payload::TransferComplete(
        v1::TransferComplete {
            transfer_id: transfer_id.to_string(),
            bytes_transferred,
            sha256_digest: digest.to_vec(),
        },
    ));
    write_control_envelope(send, &complete).await?;
    send.finish()
        .map_err(|err| KeynearError::Transport(format!("finish send stream failed: {err}")))?;

    let reply = read_control_envelope(recv).await?;
    match reply.payload {
        Some(v1::envelope::Payload::ControlAck(ack)) if ack.accepted => Ok(()),
        Some(v1::envelope::Payload::ControlAck(ack)) => Err(KeynearError::Transfer(ack.reason)),
        Some(v1::envelope::Payload::TransferCancel(cancel)) => {
            Err(KeynearError::TransferCancelled(cancel.reason))
        }
        _ => Err(KeynearError::Protocol(
            "expected ControlAck after TransferComplete".to_string(),
        )),
    }
}

async fn verify_completion_envelope(
    recv: &mut RecvStream,
    transfer_id: &str,
    expected_bytes: u64,
    expected_digest: &[u8; 32],
) -> Result<(), KeynearError> {
    let env = read_control_envelope(recv).await?;
    let Some(v1::envelope::Payload::TransferComplete(complete)) = env.payload else {
        return Err(KeynearError::Protocol(
            "expected TransferComplete envelope at end of stream".to_string(),
        ));
    };
    if complete.transfer_id != transfer_id {
        return Err(KeynearError::Protocol(
            "TransferComplete transfer_id mismatch".to_string(),
        ));
    }
    if complete.bytes_transferred != expected_bytes {
        return Err(KeynearError::Transfer(format!(
            "transferred byte count {} does not match sender {}",
            expected_bytes, complete.bytes_transferred
        )));
    }
    if complete.sha256_digest.len() != 32
        || !bool::from(complete.sha256_digest.ct_eq(expected_digest))
    {
        return Err(KeynearError::Crypto(
            "transfer SHA-256 verification failed".to_string(),
        ));
    }
    Ok(())
}

async fn await_transfer_consent(
    recv: &mut RecvStream,
    transfer_id: &str,
) -> Result<(), KeynearError> {
    let env = read_control_envelope(recv).await?;
    match env.payload {
        Some(v1::envelope::Payload::TransferConsent(consent)) => {
            if consent.transfer_id != transfer_id {
                return Err(KeynearError::Protocol(
                    "TransferConsent transfer_id mismatch".to_string(),
                ));
            }
            if consent.accepted {
                Ok(())
            } else {
                Err(KeynearError::TransferCancelled(consent.reason))
            }
        }
        Some(v1::envelope::Payload::TransferCancel(cancel)) => {
            Err(KeynearError::TransferCancelled(cancel.reason))
        }
        _ => Err(KeynearError::Protocol(
            "expected TransferConsent response".to_string(),
        )),
    }
}

async fn send_consent_frame(
    send: &mut SendStream,
    transfer_id: &str,
    accepted: bool,
    reason: &str,
) -> Result<(), KeynearError> {
    let env = wrap_payload(v1::envelope::Payload::TransferConsent(
        v1::TransferConsent {
            transfer_id: transfer_id.to_string(),
            accepted,
            reason: reason.to_string(),
        },
    ));
    write_control_envelope(send, &env).await
}

async fn send_cancel_frame(
    send: &mut SendStream,
    transfer_id: &str,
    reason: &str,
) -> Result<(), KeynearError> {
    let env = wrap_payload(v1::envelope::Payload::TransferCancel(v1::TransferCancel {
        transfer_id: transfer_id.to_string(),
        reason: reason.to_string(),
    }));
    write_control_envelope(send, &env).await
}

fn wrap_payload(payload: v1::envelope::Payload) -> v1::Envelope {
    v1::Envelope {
        version: 1,
        sequence: 1,
        timestamp_ms: now_millis(),
        payload: Some(payload),
    }
}

struct ChannelChunkWriter {
    tx: mpsc::Sender<Result<Vec<u8>, KeynearError>>,
    cancel_flag: Arc<AtomicBool>,
}

impl ChannelChunkWriter {
    fn new(
        tx: mpsc::Sender<Result<Vec<u8>, KeynearError>>,
        cancel_flag: Arc<AtomicBool>,
    ) -> Self {
        Self { tx, cancel_flag }
    }
}

impl Write for ChannelChunkWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if self.cancel_flag.load(Ordering::Acquire) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Interrupted,
                "transfer cancelled",
            ));
        }
        if buf.is_empty() {
            return Ok(0);
        }
        for slice in buf.chunks(STREAM_CHUNK_BYTES) {
            self.tx
                .blocking_send(Ok(slice.to_vec()))
                .map_err(|_| std::io::Error::new(std::io::ErrorKind::BrokenPipe, "receiver closed"))?;
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn expand_tilde(path: &Path) -> PathBuf {
    let raw = path.to_string_lossy();
    if let Some(rest) = raw.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join(rest);
        }
    }
    path.to_path_buf()
}

fn unique_target_path(base_dir: &Path, rel: &Path) -> PathBuf {
    let candidate = base_dir.join(rel);
    if !candidate.exists() {
        return candidate;
    }
    let stem = candidate
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("file");
    let ext = candidate.extension().and_then(|s| s.to_str());
    let parent = candidate.parent().unwrap_or(base_dir);
    for idx in 1..10_000 {
        let name = match ext {
            Some(e) => format!("{stem} ({idx}).{e}"),
            None => format!("{stem} ({idx})"),
        };
        let next = parent.join(name);
        if !next.exists() {
            return next;
        }
    }
    parent.join(format!("{stem}-{}", Uuid::new_v4()))
}

fn unique_temp_path(base_dir: &Path, transfer_id: &str) -> PathBuf {
    let seq = TEMP_SEQ.fetch_add(1, Ordering::Relaxed);
    base_dir.join(format!(
        ".keynear-part-{transfer_id}-{}-{seq}.tmp",
        std::process::id()
    ))
}

fn open_private_new_file(path: &Path) -> std::io::Result<File> {
    let mut opts = OpenOptions::new();
    opts.create_new(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    opts.open(path)
}

fn zip_err(err: zip::result::ZipError) -> KeynearError {
    KeynearError::Transfer(format!("zip stream error: {err}"))
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn now_timestamp_string() -> String {
    format!("{}", now_millis())
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::quic::OrionNodeConfig;
    use std::io::Cursor;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use zeroize::Zeroizing;

    fn temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!("keynear-transfer-{label}-{nanos}"))
    }

    fn loopback_addr() -> SocketAddr {
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0)
    }

    async fn connected_pair() -> (OrionQuicNode, OrionQuicNode, Connection, Connection) {
        let server = OrionQuicNode::bind(
            OrionNodeConfig {
                device_id: "server-1".to_string(),
                display_name: "Server".to_string(),
                capability_flags: 1,
                quic_secret_key: Zeroizing::new([0x61; 32]),
            },
            Some(loopback_addr()),
        )
        .await
        .expect("bind server");
        let client = OrionQuicNode::bind(
            OrionNodeConfig {
                device_id: "client-1".to_string(),
                display_name: "Client".to_string(),
                capability_flags: 1,
                quic_secret_key: Zeroizing::new([0x62; 32]),
            },
            Some(loopback_addr()),
        )
        .await
        .expect("bind client");

        let secret = [0xAB; 32];
        let server_addrs: Vec<SocketAddr> = server.endpoint_addr().ip_addrs().copied().collect();
        let client_addrs: Vec<SocketAddr> = client.endpoint_addr().ip_addrs().copied().collect();

        server
            .register_peer("client-1", client.endpoint_id(), client_addrs, secret)
            .expect("register client");
        client
            .register_peer("server-1", server.endpoint_id(), server_addrs, secret)
            .expect("register server");

        let server_clone = server.clone();
        let accept_task = tokio::spawn(async move { server_clone.accept_authenticated().await });
        let client_conn = client.connect_peer("server-1").await.expect("connect");
        let (_, server_conn) = accept_task
            .await
            .expect("join")
            .expect("accept")
            .expect("conn");

        (server, client, server_conn, client_conn)
    }

    #[tokio::test]
    async fn streams_60kb_chunks_and_stored_zip_directory_skipping_symlinks() {
        let dir = temp_dir("stream-ok");
        let src_dir = dir.join("src");
        let dest_dir = dir.join("dest");
        fs::create_dir_all(src_dir.join("nested")).expect("create src");
        fs::create_dir_all(&dest_dir).expect("create dest");

        // > 2 chunks (150 KiB) single file test.
        let single_file = dir.join("payload.bin");
        let single_bytes: Vec<u8> = (0..(STREAM_CHUNK_BYTES * 2 + 12_345))
            .map(|i| (i % 251) as u8)
            .collect();
        fs::write(&single_file, &single_bytes).expect("write single file");

        // Directory with nested file and a symlink that must be skipped.
        let nested_file = src_dir.join("nested").join("hello.txt");
        fs::write(&nested_file, b"orion stored zip content").expect("write nested file");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&single_file, src_dir.join("escape-link"))
            .expect("create symlink");

        let (server, client, server_conn, client_conn) = connected_pair().await;
        let receiver_engine = Arc::new(TransferEngine::new(dest_dir.clone(), true));
        let sender_engine = Arc::new(TransferEngine::new(dir.clone(), false));

        let recv_clone = Arc::clone(&receiver_engine);
        let recv_task = tokio::spawn(async move {
            let (s1, r1) = server_conn.accept_bi().await.expect("accept file stream");
            let session1 = recv_clone
                .handle_incoming_stream("client-1", s1, r1)
                .await
                .expect("receive single file");
            let (s2, r2) = server_conn.accept_bi().await.expect("accept zip stream");
            let session2 = recv_clone
                .handle_incoming_stream("client-1", s2, r2)
                .await
                .expect("receive zip dir");
            (session1, session2)
        });

        let send_file_session = sender_engine
            .send_paths_on_connection(&client_conn, "server-1", &[single_file])
            .await
            .expect("send single file");
        assert_eq!(send_file_session.state, TransferState::Completed);
        assert_eq!(send_file_session.transferred_bytes, single_bytes.len() as u64);

        let send_dir_session = sender_engine
            .send_paths_on_connection(&client_conn, "server-1", std::slice::from_ref(&src_dir))
            .await
            .expect("send directory");
        assert_eq!(send_dir_session.state, TransferState::Completed);

        let (recv_s1, recv_s2) = recv_task.await.expect("recv join");
        assert_eq!(recv_s1.state, TransferState::Completed);
        assert_eq!(recv_s2.state, TransferState::Completed);
        assert_eq!(
            fs::read(dest_dir.join("payload.bin")).expect("read received file"),
            single_bytes
        );
        assert_eq!(
            fs::read(dest_dir.join("src").join("nested").join("hello.txt"))
                .expect("read received nested"),
            b"orion stored zip content"
        );
        assert!(!dest_dir.join("src").join("escape-link").exists());

        client.close().await;
        server.close().await;
        let _ = fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn pending_consent_blocks_writes_unless_auto_accept() {
        let dir = temp_dir("consent");
        let src_file = dir.join("secret.txt");
        let dest_dir = dir.join("dest");
        fs::create_dir_all(&dir).expect("create dir");
        fs::create_dir_all(&dest_dir).expect("create dest");
        fs::write(&src_file, b"consent-gated payload").expect("write src");

        let (server, client, server_conn, client_conn) = connected_pair().await;
        let receiver_engine = Arc::new(TransferEngine::new(dest_dir.clone(), false));
        receiver_engine
            .set_peer_auto_accept("client-1", false)
            .expect("set peer auto accept false");
        let sender_engine = Arc::new(TransferEngine::new(dir.clone(), false));

        // 1. Declined offer writes nothing to disk.
        let recv_clone = Arc::clone(&receiver_engine);
        let server_conn_1 = server_conn.clone();
        let recv_task = tokio::spawn(async move {
            let (s, r) = server_conn_1.accept_bi().await.expect("accept stream");
            recv_clone.handle_incoming_stream("client-1", s, r).await
        });

        let send_clone = Arc::clone(&sender_engine);
        let client_conn_1 = client_conn.clone();
        let src_1 = src_file.clone();
        let send_task = tokio::spawn(async move {
            send_clone
                .send_paths_on_connection(&client_conn_1, "server-1", &[src_1])
                .await
        });

        let pending_id = loop {
            let list = receiver_engine.list_transfers();
            if let Some(s) = list.into_iter().find(|s| s.state == TransferState::PendingConsent) {
                break s.transfer_id;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        };
        assert!(!dest_dir.join("secret.txt").exists());
        let declined = receiver_engine
            .respond_consent(&pending_id, false)
            .expect("decline");
        assert_eq!(declined.state, TransferState::Cancelled);
        assert!(matches!(
            recv_task.await.expect("join"),
            Err(KeynearError::TransferCancelled(_))
        ));
        assert!(matches!(
            send_task.await.expect("join"),
            Err(KeynearError::TransferCancelled(_))
        ));
        assert!(!dest_dir.join("secret.txt").exists());

        // 2. Enabling per-device auto_accept_files allows immediate completion.
        receiver_engine
            .set_peer_auto_accept("client-1", true)
            .expect("enable peer auto accept");
        let recv_clone_2 = Arc::clone(&receiver_engine);
        let recv_task_2 = tokio::spawn(async move {
            let (s, r) = server_conn.accept_bi().await.expect("accept stream");
            recv_clone_2.handle_incoming_stream("client-1", s, r).await
        });
        let sent = sender_engine
            .send_paths_on_connection(&client_conn, "server-1", &[src_file])
            .await
            .expect("auto-accepted send completes");
        assert_eq!(sent.state, TransferState::Completed);
        let recvd = recv_task_2.await.expect("join").expect("recv completes");
        assert_eq!(recvd.state, TransferState::Completed);
        assert_eq!(
            fs::read(dest_dir.join("secret.txt")).expect("read file"),
            b"consent-gated payload"
        );

        client.close().await;
        server.close().await;
        let _ = fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn rejects_zip_slip_symlink_and_byte_overflow() {
        assert!(sanitize_relative_path("../escape.txt").is_err());
        assert!(sanitize_relative_path("/etc/passwd").is_err());
        assert!(sanitize_relative_path("a\\b.txt").is_err());
        assert!(sanitize_relative_path("").is_err());
        assert_eq!(
            sanitize_relative_path("folder/item.txt").unwrap(),
            PathBuf::from("folder/item.txt")
        );

        let dir = temp_dir("security-reject");
        let dest_dir = dir.join("dest");
        fs::create_dir_all(&dest_dir).expect("create dest");

        // Build malicious STORED zip containing zip-slip path in raw local header.
        let mut zip_bytes = Vec::new();
        {
            let mut zw = ZipWriter::new(Cursor::new(&mut zip_bytes));
            let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
            zw.start_file("../evil.txt", opts).expect("start evil");
            zw.write_all(b"evil").expect("write evil");
            zw.finish().expect("finish");
        }

        let (server, client, server_conn, client_conn) = connected_pair().await;
        let receiver_engine = Arc::new(TransferEngine::new(dest_dir.clone(), true));

        let recv_clone = Arc::clone(&receiver_engine);
        let server_conn_1 = server_conn.clone();
        let recv_task = tokio::spawn(async move {
            let (s, r) = server_conn_1.accept_bi().await.expect("accept");
            recv_clone.handle_incoming_stream("client-1", s, r).await
        });

        // Send offer claiming valid item name, then inject zip-slip entry in ZIP stream.
        let (mut send, mut recv) = client_conn.open_bi().await.expect("open bi");
        write_stream_kind(&mut send, QuicStreamKind::StoredZipArchive)
            .await
            .unwrap();
        let offer = v1::TransferOffer {
            transfer_id: "tx-zipslip".to_string(),
            items: vec![v1::TransferItemMeta {
                name: "safe.txt".to_string(),
                mime_type: "text/plain".to_string(),
                size_bytes: 4,
                is_directory: true,
            }],
            total_bytes: 4,
        };
        write_control_envelope(&mut send, &wrap_payload(v1::envelope::Payload::TransferOffer(offer)))
            .await
            .unwrap();
        await_transfer_consent(&mut recv, "tx-zipslip")
            .await
            .unwrap();
        write_data_chunk(&mut send, &zip_bytes).await.unwrap();
        let mut hasher = Sha256::new();
        hasher.update(&zip_bytes);
        let digest: [u8; 32] = hasher.finalize().into();
        let _ = send_completion_and_wait_ack(&mut send, &mut recv, "tx-zipslip", zip_bytes.len() as u64, &digest).await;

        let res = recv_task.await.expect("join");
        assert!(matches!(res, Err(KeynearError::Transfer(_))));
        assert!(!dir.join("evil.txt").exists());

        // Test byte overflow rejection: offer claims 4 bytes, sender streams 100 bytes.
        let recv_clone_2 = Arc::clone(&receiver_engine);
        let recv_task_2 = tokio::spawn(async move {
            let (s, r) = server_conn.accept_bi().await.expect("accept 2");
            recv_clone_2.handle_incoming_stream("client-1", s, r).await
        });
        let (mut send2, mut recv2) = client_conn.open_bi().await.expect("open bi 2");
        write_stream_kind(&mut send2, QuicStreamKind::RawFileChunk)
            .await
            .unwrap();
        let offer2 = v1::TransferOffer {
            transfer_id: "tx-overflow".to_string(),
            items: vec![v1::TransferItemMeta {
                name: "bomb.bin".to_string(),
                mime_type: "application/octet-stream".to_string(),
                size_bytes: 4,
                is_directory: false,
            }],
            total_bytes: 4,
        };
        write_control_envelope(&mut send2, &wrap_payload(v1::envelope::Payload::TransferOffer(offer2)))
            .await
            .unwrap();
        await_transfer_consent(&mut recv2, "tx-overflow")
            .await
            .unwrap();
        let _ = write_data_chunk(&mut send2, &[0xAA; 100]).await;
        let _ = send2.finish();

        let res2 = recv_task_2.await.expect("join 2");
        assert!(matches!(res2, Err(KeynearError::Transfer(_))));
        assert!(!dest_dir.join("bomb.bin").exists());

        client.close().await;
        server.close().await;
        let _ = fs::remove_dir_all(&dir);
    }
}

