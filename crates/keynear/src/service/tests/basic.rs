use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use keynear_core::{InMemorySecretStore, PairedDeviceStore};
use keynear_protocol::{DaemonConfig, KeynearError, VisibilityMode};
use tokio::sync::oneshot;

use super::*;

#[tokio::test]
async fn shutdown_loop_completes_once() {
    let dir = temp_service_dir("shutdown");
    let config = DaemonConfig {
        incoming_directory: dir.join("incoming").display().to_string(),
        ..Default::default()
    };
    let (tx, rx) = oneshot::channel::<()>();
    let completions = Arc::new(AtomicUsize::new(0));
    let completions_clone = Arc::clone(&completions);
    let options = RuntimeOptions {
        enable_bluez: false,
        enable_mdns: false,
        enable_ipc: false,
        enable_tray: false,
        enable_notifications: false,
        enable_clipboard: false,
        quic_bind_addr: Some("127.0.0.1:0".parse().expect("loopback")),
        ..Default::default()
    };

    let runtime_dir = dir.clone();
    let task = tokio::spawn(async move {
        let result = DaemonRuntime::run_with_deps(
            &config,
            options,
            RuntimeDeps {
                device_store: PairedDeviceStore::new(runtime_dir.join("devices.toml")),
                secret_store: Some(Arc::new(InMemorySecretStore::new())),
            },
            async move {
                let _ = rx.await;
            },
        )
        .await;
        completions_clone.fetch_add(1, Ordering::SeqCst);
        result
    });

    tx.send(()).expect("send shutdown signal");
    task.await
        .expect("join runtime task")
        .expect("runtime exits cleanly");
    assert_eq!(completions.load(Ordering::SeqCst), 1);
    let _ = fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn invalid_config_fails_before_awaiting_shutdown() {
    let config = DaemonConfig {
        incoming_directory: String::new(),
        ..Default::default()
    };
    let err = DaemonRuntime::run_with_shutdown_with_config_path(
        &config,
        None,
        std::future::pending::<()>(),
    )
    .await
    .expect_err("invalid config must fail immediately");
    assert!(matches!(
        err,
        KeynearError::Validation {
            field: "incomingDirectory",
            ..
        }
    ));
}

#[tokio::test]
async fn degrades_gracefully_when_bluetooth_and_keyring_absent() {
    let dir = temp_service_dir("degraded");
    let config = DaemonConfig {
        visibility_mode: VisibilityMode::EveryoneNearby,
        incoming_directory: dir.join("incoming").display().to_string(),
        ..Default::default()
    };
    let (tx, rx) = oneshot::channel::<()>();
    let options = RuntimeOptions {
        everyone_nearby_timeout: Duration::from_millis(25),
        enable_bluez: true,
        enable_mdns: false,
        enable_ipc: false,
        enable_tray: false,
        enable_notifications: false,
        enable_clipboard: false,
        ..Default::default()
    };

    let task = spawn_test_daemon(TestDaemonParams {
        config,
        options,
        store: PairedDeviceStore::new(dir.join("devices.toml")),
        secrets: None,
        rx,
    });

    tokio::time::sleep(Duration::from_millis(45)).await;
    tx.send(()).expect("shutdown signal");
    task.await
        .expect("join task")
        .expect("daemon degrades cleanly without bluetooth or keyring");
    let _ = fs::remove_dir_all(&dir);
}
