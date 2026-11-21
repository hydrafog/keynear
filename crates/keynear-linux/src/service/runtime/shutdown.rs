use std::path::PathBuf;
use std::sync::Arc;

use crate::ipc::IpcDispatchContext;
use keynear_core::{MdnsDiscovery, OrionQuicNode};
use keynear_protocol::{DaemonConfig, VisibilityMode};
use tokio::sync::watch;
use tokio::task::JoinHandle;

use crate::clipboard::ClipboardMonitorHandle;

pub(super) struct ShutdownParams {
    pub clipboard_handle: ClipboardMonitorHandle,
    pub tray_handle: Option<ksni::Handle<crate::tray::KeynearTray>>,
    pub shared_ctx: IpcDispatchContext,
    pub config_watch_tx: watch::Sender<DaemonConfig>,
    pub lan_watch_tx: watch::Sender<Option<keynear_protocol::v1::LanEndpoint>>,
    pub visibility_tx: watch::Sender<VisibilityMode>,
    pub tasks: Vec<JoinHandle<()>>,
    pub mdns: Option<Arc<MdnsDiscovery>>,
    pub quic_node: Option<Arc<OrionQuicNode>>,
    pub bound_socket_path: Option<PathBuf>,
}

pub(super) async fn shutdown_all(params: ShutdownParams) {
    let ShutdownParams {
        clipboard_handle,
        tray_handle,
        shared_ctx,
        config_watch_tx,
        lan_watch_tx,
        visibility_tx,
        tasks,
        mdns,
        quic_node,
        bound_socket_path,
    } = params;

    clipboard_handle.shutdown();
    if let Some(handle) = tray_handle {
        handle.shutdown().await;
    }
    drop(shared_ctx);
    drop(config_watch_tx);
    drop(lan_watch_tx);
    drop(visibility_tx);
    for handle in tasks {
        handle.abort();
        let _ = handle.await;
    }
    if let Some(discovery) = mdns {
        let _ = tokio::task::spawn_blocking(move || discovery.shutdown()).await;
    }
    if let Some(node) = quic_node {
        node.close().await;
    }
    if let Some(sock_path) = bound_socket_path {
        let _ = std::fs::remove_file(&sock_path);
    }

    log_daemon_stopped();
}

fn log_daemon_stopped() {
    tracing::info!(event = "daemon_stopped");
}
