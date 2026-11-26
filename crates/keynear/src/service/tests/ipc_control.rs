use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use keynear_core::{
    commit_paired_device, DeviceRole, InMemorySecretStore, PairedDeviceStore, PeerSecrets,
};
use keynear_ipc::{send_request_to, IpcRequest, IpcResponse};
use keynear_protocol::{DaemonConfig, PairedDevice, VisibilityMode};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

use super::*;
use crate::ble::DEFAULT_CAPABILITY_FLAGS;

struct ControlRig {
    dir: PathBuf,
    config_path: PathBuf,
    sock_path: PathBuf,
    device_store: PairedDeviceStore,
    incoming_dir_updated: PathBuf,
    tx: oneshot::Sender<()>,
    runtime_task: JoinHandle<Result<(), KeynearError>>,
}

async fn ipc_request(sock_path: &Path, request: IpcRequest) -> IpcResponse {
    let sock = sock_path.to_path_buf();
    tokio::task::spawn_blocking(move || send_request_to(&sock, &request))
        .await
        .expect("join ipc request")
        .expect("ipc request ok")
}

fn seed_control_device(store: &PairedDeviceStore, secrets: &InMemorySecretStore) {
    commit_paired_device(
        store,
        secrets,
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
}

fn spawn_control_rig() -> ControlRig {
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

    seed_control_device(&device_store, &secret_store);

    let (tx, rx) = oneshot::channel::<()>();
    let mut options = ipc_runtime_options("host-linux", "Host", sock_path.clone());
    options.everyone_nearby_timeout = Duration::from_secs(120);
    options.config_path = Some(config_path.clone());
    let runtime_task = spawn_test_daemon(TestDaemonParams {
        config: initial_config.clone(),
        options,
        store: device_store.clone(),
        secrets: Some(secret_store),
        rx,
    });

    ControlRig {
        dir,
        config_path,
        sock_path,
        device_store,
        incoming_dir_updated,
        tx,
        runtime_task,
    }
}

async fn check_everyone_visibility(rig: &ControlRig) {
    let everyone_resp = ipc_request(
        &rig.sock_path,
        IpcRequest::SetVisibility {
            mode: VisibilityMode::EveryoneNearby,
        },
    )
    .await;
    let IpcResponse::Status(everyone_status) = everyone_resp else {
        panic!("expected status response, got {everyone_resp:?}");
    };
    assert_eq!(
        everyone_status.visibility_mode,
        VisibilityMode::EveryoneNearby
    );
    assert!(everyone_status.everyone_remaining_secs.unwrap_or(0) > 0);
    let saved_after_vis =
        DaemonConfig::load_or_default(&rig.config_path).expect("reload persisted config");
    assert_eq!(
        saved_after_vis.visibility_mode,
        VisibilityMode::EveryoneNearby
    );
}

async fn check_paired_visibility(rig: &ControlRig) {
    let paired_resp = ipc_request(
        &rig.sock_path,
        IpcRequest::SetVisibility {
            mode: VisibilityMode::PairedOnly,
        },
    )
    .await;
    let IpcResponse::Status(paired_status) = paired_resp else {
        panic!("expected status response, got {paired_resp:?}");
    };
    assert_eq!(paired_status.visibility_mode, VisibilityMode::PairedOnly);
    assert_eq!(paired_status.everyone_remaining_secs, None);
}

async fn check_visibility(rig: &ControlRig) {
    let initial_status = wait_for_status(&rig.sock_path).await;
    assert_eq!(initial_status.visibility_mode, VisibilityMode::Hidden);
    assert_eq!(initial_status.everyone_remaining_secs, None);
    check_everyone_visibility(rig).await;
    check_paired_visibility(rig).await;
}

async fn check_devices(rig: &ControlRig) {
    let devices_resp = ipc_request(&rig.sock_path, IpcRequest::ListDevices).await;
    let IpcResponse::Devices(devices) = devices_resp else {
        panic!("expected devices response, got {devices_resp:?}");
    };
    assert_eq!(devices.len(), 1);
    assert_eq!(devices[0].device_id, "pixel-9");

    let renamed_resp = ipc_request(
        &rig.sock_path,
        IpcRequest::RenameDevice {
            device_id: "pixel-9".to_string(),
            display_name: "Pixel Desk".to_string(),
        },
    )
    .await;
    let IpcResponse::Device(renamed_device) = renamed_resp else {
        panic!("expected device response, got {renamed_resp:?}");
    };
    assert_eq!(renamed_device.display_name, "Pixel Desk");

    let auto_accept_resp = ipc_request(
        &rig.sock_path,
        IpcRequest::SetDeviceAutoAccept {
            device_id: "pixel-9".to_string(),
            enabled: true,
        },
    )
    .await;
    let IpcResponse::Device(auto_device) = auto_accept_resp else {
        panic!("expected device response, got {auto_accept_resp:?}");
    };
    assert!(auto_device.auto_accept_files);
}

fn assert_updated_config(cfg: &DaemonConfig, expected_dir: &str) {
    assert!(cfg.auto_accept_from_paired);
    assert!(cfg.proximity_lock_enabled);
    assert!((cfg.proximity_distance_meters - 5.5).abs() < f32::EPSILON);
    assert_eq!(cfg.incoming_directory, expected_dir);
}

async fn check_config(rig: &ControlRig) {
    let get_cfg_resp = ipc_request(&rig.sock_path, IpcRequest::GetConfig).await;
    let IpcResponse::Config(mut updated_cfg) = get_cfg_resp else {
        panic!("expected config response, got {get_cfg_resp:?}");
    };
    let expected_dir = rig.incoming_dir_updated.display().to_string();
    updated_cfg.auto_accept_from_paired = true;
    updated_cfg.incoming_directory = expected_dir.clone();
    updated_cfg.proximity_lock_enabled = true;
    updated_cfg.proximity_distance_meters = 5.5;

    let update_cfg_resp = ipc_request(
        &rig.sock_path,
        IpcRequest::UpdateConfig {
            config: updated_cfg,
        },
    )
    .await;
    let IpcResponse::Config(returned_cfg) = update_cfg_resp else {
        panic!("expected config response, got {update_cfg_resp:?}");
    };
    assert_updated_config(&returned_cfg, &expected_dir);
    let persisted_cfg =
        DaemonConfig::load_or_default(&rig.config_path).expect("reload updated cfg");
    assert_updated_config(&persisted_cfg, &expected_dir);
}

async fn check_unpair(rig: &ControlRig) {
    let unpair_resp = ipc_request(
        &rig.sock_path,
        IpcRequest::UnpairDevice {
            device_id: "pixel-9".to_string(),
        },
    )
    .await;
    assert_eq!(
        unpair_resp,
        IpcResponse::Unpaired {
            device_id: "pixel-9".to_string()
        }
    );
    assert!(rig.device_store.load().expect("reload devices").is_empty());
}

#[tokio::test]
async fn daemon_ipc_handles_visibility_config_rename_and_device_auto_accept() {
    let rig = spawn_control_rig();
    check_visibility(&rig).await;
    check_devices(&rig).await;
    check_config(&rig).await;
    check_unpair(&rig).await;

    let _ = rig.tx.send(());
    rig.runtime_task
        .await
        .expect("join runtime")
        .expect("runtime exit");
    let _ = fs::remove_dir_all(&rig.dir);
}
