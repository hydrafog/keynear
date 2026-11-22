use std::sync::Arc;

use keynear_core::{DaemonConfig, PairedDeviceStore, SecretStore};
use keynear_protocol::KeynearError;

use super::services::{bind_quic_node, init_clipboard, setup_mdns, RuntimeServices};
use super::{sync_initial_peers, RuntimeOptions, RuntimeState, SyncPeersParams};

pub(super) struct InitParams<'a> {
    pub config: &'a DaemonConfig,
    pub options: &'a RuntimeOptions,
    pub state: &'a RuntimeState,
    pub device_store: &'a PairedDeviceStore,
    pub secret_store: Option<&'a Arc<dyn SecretStore>>,
}

pub(super) async fn init_services(params: InitParams<'_>) -> Result<RuntimeServices, KeynearError> {
    let (
        transfer_engine,
        incoming_clip_rx,
        clipboard_engine,
        clip_change_rx,
        clipboard_handle,
        latest_otp,
        known_peers,
    ) = init_clipboard(params.config, params.options, params.state);
    let quic_node = bind_quic_node(params.options, params.secret_store).await?;
    sync_initial_peers(SyncPeersParams {
        device_store: params.device_store,
        secret_store: params.secret_store,
        quic_node: &quic_node,
        transfer_engine: &transfer_engine,
        known_peers: &known_peers,
    })
    .await;
    let mdns = setup_mdns(params.options, quic_node.as_ref());

    Ok(RuntimeServices {
        device_store: params.device_store.clone(),
        secret_store: params.secret_store.cloned(),
        transfer_engine,
        incoming_clip_rx: Some(incoming_clip_rx),
        clipboard_engine,
        clip_change_rx: Some(clip_change_rx),
        clipboard_handle,
        latest_otp,
        known_peers,
        quic_node,
        mdns,
    })
}
