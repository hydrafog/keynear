use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use keynear_core::{ClipboardEngine, ClipboardItem, ClipboardPolicy, TransferEngine};
use keynear_ipc::{IpcRequest, IpcResponse};
use keynear_protocol::DaemonConfig;
use tokio::sync::{watch, Mutex, Notify};
use zeroize::Zeroizing;

use super::*;
use crate::clipboard::ClipboardMonitorHandle;
use crate::ipc::{
    build_daemon_status, build_tray_state, dispatch_ipc_request, handle_tray_command,
    IpcDispatchContext,
};
use crate::service::clipboard::apply_incoming_clipboard_item;
use crate::tray::TrayCommand;

struct OtpRig {
    dir: PathBuf,
    config_path: PathBuf,
    ctx: IpcDispatchContext,
    clipboard_engine: Arc<ClipboardEngine>,
    clipboard_handle: ClipboardMonitorHandle,
    latest_otp: Arc<Mutex<Option<Zeroizing<String>>>>,
    refresh_notify: Arc<Notify>,
    initial_config: DaemonConfig,
}

fn build_otp_rig() -> OtpRig {
    let dir = temp_service_dir("otp-ram");
    let incoming_dir = dir.join("incoming");
    fs::create_dir_all(&incoming_dir).expect("mkdir");
    let config_path = dir.join("config.toml");
    let initial_config = DaemonConfig {
        incoming_directory: incoming_dir.display().to_string(),
        ..Default::default()
    };
    initial_config.save_to_path(&config_path).expect("save");
    let clipboard_engine = Arc::new(ClipboardEngine::new(ClipboardPolicy::from(&initial_config)));
    let clipboard_handle = ClipboardMonitorHandle::headless();
    let latest_otp = Arc::new(Mutex::new(None));
    let refresh_notify = Arc::new(Notify::new());
    let (visibility_tx, visibility_rx) = watch::channel(initial_config.visibility_mode);
    let (config_watch_tx, _rx) = watch::channel(initial_config.clone());
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
        device_store: PairedDeviceStore::new(dir.join("devices.toml")),
        secret_store: None,
        quic_node: None,
        transfer_engine: Arc::new(TransferEngine::new(incoming_dir, false)),
        clipboard_engine: Arc::clone(&clipboard_engine),
        clipboard_handle: clipboard_handle.clone(),
        latest_otp: Arc::clone(&latest_otp),
        known_peers: Arc::new(Mutex::new(HashSet::new())),
        mdns_degraded: false,
        mdns: None,
        pairing_decision_tx: None,
        phoenix_supervisor: None,
    };
    OtpRig {
        dir,
        config_path,
        ctx,
        clipboard_engine,
        clipboard_handle,
        latest_otp,
        refresh_notify,
        initial_config,
    }
}

fn otp_item(id: &str, code: &[u8]) -> ClipboardItem {
    ClipboardItem {
        id: id.to_string(),
        peer_device_id: "pixel-9".to_string(),
        mime_type: "text/plain;charset=utf-8".to_string(),
        data: code.to_vec(),
        is_otp: true,
        is_image_offer: false,
        offer_id: String::new(),
        offered_mimes: Vec::new(),
    }
}

async fn stage_otp(rig: &OtpRig, id: &str, code: &[u8]) {
    apply_incoming_clipboard_item(
        otp_item(id, code),
        &rig.clipboard_engine,
        None,
        &rig.clipboard_handle,
        &rig.latest_otp,
        &rig.refresh_notify,
    )
    .await;
}

#[tokio::test]
async fn latest_otp_stays_in_memory_only_and_enables_tray_action() {
    let rig = build_otp_rig();
    assert!(!build_tray_state(&rig.ctx).await.has_latest_otp);

    stage_otp(&rig, "otp-1", b"847291").await;
    assert!(build_tray_state(&rig.ctx).await.has_latest_otp);
    assert_eq!(
        rig.latest_otp.lock().await.as_deref().map(String::as_str),
        Some("847291")
    );

    stage_otp(&rig, "otp-2", b"654321").await;
    assert_eq!(
        rig.latest_otp.lock().await.as_deref().map(String::as_str),
        Some("654321")
    );
    handle_tray_command(TrayCommand::CopyLatestOtp, &rig.ctx).await;

    let status = build_daemon_status(&rig.ctx).await.expect("status");
    let status_debug = format!("{status:?}");
    assert!(!status_debug.contains("847291"));
    assert!(!status_debug.contains("654321"));

    let mut config = rig.initial_config.clone();
    config
        .ignored_mime_types
        .push("application/x-custom-secret".to_string());
    let resp = dispatch_ipc_request(IpcRequest::UpdateConfig { config }, &rig.ctx).await;
    assert!(matches!(resp, IpcResponse::Config(_)));
    assert!(!rig.clipboard_engine.can_accept_mimes(
        &["application/x-custom-secret".to_string()],
        false,
        false
    ));

    let saved_toml = fs::read_to_string(&rig.config_path).expect("read config.toml");
    assert!(!saved_toml.contains("847291"));
    assert!(!saved_toml.contains("654321"));

    let _ = fs::remove_dir_all(&rig.dir);
}
