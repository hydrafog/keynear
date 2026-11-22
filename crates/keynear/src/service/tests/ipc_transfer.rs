use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use keynear_core::{
    commit_paired_device, DeviceRole, InMemorySecretStore, PairedDeviceStore, PeerSecrets,
};
use keynear_ipc::{send_request_to, IpcRequest, IpcResponse};
use keynear_protocol::{DaemonConfig, PairedDevice};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

type DaemonTask = JoinHandle<Result<(), KeynearError>>;

use super::*;
use crate::ble::DEFAULT_CAPABILITY_FLAGS;

struct TransferRig {
    dir: PathBuf,
    alice_store: PairedDeviceStore,
    bob_store: PairedDeviceStore,
    alice_secrets: Arc<InMemorySecretStore>,
    bob_secrets: Arc<InMemorySecretStore>,
    alice_sock: PathBuf,
    bob_sock: PathBuf,
    bob_incoming: PathBuf,
    alice_tx: oneshot::Sender<()>,
    bob_tx: oneshot::Sender<()>,
    alice_task: DaemonTask,
    bob_task: DaemonTask,
}

struct DaemonSpawnParams {
    alice_incoming: PathBuf,
    bob_incoming: PathBuf,
    alice_sock: PathBuf,
    bob_sock: PathBuf,
    alice_store: PairedDeviceStore,
    bob_store: PairedDeviceStore,
    alice_secrets: Arc<InMemorySecretStore>,
    bob_secrets: Arc<InMemorySecretStore>,
}

fn spawn_ipc_daemons(
    params: DaemonSpawnParams,
) -> (
    oneshot::Sender<()>,
    oneshot::Sender<()>,
    DaemonTask,
    DaemonTask,
) {
    let bob_config = DaemonConfig {
        incoming_directory: params.bob_incoming.display().to_string(),
        ..Default::default()
    };
    let alice_config = DaemonConfig {
        incoming_directory: params.alice_incoming.display().to_string(),
        ..Default::default()
    };

    let (bob_tx, bob_rx) = oneshot::channel::<()>();
    let bob_task = spawn_test_daemon(TestDaemonParams {
        config: bob_config,
        options: ipc_runtime_options("bob-linux", "Bob", params.bob_sock),
        store: params.bob_store,
        secrets: Some(params.bob_secrets),
        rx: bob_rx,
    });

    let (alice_tx, alice_rx) = oneshot::channel::<()>();
    let alice_task = spawn_test_daemon(TestDaemonParams {
        config: alice_config,
        options: ipc_runtime_options("alice-linux", "Alice", params.alice_sock),
        store: params.alice_store,
        secrets: Some(params.alice_secrets),
        rx: alice_rx,
    });
    (alice_tx, bob_tx, alice_task, bob_task)
}

fn spawn_transfer_rig() -> TransferRig {
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
    let alice_sock = alice_dir.join("daemon.sock");
    let bob_sock = bob_dir.join("daemon.sock");

    let (alice_tx, bob_tx, alice_task, bob_task) = spawn_ipc_daemons(DaemonSpawnParams {
        alice_incoming,
        bob_incoming: bob_incoming.clone(),
        alice_sock: alice_sock.clone(),
        bob_sock: bob_sock.clone(),
        alice_store: alice_store.clone(),
        bob_store: bob_store.clone(),
        alice_secrets: Arc::clone(&alice_secrets),
        bob_secrets: Arc::clone(&bob_secrets),
    });
    TransferRig {
        dir,
        alice_store,
        bob_store,
        alice_secrets,
        bob_secrets,
        alice_sock,
        bob_sock,
        bob_incoming,
        alice_tx,
        bob_tx,
        alice_task,
        bob_task,
    }
}

struct DeviceSeed<'a> {
    device_id: &'a str,
    display_name: &'a str,
    pubkey: &'a str,
    lan: String,
    eid: String,
}

fn paired_device(seed: DeviceSeed<'_>) -> PairedDevice {
    PairedDevice {
        device_id: seed.device_id.to_string(),
        display_name: seed.display_name.to_string(),
        device_role: DeviceRole::Desktop,
        static_x25519_pubkey: seed.pubkey.repeat(32),
        last_ble_address: String::new(),
        last_lan_address: seed.lan,
        last_endpoint_id: seed.eid,
        capability_flags: DEFAULT_CAPABILITY_FLAGS,
        auto_accept_files: false,
        paired_at: "1".to_string(),
    }
}

async fn pair_rig(rig: &TransferRig) {
    let bob_status = wait_for_status(&rig.bob_sock).await;
    let alice_status = wait_for_status(&rig.alice_sock).await;
    assert_eq!(bob_status.device_id, "bob-linux");
    assert!(!bob_status.endpoint_id.is_empty());
    assert!(!bob_status.listen_addrs.is_empty());
    let shared_secret = [0x44u8; 32];

    commit_paired_device(
        &rig.alice_store,
        rig.alice_secrets.as_ref(),
        paired_device(DeviceSeed {
            device_id: "bob-linux",
            display_name: "Bob",
            pubkey: "bb",
            lan: bob_status.listen_addrs.join(","),
            eid: bob_status.endpoint_id.clone(),
        }),
        &PeerSecrets {
            irk: shared_secret,
            reconnect_secret: shared_secret,
        },
    )
    .expect("commit bob on alice");

    commit_paired_device(
        &rig.bob_store,
        rig.bob_secrets.as_ref(),
        paired_device(DeviceSeed {
            device_id: "alice-linux",
            display_name: "Alice",
            pubkey: "aa",
            lan: alice_status.listen_addrs.join(","),
            eid: alice_status.endpoint_id.clone(),
        }),
        &PeerSecrets {
            irk: shared_secret,
            reconnect_secret: shared_secret,
        },
    )
    .expect("commit alice on bob");

    wait_for_paired_devices(&rig.bob_sock, 1).await;
    wait_for_paired_devices(&rig.alice_sock, 1).await;
}

async fn run_transfer(rig: &TransferRig) {
    let sample_file = rig.dir.join("note.txt");
    let payload = b"orion daemon ipc transfer payload";
    fs::write(&sample_file, payload).expect("write sample file");

    let send_sock = rig.alice_sock.clone();
    let mut send_handle = tokio::task::spawn_blocking(move || {
        send_request_to(
            &send_sock,
            &IpcRequest::SendFiles {
                device_id: "bob-linux".to_string(),
                paths: vec![sample_file],
            },
        )
    });
    let pending_id = wait_for_pending_transfer(&rig.bob_sock, &mut send_handle).await;
    let bob_sock_accept = rig.bob_sock.clone();
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
    assert_eq!(sent_session.state, keynear_core::TransferState::Completed);
    assert_eq!(
        fs::read(rig.bob_incoming.join("note.txt")).expect("read received file"),
        payload
    );
}

#[tokio::test]
async fn daemon_ipc_handles_status_send_list_consent_and_reload() {
    let rig = spawn_transfer_rig();
    pair_rig(&rig).await;
    run_transfer(&rig).await;

    let _ = rig.alice_tx.send(());
    let _ = rig.bob_tx.send(());
    rig.alice_task
        .await
        .expect("join alice")
        .expect("alice exit");
    rig.bob_task.await.expect("join bob").expect("bob exit");
    let _ = fs::remove_dir_all(&rig.dir);
}
