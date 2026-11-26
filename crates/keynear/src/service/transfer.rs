use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use keynear_core::{
    read_stream_kind, ClipboardEngine, OrionQuicNode, PairedDeviceStore, ProximityConfig,
    QuicStreamKind, RangingSample, RangingTechnology, SecretStore, TransferEngine, TransferState,
    MAX_RANGE_METERS,
};
use keynear_protocol::{
    read_control_envelope, v1, write_control_envelope, DaemonConfig, KeynearError,
};
use tokio::io::AsyncWrite;
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;
use tracing::warn;

use crate::ipc::IpcDispatchContext;
use crate::notifications::NotificationsClient;
use crate::proximity::{Login1SessionLock, ProximitySupervisor};
use crate::service::pairing::sync_paired_devices_and_policies;

pub(crate) struct QuicAcceptParams<'a> {
    pub node: &'a Arc<OrionQuicNode>,
    pub device_store: &'a PairedDeviceStore,
    pub secret_store: Option<&'a Arc<dyn SecretStore>>,
    pub transfer_engine: &'a Arc<TransferEngine>,
    pub clipboard_engine: &'a Arc<ClipboardEngine>,
    pub known_peers: &'a Arc<Mutex<HashSet<String>>>,
    pub phoenix_supervisor: Option<&'a Arc<Mutex<ProximitySupervisor<Login1SessionLock>>>>,
    pub config_state: &'a Arc<Mutex<DaemonConfig>>,
    pub config_path: Option<&'a Path>,
    pub ranging_tx: &'a mpsc::Sender<RangingSample>,
}

#[derive(Clone)]
struct ControlDispatchCtx {
    clipboard: Arc<ClipboardEngine>,
    phoenix_supervisor: Option<Arc<Mutex<ProximitySupervisor<Login1SessionLock>>>>,
    config_state: Arc<Mutex<DaemonConfig>>,
    config_path: Option<PathBuf>,
    ranging_tx: mpsc::Sender<RangingSample>,
}

struct AcceptLoopState {
    node: Arc<OrionQuicNode>,
    store: PairedDeviceStore,
    secrets: Option<Arc<dyn SecretStore>>,
    engine: Arc<TransferEngine>,
    known: Arc<Mutex<HashSet<String>>>,
    control: ControlDispatchCtx,
}

pub(crate) fn spawn_quic_accept_loop(
    params: QuicAcceptParams<'_>,
    tasks: &mut Vec<JoinHandle<()>>,
) {
    let control = ControlDispatchCtx {
        clipboard: Arc::clone(params.clipboard_engine),
        phoenix_supervisor: params.phoenix_supervisor.cloned(),
        config_state: Arc::clone(params.config_state),
        config_path: params.config_path.map(Path::to_path_buf),
        ranging_tx: params.ranging_tx.clone(),
    };
    let state = AcceptLoopState {
        node: Arc::clone(params.node),
        store: params.device_store.clone(),
        secrets: params.secret_store.cloned(),
        engine: Arc::clone(params.transfer_engine),
        known: Arc::clone(params.known_peers),
        control,
    };
    tasks.push(tokio::spawn(run_quic_accept_loop(state)));
}

fn warn_quic_accept_failed(err: &KeynearError) {
    warn!(event = "quic_accept_failed", error = %err);
}

fn warn_peer_sync_failed(err: &KeynearError) {
    warn!(event = "peer_sync_on_accept_failed", error = %err);
}

async fn sync_on_accept(state: &AcceptLoopState) {
    if let Err(err) = sync_paired_devices_and_policies(
        &state.store,
        state.secrets.as_deref(),
        Some(&state.node),
        &state.engine,
        &state.known,
    )
    .await
    {
        warn_peer_sync_failed(&err);
    }
}

async fn send_control_ack<W: AsyncWrite + Unpin>(
    send: &mut W,
    acked_sequence: u64,
    accepted: bool,
    reason: &str,
) {
    let ack = v1::Envelope {
        version: 1,
        sequence: acked_sequence,
        timestamp_ms: 0,
        payload: Some(v1::envelope::Payload::ControlAck(v1::ControlAck {
            acked_sequence,
            accepted,
            reason: reason.to_string(),
        })),
    };
    let _ = write_control_envelope(send, &ack).await;
}

