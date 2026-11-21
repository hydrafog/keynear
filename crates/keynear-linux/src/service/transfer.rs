use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use keynear_core::{
    read_stream_kind, ClipboardEngine, OrionQuicNode, PairedDeviceStore, QuicStreamKind,
    SecretStore, TransferEngine, TransferState,
};
use keynear_protocol::KeynearError;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tracing::warn;

use crate::ipc::IpcDispatchContext;
use crate::notifications::NotificationsClient;
use crate::service::pairing::sync_paired_devices_and_policies;

#[allow(clippy::too_many_arguments)]
pub(crate) fn spawn_quic_accept_loop(
    node: &Arc<OrionQuicNode>,
    device_store: &PairedDeviceStore,
    secret_store: Option<&Arc<dyn SecretStore>>,
    transfer_engine: &Arc<TransferEngine>,
    clipboard_engine: &Arc<ClipboardEngine>,
    known_peers: &Arc<Mutex<HashSet<String>>>,
    tasks: &mut Vec<JoinHandle<()>>,
) {
    tasks.push(tokio::spawn(run_quic_accept_loop(
        Arc::clone(node),
        device_store.clone(),
        secret_store.cloned(),
        Arc::clone(transfer_engine),
        Arc::clone(clipboard_engine),
        Arc::clone(known_peers),
    )));
}

fn warn_quic_accept_failed(err: &KeynearError) {
    warn!(event = "quic_accept_failed", error = %err);
}

fn warn_peer_sync_failed(err: &KeynearError) {
    warn!(event = "peer_sync_on_accept_failed", error = %err);
}

#[allow(clippy::too_many_arguments)]
async fn sync_on_accept(
    store: &PairedDeviceStore,
    secrets: Option<&dyn SecretStore>,
    node: &OrionQuicNode,
    engine: &TransferEngine,
    known: &Mutex<HashSet<String>>,
) {
    if let Err(err) =
        sync_paired_devices_and_policies(store, secrets, Some(node), engine, known).await
    {
        warn_peer_sync_failed(&err);
    }
}

#[allow(clippy::too_many_arguments)]
async fn run_quic_accept_loop(
    node: Arc<OrionQuicNode>,
    store: PairedDeviceStore,
    secrets: Option<Arc<dyn SecretStore>>,
    engine: Arc<TransferEngine>,
    clipboard: Arc<ClipboardEngine>,
    known: Arc<Mutex<HashSet<String>>>,
) {
    loop {
        let (peer_device_id, conn) = match node.accept_authenticated().await {
            Ok(Some(accepted)) => accepted,
            Ok(None) => break,
            Err(err) => {
                warn_quic_accept_failed(&err);
                continue;
            }
        };
        sync_on_accept(&store, secrets.as_deref(), &node, &engine, &known).await;
        let conn_engine = Arc::clone(&engine);
        let conn_clipboard = Arc::clone(&clipboard);
        tokio::spawn(async move {
            while let Ok((send, mut recv)) = conn.accept_bi().await {
                let Ok(kind) = read_stream_kind(&mut recv).await else {
                    break;
                };
                let peer = peer_device_id.clone();
                if matches!(
                    kind,
                    QuicStreamKind::RawFileChunk | QuicStreamKind::StoredZipArchive
                ) {
                    let eng = Arc::clone(&conn_engine);
                    tokio::spawn(async move {
                        let _ = eng
                            .handle_incoming_stream_with_kind(&peer, kind, send, recv)
                            .await;
                    });
                } else {
                    let clip = Arc::clone(&conn_clipboard);
                    tokio::spawn(async move {
                        let _ = clip
                            .handle_incoming_stream_with_kind(&peer, kind, send, recv)
                            .await;
                    });
                }
            }
        });
    }
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
