use std::sync::Arc;

use keynear_core::{OrionQuicNode, PairedDeviceStore};
use keynear_protocol::{
    decode_envelope, proto::validate_control_envelope, v1, write_control_envelope,
    write_stream_kind, KeynearError, QuicStreamKind,
};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::warn;

use super::audio_control::{enqueue_audio_handoff, AudioHandoffEvent};
use crate::ipc::handlers::OutboundEnvelope;

pub(crate) struct OutboundDrainParams {
    pub rx: mpsc::Receiver<OutboundEnvelope>,
    pub node: Arc<OrionQuicNode>,
    pub device_store: PairedDeviceStore,
    pub audio_tx: mpsc::Sender<AudioHandoffEvent>,
}

pub(crate) fn spawn_outbound_drain(params: OutboundDrainParams, tasks: &mut Vec<JoinHandle<()>>) {
    tasks.push(tokio::spawn(run_drain(params)));
}

fn log_outbound_rejected(kind: &'static str, err: &KeynearError) {
    warn!(event = "continuity_outbound_frame_rejected", kind, error = %err);
}

fn log_peer_load_failed(err: &KeynearError) {
    warn!(event = "continuity_peer_load_failed", error = %err);
}

fn log_outbound_send_failed(peer_device_id: &str, err: &KeynearError) {
    warn!(
        event = "continuity_outbound_send_failed",
        peer_device_id = %peer_device_id,
        error = %err
    );
}

async fn run_drain(params: OutboundDrainParams) {
    let OutboundDrainParams {
        mut rx,
        node,
        device_store,
        audio_tx,
    } = params;
    while let Some(outbound) = rx.recv().await {
        match decode_outbound(&outbound.bytes) {
            Ok(envelope) => {
                let _ = enqueue_audio_handoff(&audio_tx, &envelope);
                send_to_peers(&node, &device_store, &envelope).await;
            }
            Err(err) => log_outbound_rejected(outbound.label, &err),
        }
    }
}

fn decode_outbound(bytes: &[u8]) -> Result<v1::Envelope, KeynearError> {
    let envelope = decode_envelope(bytes)?;
    validate_control_envelope(&envelope)?;
    Ok(envelope)
}

async fn send_to_peers(node: &OrionQuicNode, store: &PairedDeviceStore, envelope: &v1::Envelope) {
    let peers = match store.load() {
        Ok(peers) => peers,
        Err(err) => {
            log_peer_load_failed(&err);
            return;
        }
    };
    for peer in peers {
        if let Err(err) = send_one(node, &peer.device_id, envelope).await {
            log_outbound_send_failed(&peer.device_id, &err);
        }
    }
}

async fn send_one(
    node: &OrionQuicNode,
    peer_device_id: &str,
    envelope: &v1::Envelope,
) -> Result<(), KeynearError> {
    let conn = node.connect_peer(peer_device_id).await?;
    let (mut send, _recv) = conn
        .open_bi()
        .await
        .map_err(|err| KeynearError::Transport(format!("open control stream failed: {err}")))?;
    write_stream_kind(&mut send, QuicStreamKind::Control).await?;
    write_control_envelope(&mut send, envelope).await?;
    send.finish()
        .map_err(|err| KeynearError::Transport(format!("finish control stream failed: {err}")))
}
