use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use keynear_core::{
    read_stream_kind, ClipboardEngine, OrionQuicNode, PairedDeviceStore, QuicStreamKind,
    SecretStore, TransferEngine, TransferState,
};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tracing::warn;

use crate::ipc::IpcDispatchContext;
use crate::notifications::NotificationsClient;
use crate::service::pairing::sync_paired_devices_and_policies;

pub(crate) fn spawn_quic_accept_loop(
    node: &Arc<OrionQuicNode>,
    device_store: &PairedDeviceStore,
    secret_store: Option<&Arc<dyn SecretStore>>,
    transfer_engine: &Arc<TransferEngine>,
    clipboard_engine: &Arc<ClipboardEngine>,
    known_peers: &Arc<Mutex<HashSet<String>>>,
    tasks: &mut Vec<JoinHandle<()>>,
) {
    let accept_node = Arc::clone(node);
    let accept_engine = Arc::clone(transfer_engine);
    let accept_clipboard = Arc::clone(clipboard_engine);
    let accept_store = device_store.clone();
    let accept_secrets = secret_store.cloned();
    let accept_known = Arc::clone(known_peers);
    tasks.push(tokio::spawn(async move {
        loop {
            match accept_node.accept_authenticated().await {
                Ok(Some((peer_device_id, conn))) => {
                    if let Err(err) = sync_paired_devices_and_policies(
                        &accept_store,
                        accept_secrets.as_deref(),
                        Some(accept_node.as_ref()),
                        &accept_engine,
                        &accept_known,
                    )
                    .await
                    {
                        warn!(event = "peer_sync_on_accept_failed", error = %err);
                    }
                    let conn_engine = Arc::clone(&accept_engine);
                    let conn_clipboard = Arc::clone(&accept_clipboard);
                    tokio::spawn(async move {
                        while let Ok((send, mut recv)) = conn.accept_bi().await {
                            match read_stream_kind(&mut recv).await {
                                Ok(
                                    kind @ (QuicStreamKind::RawFileChunk
                                    | QuicStreamKind::StoredZipArchive),
                                ) => {
                                    let eng = Arc::clone(&conn_engine);
                                    let peer = peer_device_id.clone();
                                    tokio::spawn(async move {
                                        let _ = eng
                                            .handle_incoming_stream_with_kind(
                                                &peer, kind, send, recv,
                                            )
                                            .await;
                                    });
                                }
                                Ok(
                                    kind
                                    @ (QuicStreamKind::Control | QuicStreamKind::ClipboardBlob),
                                ) => {
                                    let clip = Arc::clone(&conn_clipboard);
                                    let peer = peer_device_id.clone();
                                    tokio::spawn(async move {
                                        let _ = clip
                                            .handle_incoming_stream_with_kind(
                                                &peer, kind, send, recv,
                                            )
                                            .await;
                                    });
                                }
                                Err(_) => break,
                            }
                        }
                    });
                }
                Ok(None) => break,
                Err(err) => {
                    warn!(event = "quic_accept_failed", error = %err);
                }
            }
        }
    }));
}

pub(crate) async fn sync_transfer_notifications(
    client: &NotificationsClient,
    ctx: &IpcDispatchContext,
    sent: &mut HashMap<String, (u32, TransferState, u64)>,
) {
    let devices = ctx.device_store.load().unwrap_or_default();
    let cfg = ctx.config_state.lock().await.clone();
    let transfers = ctx.transfer_engine.list_transfers();
    for session in transfers {
        let peer_name = devices
            .iter()
            .find(|d| d.device_id == session.peer_device_id)
            .map(|d| d.display_name.as_str())
            .unwrap_or(session.peer_device_id.as_str());
        if session.state == TransferState::Completed && !cfg.notify_auto_accept_complete {
            continue;
        }
        let prev = sent.get(&session.transfer_id).copied();
        let should_send = match prev {
            None => matches!(
                session.state,
                TransferState::PendingConsent | TransferState::Active
            ),
            Some((_, prev_state, prev_bytes)) => {
                prev_state != session.state
                    || (session.state == TransferState::Active
                        && session.transferred_bytes != prev_bytes)
            }
        };
        if !should_send {
            continue;
        }
        let replaces_id = prev.map(|(nid, _, _)| nid).unwrap_or(0);
        if let Some(nid) = client
            .notify_transfer_session(&session, peer_name, replaces_id)
            .await
        {
            sent.insert(
                session.transfer_id,
                (nid, session.state, session.transferred_bytes),
            );
        }
    }
}
