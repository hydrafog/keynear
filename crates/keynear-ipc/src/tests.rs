use std::os::unix::net::UnixListener;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use super::*;
use keynear_protocol::{TransferDirection, TransferItem, TransferState};
use proptest::prelude::*;

fn temp_sock_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!("keynear-ipc-{label}-{nanos}"))
}

fn sample_session() -> TransferSession {
    TransferSession {
        transfer_id: "tx-123".to_string(),
        peer_device_id: "pixel-9".to_string(),
        direction: TransferDirection::Receive,
        items: vec![TransferItem {
            name: "report\nfinal.pdf".to_string(),
            mime_type: "application/pdf".to_string(),
            size_bytes: 12_345,
            is_directory: false,
            target_path: Some("/tmp/report\nfinal.pdf".to_string()),
            checksum: Some("abcd".repeat(16)),
        }],
        total_bytes: 12_345,
        transferred_bytes: 6_144,
        state: TransferState::PendingConsent,
        started_at: "1700000000".to_string(),
    }
}

#[test]
fn request_and_response_roundtrip_all_variants() {
    let requests = vec![
        IpcRequest::SendFiles {
            device_id: "pixel-9".to_string(),
            paths: vec![
                PathBuf::from("/tmp/hello world.txt"),
                PathBuf::from("/tmp/dir/sub\nfile.bin"),
            ],
        },
        IpcRequest::AcceptTransfer {
            transfer_id: "tx-1".to_string(),
        },
        IpcRequest::DeclineTransfer {
            transfer_id: "tx-2".to_string(),
        },
        IpcRequest::CancelTransfer {
            transfer_id: "tx-3".to_string(),
        },
        IpcRequest::ListTransfers,
        IpcRequest::Status,
        IpcRequest::ListDevices,
        IpcRequest::SetVisibility {
            mode: VisibilityMode::EveryoneNearby,
        },
        IpcRequest::GetConfig,
        IpcRequest::UpdateConfig {
            config: DaemonConfig {
                visibility_mode: VisibilityMode::Hidden,
                incoming_directory: "/tmp/keynear-in".to_string(),
                auto_accept_from_paired: true,
                ..DaemonConfig::default()
            },
        },
        IpcRequest::RenameDevice {
            device_id: "pixel-9".to_string(),
            display_name: "Pixel Renamed".to_string(),
        },
        IpcRequest::SetDeviceAutoAccept {
            device_id: "pixel-9".to_string(),
            enabled: true,
        },
        IpcRequest::UnpairDevice {
            device_id: "pixel-9".to_string(),
        },
        IpcRequest::RespondPairing { approve: true },
        IpcRequest::RespondPairing { approve: false },
    ];
    for req in requests {
        let encoded = encode_request(&req).expect("encode request");
        let decoded = decode_request(&encoded).expect("decode request");
        assert_eq!(decoded, req);
    }

    let sample_device = PairedDevice {
        device_id: "pixel-9".to_string(),
        display_name: "Pixel 9".to_string(),
        device_role: DeviceRole::Mobile,
        static_x25519_pubkey: "abcd".to_string(),
        last_ble_address: "AA:BB:CC:DD:EE:FF".to_string(),
        last_lan_address: "192.168.1.42:5555".to_string(),
        last_endpoint_id: "ef".repeat(32),
        capability_flags: 1,
        auto_accept_files: true,
        paired_at: "2026-11-14T00:00:00Z".to_string(),
    };

    let responses = vec![
        IpcResponse::Transfer(sample_session()),
        IpcResponse::Transfers(vec![sample_session()]),
        IpcResponse::Status(DaemonStatus {
            device_id: "linux-workstation".to_string(),
            endpoint_id: "ab".repeat(32),
            listen_addrs: vec!["127.0.0.1:42000".to_string()],
            visibility_mode: VisibilityMode::EveryoneNearby,
            mdns_degraded: true,
            active_transfer_count: 2,
            paired_device_count: 1,
            everyone_remaining_secs: Some(321),
            auto_accept_from_paired: true,
            incoming_directory: "/tmp/keynear-in".to_string(),
        }),
        IpcResponse::Devices(vec![sample_device.clone()]),
        IpcResponse::Device(sample_device),
        IpcResponse::Config(DaemonConfig::default()),
        IpcResponse::Unpaired {
            device_id: "pixel-9".to_string(),
        },
        IpcResponse::PairingDecision { approved: true },
        IpcResponse::PairingDecision { approved: false },
        IpcResponse::Error {
            message: "transfer not found".to_string(),
        },
    ];
    for resp in responses {
        let encoded = encode_response(&resp).expect("encode response");
        let decoded = decode_response(&encoded).expect("decode response");
        assert_eq!(decoded, resp);
    }
}

#[test]
fn rejects_invalid_and_oversized_ipc_frames() {
    assert!(matches!(
        encode_request(&IpcRequest::SendFiles {
            device_id: "../evil".to_string(),
            paths: vec![PathBuf::from("/tmp/a")],
        }),
        Err(KeynearError::Validation {
            field: "deviceId",
            ..
        })
    ));
    assert!(matches!(
        encode_request(&IpcRequest::SendFiles {
            device_id: "pixel-9".to_string(),
            paths: Vec::new(),
        }),
        Err(KeynearError::Validation { field: "paths", .. })
    ));
    assert!(matches!(
        encode_request(&IpcRequest::AcceptTransfer {
            transfer_id: "bad/id".to_string(),
        }),
        Err(KeynearError::Validation {
            field: "transferId",
            ..
        })
    ));
    assert!(matches!(
        decode_request(&[]),
        Err(KeynearError::Protocol(_))
    ));
    let mut bad_trailing = encode_request(&IpcRequest::Status).expect("encode");
    bad_trailing.push(0x99);
    assert!(matches!(
        decode_request(&bad_trailing),
        Err(KeynearError::Protocol(_))
    ));
    let oversized = vec![0u8; MAX_IPC_FRAME_BYTES + 1];
    assert!(matches!(
        decode_response(&oversized),
        Err(KeynearError::FrameTooLarge { .. })
    ));
}