fn ranging_report_to_sample(peer_device_id: &str, rr: v1::RangingReport) -> Option<RangingSample> {
    let tech = match v1::RangingTechnology::try_from(rr.technology).ok()? {
        v1::RangingTechnology::Uwb => RangingTechnology::Uwb,
        v1::RangingTechnology::BleCs => RangingTechnology::BleChannelSounding,
        v1::RangingTechnology::BleRssi => RangingTechnology::BleRssi,
        v1::RangingTechnology::Unknown => return None,
    };
    let uses_distance = matches!(
        tech,
        RangingTechnology::Uwb | RangingTechnology::BleChannelSounding
    );
    let device_id = if rr.device_id.trim().is_empty() {
        peer_device_id.to_string()
    } else {
        rr.device_id
    };
    Some(RangingSample {
        device_id,
        technology: tech,
        distance_meters: uses_distance.then_some(rr.distance_meters),
        rssi_dbm: (!uses_distance).then_some(rr.rssi_dbm),
        sequence: rr.sequence,
    })
}

async fn apply_proximity_state(
    ctx: &ControlDispatchCtx,
    ps: &v1::ProximityState,
) -> Result<(), &'static str> {
    if !ps.distance_meters.is_finite() || !(0.5..=MAX_RANGE_METERS).contains(&ps.distance_meters) {
        return Err("invalid proximity distance");
    }
    let lock_enabled = ps.status != v1::ProximityStatus::ProximityUnspecified as i32;
    if let Some(ref supervisor) = ctx.phoenix_supervisor {
        let mut guard = supervisor.lock().await;
        let cfg = ProximityConfig::from_desk_distance(ps.distance_meters);
        guard
            .update_config(cfg, lock_enabled)
            .map_err(|_| "invalid proximity config")?;
    }
    let mut cfg = ctx.config_state.lock().await;
    cfg.proximity_lock_enabled = lock_enabled;
    cfg.proximity_distance_meters = ps.distance_meters;
    if let Some(ref path) = ctx.config_path {
        let _ = cfg.save_to_path(path);
    }
    Ok(())
}

async fn handle_proximity_or_ranging<W: AsyncWrite + Unpin>(
    ctrl: &ControlDispatchCtx,
    peer: &str,
    env: &v1::Envelope,
    send: &mut W,
) -> bool {
    match &env.payload {
        Some(v1::envelope::Payload::ProximityState(ps)) => {
            match apply_proximity_state(ctrl, ps).await {
                Ok(()) => send_control_ack(send, env.sequence, true, "").await,
                Err(reason) => send_control_ack(send, env.sequence, false, reason).await,
            }
            true
        }
        Some(v1::envelope::Payload::RangingReport(rr)) => {
            if let Some(sample) = ranging_report_to_sample(peer, rr.clone()) {
                let _ = ctrl.ranging_tx.try_send(sample);
                send_control_ack(send, env.sequence, true, "").await;
            } else {
                send_control_ack(send, env.sequence, false, "invalid ranging report").await;
            }
            true
        }
        _ => false,
    }
}

async fn run_quic_accept_loop(state: AcceptLoopState) {
    loop {
        let (peer_device_id, conn) = match state.node.accept_authenticated().await {
            Ok(Some(accepted)) => accepted,
            Ok(None) => break,
            Err(err) => {
                warn_quic_accept_failed(&err);
                continue;
            }
        };
        sync_on_accept(&state).await;
        let (eng, ctrl) = (Arc::clone(&state.engine), state.control.clone());
        tokio::spawn(async move {
            while let Ok((mut send, mut recv)) = conn.accept_bi().await {
                let Ok(kind) = read_stream_kind(&mut recv).await else {
                    break;
                };
                let (peer, eng, ctrl) = (peer_device_id.clone(), Arc::clone(&eng), ctrl.clone());
                tokio::spawn(async move {
                    match kind {
                        QuicStreamKind::RawFileChunk | QuicStreamKind::StoredZipArchive => {
                            let _ = eng
                                .handle_incoming_stream_with_kind(&peer, kind, send, recv)
                                .await;
                        }
                        QuicStreamKind::Control => {
                            if let Ok(env) = read_control_envelope(&mut recv).await {
                                if handle_proximity_or_ranging(&ctrl, &peer, &env, &mut send).await
                                {
                                    let _ = send.finish();
                                } else {
                                    let _ = ctrl
                                        .clipboard
                                        .handle_control_envelope(&peer, env, &mut send)
                                        .await;
                                }
                            }
                        }
                        _ => {
                            let _ = ctrl
                                .clipboard
                                .handle_incoming_stream_with_kind(&peer, kind, send, recv)
                                .await;
                        }
                    }
                });
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
