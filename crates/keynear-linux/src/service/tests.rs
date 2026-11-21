mod basic;
mod ipc_control;
mod ipc_transfer;
mod otp;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use keynear_core::{InMemorySecretStore, PairedDeviceStore};
use keynear_ipc::{send_request_to, DaemonStatus, IpcRequest, IpcResponse};
use keynear_protocol::{DaemonConfig, KeynearError, TransferState};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

use super::*;

fn temp_service_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("keynear-svc-{label}-{nanos}"));
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

struct TestDaemonParams {
    config: DaemonConfig,
    options: RuntimeOptions,
    store: PairedDeviceStore,
    secrets: Option<Arc<InMemorySecretStore>>,
    rx: oneshot::Receiver<()>,
}

fn to_secret_store(
    secrets: Option<Arc<InMemorySecretStore>>,
) -> Option<Arc<dyn keynear_core::SecretStore>> {
    secrets.map(|s| s as Arc<dyn keynear_core::SecretStore>)
}

fn spawn_test_daemon(params: TestDaemonParams) -> JoinHandle<Result<(), KeynearError>> {
    let TestDaemonParams {
        config,
        options,
        store,
        secrets,
        rx,
    } = params;
    tokio::spawn(async move {
        DaemonRuntime::run_with_deps(
            &config,
            options,
            RuntimeDeps {
                device_store: store,
                secret_store: to_secret_store(secrets),
            },
            async move {
                let _ = rx.await;
            },
        )
        .await
    })
}

async fn wait_for_status(sock_path: &Path) -> DaemonStatus {
    wait_for_status_with(sock_path, |_| true, "daemon status").await
}

async fn wait_for_paired_devices(sock_path: &Path, min_count: u32) -> DaemonStatus {
    let description = format!("paired_device_count >= {min_count}");
    wait_for_status_with(
        sock_path,
        |status| status.paired_device_count >= min_count,
        &description,
    )
    .await
}

async fn wait_for_status_with<F>(
    sock_path: &Path,
    mut predicate: F,
    description: &str,
) -> DaemonStatus
where
    F: FnMut(&DaemonStatus) -> bool,
{
    let mut last_err = None;
    for _ in 0..80 {
        let p = sock_path.to_path_buf();
        match tokio::task::spawn_blocking(move || send_request_to(&p, &IpcRequest::Status)).await {
            Ok(Ok(IpcResponse::Status(status))) if predicate(&status) => return status,
            Ok(Ok(IpcResponse::Status(_))) => {}
            Ok(Ok(other)) => last_err = Some(format!("unexpected IPC response: {other:?}")),
            Ok(Err(err)) => last_err = Some(err.to_string()),
            Err(err) => last_err = Some(err.to_string()),
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!(
        "timed out waiting for {description} on {} (last error: {last_err:?})",
        sock_path.display()
    );
}

fn assert_send_finished(
    result: &Result<Result<IpcResponse, KeynearError>, tokio::task::JoinError>,
    phase: &str,
) -> ! {
    panic!("send task finished {phase}: {result:?}");
}

fn find_pending_id(
    result: Result<Result<IpcResponse, KeynearError>, tokio::task::JoinError>,
) -> Option<String> {
    let Ok(Ok(IpcResponse::Transfers(list))) = result else {
        return None;
    };
    list.into_iter()
        .find(|s| s.state == TransferState::PendingConsent)
        .map(|s| s.transfer_id)
}

type PendingSend = tokio::task::JoinHandle<Result<IpcResponse, KeynearError>>;
type ListTask = tokio::task::JoinHandle<Result<IpcResponse, KeynearError>>;

async fn select_list_result(send_handle: &mut PendingSend, list_fut: ListTask) -> Option<String> {
    tokio::select! {
        send_res = &mut *send_handle => assert_send_finished(&send_res, "before PendingConsent appeared"),
        list_res = list_fut => find_pending_id(list_res),
    }
}

async fn select_sleep_or_send(send_handle: &mut PendingSend) {
    tokio::select! {
        send_res = &mut *send_handle => assert_send_finished(&send_res, "while waiting for PendingConsent"),
        _ = tokio::time::sleep(Duration::from_millis(25)) => {}
    }
}

async fn wait_for_pending_transfer(
    sock_path: &Path,
    send_handle: &mut tokio::task::JoinHandle<Result<IpcResponse, KeynearError>>,
) -> String {
    for _ in 0..80 {
        let p = sock_path.to_path_buf();
        let list_fut =
            tokio::task::spawn_blocking(move || send_request_to(&p, &IpcRequest::ListTransfers));
        if let Some(id) = select_list_result(send_handle, list_fut).await {
            return id;
        }
        select_sleep_or_send(send_handle).await;
    }
    panic!("timed out waiting for pending consent transfer");
}

fn ipc_runtime_options(device_id: &str, display_name: &str, sock: PathBuf) -> RuntimeOptions {
    RuntimeOptions {
        local_device_id: device_id.to_string(),
        display_name: display_name.to_string(),
        enable_bluez: false,
        enable_mdns: false,
        enable_ipc: true,
        enable_tray: false,
        enable_notifications: false,
        enable_clipboard: false,
        ipc_socket_path: Some(sock),
        quic_bind_addr: Some("127.0.0.1:0".parse().expect("addr")),
        ..Default::default()
    }
}