#[test]
fn loopback_unix_socket_enforces_0600_permissions_and_roundtrips() {
    let dir = temp_sock_dir("loopback");
    let sock_path = dir.join("daemon.sock");
    prepare_daemon_socket_dir(&sock_path).expect("prepare dir");
    let listener = UnixListener::bind(&sock_path).expect("bind listener");
    set_daemon_socket_permissions(&sock_path).expect("set 0600");

    let dir_mode = fs::metadata(&dir).expect("dir meta").permissions().mode() & 0o777;
    let sock_mode = fs::metadata(&sock_path)
        .expect("sock meta")
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(dir_mode, 0o700);
    assert_eq!(sock_mode, 0o600);

    let expected = sample_session();
    let expected_clone = expected.clone();
    let server = thread::spawn(move || {
        let (mut conn, _) = listener.accept().expect("accept");
        let mut len_buf = [0u8; 4];
        conn.read_exact(&mut len_buf).expect("read len");
        let len = u32::from_be_bytes(len_buf) as usize;
        let mut buf = vec![0u8; len];
        conn.read_exact(&mut buf).expect("read body");
        let req = decode_request(&buf).expect("decode req");
        assert_eq!(
            req,
            IpcRequest::AcceptTransfer {
                transfer_id: "tx-123".to_string()
            }
        );
        let resp = encode_response(&IpcResponse::Transfer(expected_clone)).expect("encode resp");
        let rlen = (resp.len() as u32).to_be_bytes();
        conn.write_all(&rlen).expect("write len");
        conn.write_all(&resp).expect("write body");
    });

    let response = send_request_to(
        &sock_path,
        &IpcRequest::AcceptTransfer {
            transfer_id: "tx-123".to_string(),
        },
    )
    .expect("send ipc request");
    assert_eq!(response, IpcResponse::Transfer(expected));
    server.join().expect("server thread");

    let _ = fs::remove_dir_all(&dir);
}

fn token() -> impl Strategy<Value = String> {
    prop::collection::vec(b'a'..=b'z', 1..=8).prop_map(|v| v.into_iter().map(char::from).collect())
}

fn free_text() -> impl Strategy<Value = String> {
    prop::collection::vec(b' '..=b'~', 0..=16).prop_map(|v| v.into_iter().map(char::from).collect())
}

proptest! {
    #[test]
    fn generated_request_roundtrips(request in prop_oneof![
        (token(), prop::collection::vec(token(), 1..=3)).prop_map(|(device_id, paths)| {
            IpcRequest::SendFiles { device_id, paths: paths.into_iter().map(PathBuf::from).collect() }
        }),
        token().prop_map(|transfer_id| IpcRequest::AcceptTransfer { transfer_id }),
        Just(IpcRequest::ListTransfers),
        Just(IpcRequest::Status),
        Just(IpcRequest::ListDevices),
        prop_oneof![
            Just(VisibilityMode::Hidden),
            Just(VisibilityMode::PairedOnly),
            Just(VisibilityMode::EveryoneNearby),
        ]
        .prop_map(|mode| IpcRequest::SetVisibility { mode }),
        Just(IpcRequest::GetConfig),
        token().prop_map(|dir| IpcRequest::UpdateConfig {
            config: DaemonConfig { incoming_directory: dir, ..DaemonConfig::default() },
        }),
        (token(), token()).prop_map(|(device_id, display_name)| IpcRequest::RenameDevice { device_id, display_name }),
        (token(), any::<bool>()).prop_map(|(device_id, enabled)| IpcRequest::SetDeviceAutoAccept { device_id, enabled }),
        token().prop_map(|device_id| IpcRequest::UnpairDevice { device_id }),
        any::<bool>().prop_map(|approve| IpcRequest::RespondPairing { approve }),
    ]) {
        let encoded = encode_request(&request).expect("encode request");
        prop_assert_eq!(decode_request(&encoded).expect("decode request"), request);
    }

    #[test]
    fn generated_response_roundtrips(s in free_text(), id in token(), flag in any::<bool>()) {
        let mut session = sample_session();
        session.transfer_id = s.clone();
        let responses = [
            IpcResponse::Transfer(session),
            IpcResponse::Config(DaemonConfig { incoming_directory: id.clone(), ..DaemonConfig::default() }),
            IpcResponse::Unpaired { device_id: id },
            IpcResponse::PairingDecision { approved: flag },
            IpcResponse::Error { message: s },
        ];
        for response in responses {
            let encoded = encode_response(&response).expect("encode response");
            prop_assert_eq!(decode_response(&encoded).expect("decode response"), response);
        }
    }

    #[test]
    fn arbitrary_non_frames_rejected(
        first in any::<u8>().prop_filter("not a wire version", |b| *b != 1),
        tail in prop::collection::vec(any::<u8>(), 0..=1024),
    ) {
        let mut bytes = vec![first];
        bytes.extend(tail);
        prop_assert!(decode_request(&bytes).is_err());
        prop_assert!(decode_response(&bytes).is_err());
    }
}
