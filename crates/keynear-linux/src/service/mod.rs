pub mod clipboard;
pub mod pairing;
pub mod runtime;
pub mod transfer;

#[allow(unused_imports)]
pub use pairing::{apply_discovered_lan_endpoint, sync_paired_devices_and_policies};
pub use runtime::DaemonRuntime;
#[allow(unused_imports)]
pub use runtime::RuntimeOptions;

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use keynear_core::{
        commit_paired_device, ClipboardEngine, ClipboardItem, ClipboardPolicy, InMemorySecretStore,
        PairedDeviceStore, PeerSecrets, TransferEngine, TransferState,
    };
    use keynear_ipc::{send_request_to, DaemonStatus, IpcRequest, IpcResponse};
    use keynear_protocol::{DaemonConfig, DeviceRole, KeynearError, PairedDevice, VisibilityMode};
    use tokio::sync::{oneshot, watch, Mutex, Notify};

    use super::*;
    use crate::ble::DEFAULT_CAPABILITY_FLAGS;
    use crate::clipboard::ClipboardMonitorHandle;
    use crate::ipc::{
        build_daemon_status, build_tray_state, dispatch_ipc_request, handle_tray_command,
        IpcDispatchContext,
    };
    use crate::service::clipboard::apply_incoming_clipboard_item;
    use crate::tray::TrayCommand;

    fn temp_service_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("keynear-svc-{label}-{nanos}"));
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

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
                PairedDeviceStore::new(runtime_dir.join("devices.toml")),
                Some(Arc::new(InMemorySecretStore::new())),
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

        let runtime_dir = dir.clone();
        let task = tokio::spawn(async move {
            DaemonRuntime::run_with_deps(
                &config,
                options,
                PairedDeviceStore::new(runtime_dir.join("devices.toml")),
                None,
                async move {
                    let _ = rx.await;
                },
            )
            .await
        });

        tokio::time::sleep(Duration::from_millis(45)).await;
        tx.send(()).expect("shutdown signal");
        task.await
            .expect("join task")
            .expect("daemon degrades cleanly without bluetooth or keyring");
        let _ = fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn daemon_ipc_handles_status_send_list_consent_and_reload() {
        let dir = temp_service_dir("ipc-transfer");
        let alice_dir = dir.join("alice");
        let bob_dir = dir.join("bob");
        let alice_incoming = alice_dir.join("incoming");
        let bob_incoming = bob_dir.join("incoming");
        fs::create_dir_all(&alice_dir).expect("alice dir");
        fs::create_dir_all(&bob_incoming).expect("bob incoming");

        let alice_store = PairedDeviceStore::new(alice_dir.join("devices.toml"));
        let bob_store = PairedDeviceStore::new(bob_dir.join("devices.toml"));
        let alice_secrets = Arc::new(InMemorySecretStore::new());
        let bob_secrets = Arc::new(InMemorySecretStore::new());
        let shared_secret = [0x44u8; 32];

        let alice_sock = alice_dir.join("daemon.sock");
        let bob_sock = bob_dir.join("daemon.sock");

        let bob_config = DaemonConfig {
            incoming_directory: bob_incoming.display().to_string(),
            ..Default::default()
        };
        let alice_config = DaemonConfig {
            incoming_directory: alice_incoming.display().to_string(),
            ..Default::default()
        };

        let (bob_tx, bob_rx) = oneshot::channel::<()>();
        let bob_task = tokio::spawn({
            let bob_store = bob_store.clone();
            let bob_secrets = Arc::clone(&bob_secrets);
            let bob_sock = bob_sock.clone();
            async move {
                DaemonRuntime::run_with_deps(
                    &bob_config,
                    RuntimeOptions {
                        local_device_id: "bob-linux".to_string(),
                        display_name: "Bob".to_string(),
                        enable_bluez: false,
                        enable_mdns: false,
                        enable_ipc: true,
                        enable_tray: false,
                        enable_notifications: false,
                        enable_clipboard: false,
                        ipc_socket_path: Some(bob_sock),
                        quic_bind_addr: Some("127.0.0.1:0".parse().expect("addr")),
                        ..Default::default()
                    },
                    bob_store,
                    Some(bob_secrets),
                    async move {
                        let _ = bob_rx.await;
                    },
                )
                .await
            }
        });

        let (alice_tx, alice_rx) = oneshot::channel::<()>();
        let alice_task = tokio::spawn({
            let alice_store = alice_store.clone();
            let alice_secrets = Arc::clone(&alice_secrets);
            let alice_sock = alice_sock.clone();
            async move {
                DaemonRuntime::run_with_deps(
                    &alice_config,
                    RuntimeOptions {
                        local_device_id: "alice-linux".to_string(),
                        display_name: "Alice".to_string(),
                        enable_bluez: false,
                        enable_mdns: false,
                        enable_ipc: true,
                        enable_tray: false,
                        enable_notifications: false,
                        enable_clipboard: false,
                        ipc_socket_path: Some(alice_sock),
                        quic_bind_addr: Some("127.0.0.1:0".parse().expect("addr")),
                        ..Default::default()
                    },
                    alice_store,
                    Some(alice_secrets),
                    async move {
                        let _ = alice_rx.await;
                    },
                )
                .await
            }
        });

        let bob_status = wait_for_status(&bob_sock).await;
        let alice_status = wait_for_status(&alice_sock).await;
        assert_eq!(bob_status.device_id, "bob-linux");
        assert!(!bob_status.endpoint_id.is_empty());
        assert!(!bob_status.listen_addrs.is_empty());

        commit_paired_device(
            &alice_store,
            alice_secrets.as_ref(),
            PairedDevice {
                device_id: "bob-linux".to_string(),
                display_name: "Bob".to_string(),
                device_role: DeviceRole::Desktop,
                static_x25519_pubkey: "bb".repeat(32),
                last_ble_address: String::new(),
                last_lan_address: bob_status.listen_addrs.join(","),
                last_endpoint_id: bob_status.endpoint_id.clone(),
                capability_flags: DEFAULT_CAPABILITY_FLAGS,
                auto_accept_files: false,
                paired_at: "1".to_string(),
            },
            &PeerSecrets {
                irk: shared_secret,
                reconnect_secret: shared_secret,
            },
        )
        .expect("commit bob on alice");

        commit_paired_device(
            &bob_store,
            bob_secrets.as_ref(),
            PairedDevice {
                device_id: "alice-linux".to_string(),
                display_name: "Alice".to_string(),
                device_role: DeviceRole::Desktop,
                static_x25519_pubkey: "aa".repeat(32),
                last_ble_address: String::new(),
                last_lan_address: alice_status.listen_addrs.join(","),
                last_endpoint_id: alice_status.endpoint_id.clone(),
                capability_flags: DEFAULT_CAPABILITY_FLAGS,
                auto_accept_files: false,
                paired_at: "1".to_string(),
            },
            &PeerSecrets {
                irk: shared_secret,
                reconnect_secret: shared_secret,
            },
        )
        .expect("commit alice on bob");

        wait_for_paired_devices(&bob_sock, 1).await;
        wait_for_paired_devices(&alice_sock, 1).await;

        let sample_file = dir.join("note.txt");
        let payload = b"orion daemon ipc transfer payload";
        fs::write(&sample_file, payload).expect("write sample file");

        let send_sock = alice_sock.clone();
        let send_path = sample_file.clone();
        let mut send_handle = tokio::task::spawn_blocking(move || {
            send_request_to(
                &send_sock,
                &IpcRequest::SendFiles {
                    device_id: "bob-linux".to_string(),
                    paths: vec![send_path],
                },
            )
        });

        let pending_id = wait_for_pending_transfer(&bob_sock, &mut send_handle).await;
        let bob_sock_accept = bob_sock.clone();
        let accepted_resp = tokio::task::spawn_blocking(move || {
            send_request_to(
                &bob_sock_accept,
                &IpcRequest::AcceptTransfer {
                    transfer_id: pending_id,
                },
            )
        })
        .await
        .expect("join accept")
        .expect("accept ipc ok");
        assert!(matches!(accepted_resp, IpcResponse::Transfer(_)));

        let send_resp = send_handle.await.expect("join send").expect("send ipc ok");
        let IpcResponse::Transfer(sent_session) = send_resp else {
            panic!("expected completed transfer response, got {send_resp:?}");
        };
        assert_eq!(sent_session.state, TransferState::Completed);
        assert_eq!(
            fs::read(bob_incoming.join("note.txt")).expect("read received file"),
            payload
        );

        let _ = alice_tx.send(());
        let _ = bob_tx.send(());
        alice_task.await.expect("join alice").expect("alice exit");
        bob_task.await.expect("join bob").expect("bob exit");
        let _ = fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn daemon_ipc_handles_visibility_config_rename_and_device_auto_accept() {
        let dir = temp_service_dir("ipc-control");
        let incoming_dir = dir.join("incoming");
        let incoming_dir_updated = dir.join("incoming-updated");
        fs::create_dir_all(&incoming_dir).expect("create incoming dir");
        fs::create_dir_all(&incoming_dir_updated).expect("create updated incoming dir");

        let config_path = dir.join("config.toml");
        let sock_path = dir.join("daemon.sock");
        let device_store = PairedDeviceStore::new(dir.join("devices.toml"));
        let secret_store = Arc::new(InMemorySecretStore::new());

        let initial_config = DaemonConfig {
            incoming_directory: incoming_dir.display().to_string(),
            visibility_mode: VisibilityMode::Hidden,
            ..Default::default()
        };
        initial_config
            .save_to_path(&config_path)
            .expect("save initial config");

        commit_paired_device(
            &device_store,
            secret_store.as_ref(),
            PairedDevice {
                device_id: "pixel-9".to_string(),
                display_name: "Pixel 9".to_string(),
                device_role: DeviceRole::Mobile,
                static_x25519_pubkey: "cc".repeat(32),
                last_ble_address: String::new(),
                last_lan_address: "127.0.0.1:4242".to_string(),
                last_endpoint_id: "dd".repeat(32),
                capability_flags: DEFAULT_CAPABILITY_FLAGS,
                auto_accept_files: false,
                paired_at: "1".to_string(),
            },
            &PeerSecrets {
                irk: [0x21u8; 32],
                reconnect_secret: [0x31u8; 32],
            },
        )
        .expect("seed paired device");

        let (tx, rx) = oneshot::channel::<()>();
        let runtime_task = tokio::spawn({
            let store = device_store.clone();
            let secrets = Arc::clone(&secret_store);
            let sock = sock_path.clone();
            let cfg_path = config_path.clone();
            let cfg = initial_config.clone();
            async move {
                DaemonRuntime::run_with_deps(
                    &cfg,
                    RuntimeOptions {
                        local_device_id: "host-linux".to_string(),
                        display_name: "Host".to_string(),
                        everyone_nearby_timeout: Duration::from_secs(120),
                        enable_bluez: false,
                        enable_mdns: false,
                        enable_ipc: true,
                        enable_tray: false,
                        enable_notifications: false,
                        enable_clipboard: false,
                        config_path: Some(cfg_path),
                        ipc_socket_path: Some(sock),
                        quic_bind_addr: Some("127.0.0.1:0".parse().expect("addr")),
                        ..Default::default()
                    },
                    store,
                    Some(secrets),
                    async move {
                        let _ = rx.await;
                    },
                )
                .await
            }
        });

        let initial_status = wait_for_status(&sock_path).await;
        assert_eq!(initial_status.visibility_mode, VisibilityMode::Hidden);
        assert_eq!(initial_status.everyone_remaining_secs, None);

        let everyone_resp = {
            let sock = sock_path.clone();
            tokio::task::spawn_blocking(move || {
                send_request_to(
                    &sock,
                    &IpcRequest::SetVisibility {
                        mode: VisibilityMode::EveryoneNearby,
                    },
                )
            })
            .await
            .expect("join set visibility")
            .expect("set visibility ok")
        };
        let IpcResponse::Status(everyone_status) = everyone_resp else {
            panic!("expected status response, got {everyone_resp:?}");
        };
        assert_eq!(
            everyone_status.visibility_mode,
            VisibilityMode::EveryoneNearby
        );
        assert!(everyone_status.everyone_remaining_secs.unwrap_or(0) > 0);
        let saved_after_vis =
            DaemonConfig::load_or_default(&config_path).expect("reload persisted config");
        assert_eq!(
            saved_after_vis.visibility_mode,
            VisibilityMode::EveryoneNearby
        );

        let paired_resp = {
            let sock = sock_path.clone();
            tokio::task::spawn_blocking(move || {
                send_request_to(
                    &sock,
                    &IpcRequest::SetVisibility {
                        mode: VisibilityMode::PairedOnly,
                    },
                )
            })
            .await
            .expect("join paired visibility")
            .expect("set paired visibility ok")
        };
        let IpcResponse::Status(paired_status) = paired_resp else {
            panic!("expected status response, got {paired_resp:?}");
        };
        assert_eq!(paired_status.visibility_mode, VisibilityMode::PairedOnly);
        assert_eq!(paired_status.everyone_remaining_secs, None);

        let devices_resp = {
            let sock = sock_path.clone();
            tokio::task::spawn_blocking(move || send_request_to(&sock, &IpcRequest::ListDevices))
                .await
                .expect("join list devices")
                .expect("list devices ok")
        };
        let IpcResponse::Devices(devices) = devices_resp else {
            panic!("expected devices response, got {devices_resp:?}");
        };
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].device_id, "pixel-9");

        let renamed_resp = {
            let sock = sock_path.clone();
            tokio::task::spawn_blocking(move || {
                send_request_to(
                    &sock,
                    &IpcRequest::RenameDevice {
                        device_id: "pixel-9".to_string(),
                        display_name: "Pixel Desk".to_string(),
                    },
                )
            })
            .await
            .expect("join rename device")
            .expect("rename device ok")
        };
        let IpcResponse::Device(renamed_device) = renamed_resp else {
            panic!("expected device response, got {renamed_resp:?}");
        };
        assert_eq!(renamed_device.display_name, "Pixel Desk");

        let auto_accept_resp = {
            let sock = sock_path.clone();
            tokio::task::spawn_blocking(move || {
                send_request_to(
                    &sock,
                    &IpcRequest::SetDeviceAutoAccept {
                        device_id: "pixel-9".to_string(),
                        enabled: true,
                    },
                )
            })
            .await
            .expect("join set device auto accept")
            .expect("set device auto accept ok")
        };
        let IpcResponse::Device(auto_device) = auto_accept_resp else {
            panic!("expected device response, got {auto_accept_resp:?}");
        };
        assert!(auto_device.auto_accept_files);

        let get_cfg_resp = {
            let sock = sock_path.clone();
            tokio::task::spawn_blocking(move || send_request_to(&sock, &IpcRequest::GetConfig))
                .await
                .expect("join get config")
                .expect("get config ok")
        };
        let IpcResponse::Config(mut updated_cfg) = get_cfg_resp else {
            panic!("expected config response, got {get_cfg_resp:?}");
        };
        updated_cfg.auto_accept_from_paired = true;
        updated_cfg.incoming_directory = incoming_dir_updated.display().to_string();

        let update_cfg_resp = {
            let sock = sock_path.clone();
            let cfg_req = updated_cfg.clone();
            tokio::task::spawn_blocking(move || {
                send_request_to(&sock, &IpcRequest::UpdateConfig { config: cfg_req })
            })
            .await
            .expect("join update config")
            .expect("update config ok")
        };
        let IpcResponse::Config(returned_cfg) = update_cfg_resp else {
            panic!("expected config response, got {update_cfg_resp:?}");
        };
        assert!(returned_cfg.auto_accept_from_paired);
        assert_eq!(
            returned_cfg.incoming_directory,
            incoming_dir_updated.display().to_string()
        );
        let persisted_cfg =
            DaemonConfig::load_or_default(&config_path).expect("reload updated cfg");
        assert!(persisted_cfg.auto_accept_from_paired);
        assert_eq!(
            persisted_cfg.incoming_directory,
            incoming_dir_updated.display().to_string()
        );

        let unpair_resp = {
            let sock = sock_path.clone();
            tokio::task::spawn_blocking(move || {
                send_request_to(
                    &sock,
                    &IpcRequest::UnpairDevice {
                        device_id: "pixel-9".to_string(),
                    },
                )
            })
            .await
            .expect("join unpair device")
            .expect("unpair device ok")
        };
        assert_eq!(
            unpair_resp,
            IpcResponse::Unpaired {
                device_id: "pixel-9".to_string()
            }
        );
        assert!(device_store.load().expect("reload devices").is_empty());

        let _ = tx.send(());
        runtime_task
            .await
            .expect("join runtime")
            .expect("runtime exit");
        let _ = fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn latest_otp_stays_in_memory_only_and_enables_tray_action() {
        let dir = temp_service_dir("otp-ram");
        let incoming_dir = dir.join("incoming");
        fs::create_dir_all(&incoming_dir).expect("mkdir");
        let config_path = dir.join("config.toml");
        let mut initial_config = DaemonConfig {
            incoming_directory: incoming_dir.display().to_string(),
            ..Default::default()
        };
        initial_config
            .save_to_path(&config_path)
            .expect("save config");

        let device_store = PairedDeviceStore::new(dir.join("devices.toml"));
        let (visibility_tx, visibility_rx) = watch::channel(initial_config.visibility_mode);
        let (config_watch_tx, _config_watch_rx) = watch::channel(initial_config.clone());
        let clipboard_engine =
            Arc::new(ClipboardEngine::new(ClipboardPolicy::from(&initial_config)));
        let clipboard_handle = ClipboardMonitorHandle::headless();
        let latest_otp = Arc::new(Mutex::new(None));
        let refresh_notify = Arc::new(Notify::new());

        let ctx = IpcDispatchContext {
            local_device_id: "host-linux".to_string(),
            visibility_tx,
            visibility_rx,
            config_state: Arc::new(Mutex::new(initial_config.clone())),
            config_watch_tx,
            config_path: Some(config_path.clone()),
            everyone_nearby_timeout: Duration::from_secs(60),
            everyone_deadline: Arc::new(Mutex::new(None)),
            deadline_notify: Arc::new(Notify::new()),
            refresh_notify: Arc::clone(&refresh_notify),
            device_store,
            secret_store: None,
            quic_node: None,
            transfer_engine: Arc::new(TransferEngine::new(incoming_dir, false)),
            clipboard_engine: Arc::clone(&clipboard_engine),
            clipboard_handle: clipboard_handle.clone(),
            latest_otp: Arc::clone(&latest_otp),
            known_peers: Arc::new(Mutex::new(HashSet::new())),
            mdns_degraded: false,
            mdns: None,
        };

        assert!(!build_tray_state(&ctx).await.has_latest_otp);

        apply_incoming_clipboard_item(
            ClipboardItem {
                id: "otp-1".to_string(),
                peer_device_id: "pixel-9".to_string(),
                mime_type: "text/plain;charset=utf-8".to_string(),
                data: b"847291".to_vec(),
                is_otp: true,
                is_image_offer: false,
                offer_id: String::new(),
                offered_mimes: Vec::new(),
            },
            &clipboard_engine,
            None,
            &clipboard_handle,
            &latest_otp,
            &refresh_notify,
        )
        .await;

        assert!(build_tray_state(&ctx).await.has_latest_otp);
        assert_eq!(
            latest_otp.lock().await.as_deref().map(String::as_str),
            Some("847291")
        );

        apply_incoming_clipboard_item(
            ClipboardItem {
                id: "otp-2".to_string(),
                peer_device_id: "pixel-9".to_string(),
                mime_type: "text/plain;charset=utf-8".to_string(),
                data: b"654321".to_vec(),
                is_otp: true,
                is_image_offer: false,
                offer_id: String::new(),
                offered_mimes: Vec::new(),
            },
            &clipboard_engine,
            None,
            &clipboard_handle,
            &latest_otp,
            &refresh_notify,
        )
        .await;

        assert_eq!(
            latest_otp.lock().await.as_deref().map(String::as_str),
            Some("654321")
        );
        handle_tray_command(TrayCommand::CopyLatestOtp, &ctx).await;

        let status = build_daemon_status(&ctx).await.expect("status");
        let status_debug = format!("{status:?}");
        assert!(!status_debug.contains("847291"));
        assert!(!status_debug.contains("654321"));

        initial_config
            .ignored_mime_types
            .push("application/x-custom-secret".to_string());
        let resp = dispatch_ipc_request(
            IpcRequest::UpdateConfig {
                config: initial_config,
            },
            &ctx,
        )
        .await;
        assert!(matches!(resp, IpcResponse::Config(_)));
        assert!(!clipboard_engine.can_accept_mimes(
            &["application/x-custom-secret".to_string()],
            false,
            false
        ));

        let saved_toml = fs::read_to_string(&config_path).expect("read config.toml");
        assert!(!saved_toml.contains("847291"));
        assert!(!saved_toml.contains("654321"));

        let _ = fs::remove_dir_all(&dir);
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
            match tokio::task::spawn_blocking(move || send_request_to(&p, &IpcRequest::Status))
                .await
            {
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

    async fn wait_for_pending_transfer(
        sock_path: &Path,
        send_handle: &mut tokio::task::JoinHandle<Result<IpcResponse, KeynearError>>,
    ) -> String {
        for _ in 0..80 {
            let p = sock_path.to_path_buf();
            let list_fut = tokio::task::spawn_blocking(move || {
                send_request_to(&p, &IpcRequest::ListTransfers)
            });
            tokio::select! {
                send_res = &mut *send_handle => {
                    panic!("send task finished before PendingConsent appeared: {send_res:?}");
                }
                list_res = list_fut => {
                    if let Ok(Ok(IpcResponse::Transfers(list))) = list_res {
                        if let Some(session) = list
                            .into_iter()
                            .find(|s| s.state == TransferState::PendingConsent)
                        {
                            return session.transfer_id;
                        }
                    }
                }
            }
            tokio::select! {
                send_res = &mut *send_handle => {
                    panic!("send task finished while waiting for PendingConsent: {send_res:?}");
                }
                _ = tokio::time::sleep(Duration::from_millis(25)) => {}
            }
        }
        panic!("timed out waiting for pending consent transfer");
    }
}
