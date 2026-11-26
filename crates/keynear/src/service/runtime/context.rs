use std::sync::Arc;

use tokio::sync::mpsc;

use crate::ipc::IpcDispatchContext;

use super::{RuntimeOptions, RuntimeServices, RuntimeState};

pub(crate) fn build_shared_ctx(
    options: &RuntimeOptions,
    state: &RuntimeState,
    services: &RuntimeServices,
    pairing_decision_tx: Option<mpsc::Sender<bool>>,
) -> IpcDispatchContext {
    IpcDispatchContext {
        local_device_id: options.local_device_id.clone(),
        visibility_tx: state.visibility_tx.clone(),
        visibility_rx: state.visibility_rx.clone(),
        config_state: Arc::clone(&state.config_state),
        config_watch_tx: state.config_watch_tx.clone(),
        config_path: state.config_path.clone(),
        everyone_nearby_timeout: options.everyone_nearby_timeout,
        everyone_deadline: Arc::clone(&state.everyone_deadline),
        deadline_notify: Arc::clone(&state.deadline_notify),
        refresh_notify: Arc::clone(&state.refresh_notify),
        device_store: services.device_store.clone(),
        secret_store: services.secret_store.clone(),
        quic_node: services.quic_node.clone(),
        transfer_engine: Arc::clone(&services.transfer_engine),
        clipboard_engine: Arc::clone(&services.clipboard_engine),
        clipboard_handle: services.clipboard_handle.clone(),
        latest_otp: Arc::clone(&services.latest_otp),
        known_peers: Arc::clone(&services.known_peers),
        mdns_degraded: services.mdns_degraded(),
        mdns: services.mdns.clone(),
        pairing_decision_tx,
        phoenix_supervisor: services.phoenix_supervisor.clone(),
    }
}
