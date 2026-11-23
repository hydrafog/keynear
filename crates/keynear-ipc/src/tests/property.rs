use std::path::PathBuf;

use crate::*;
use keynear_protocol::{
    DaemonConfig, DeviceRole, PairedDevice, TransferDirection, TransferItem, TransferState,
    VisibilityMode,
};
use proptest::prelude::*;

fn token() -> impl Strategy<Value = String> {
    prop::collection::vec(b'a'..=b'z', 1..=8).prop_map(|v| v.into_iter().map(char::from).collect())
}

fn free_text() -> impl Strategy<Value = String> {
    prop::collection::vec(b' '..=b'~', 0..=16).prop_map(|v| v.into_iter().map(char::from).collect())
}

fn numeric_token() -> impl Strategy<Value = String> {
    (0u64..=10_000u64).prop_map(|n| n.to_string())
}

fn visibility_strategy() -> impl Strategy<Value = VisibilityMode> {
    prop_oneof![
        Just(VisibilityMode::Hidden),
        Just(VisibilityMode::PairedOnly),
        Just(VisibilityMode::EveryoneNearby),
    ]
}

fn role_strategy() -> impl Strategy<Value = DeviceRole> {
    prop_oneof![Just(DeviceRole::Desktop), Just(DeviceRole::Mobile)]
}

fn direction_strategy() -> impl Strategy<Value = TransferDirection> {
    prop_oneof![
        Just(TransferDirection::Send),
        Just(TransferDirection::Receive)
    ]
}

fn state_strategy() -> impl Strategy<Value = TransferState> {
    prop_oneof![
        Just(TransferState::PendingConsent),
        Just(TransferState::Active),
        Just(TransferState::Completed),
        Just(TransferState::Cancelled),
        Just(TransferState::Failed),
    ]
}

fn item_strategy() -> impl Strategy<Value = TransferItem> {
    (
        (free_text(), free_text()),
        (0u64..=1_000u64, any::<bool>()),
        (prop::option::of(free_text()), prop::option::of(free_text())),
    )
        .prop_map(
            |((name, mime_type), (size_bytes, is_directory), (target_path, checksum))| {
                TransferItem {
                    name,
                    mime_type,
                    size_bytes,
                    is_directory,
                    target_path,
                    checksum,
                }
            },
        )
}

fn session_strategy() -> impl Strategy<Value = TransferSession> {
    (
        (token(), token()),
        (
            direction_strategy(),
            prop::collection::vec(item_strategy(), 0..=3),
        ),
        (0u64..=1_000u64, 0u64..=1_000u64),
        (state_strategy(), numeric_token()),
    )
        .prop_map(
            |(
                (transfer_id, peer_device_id),
                (direction, items),
                (total_bytes, transferred_bytes),
                (state, started_at),
            )| TransferSession {
                transfer_id,
                peer_device_id,
                direction,
                items,
                total_bytes,
                transferred_bytes,
                state,
                started_at,
            },
        )
}

fn device_strategy() -> impl Strategy<Value = PairedDevice> {
    (
        (token(), token()),
        (role_strategy(), free_text()),
        (free_text(), free_text()),
        (free_text(), free_text(), (0u32..=1_000u32, any::<bool>())),
    )
        .prop_map(
            |(
                (device_id, display_name),
                (device_role, static_x25519_pubkey),
                (last_ble_address, last_lan_address),
                (last_endpoint_id, paired_at, (capability_flags, auto_accept_files)),
            )| PairedDevice {
                device_id,
                display_name,
                device_role,
                static_x25519_pubkey,
                last_ble_address,
                last_lan_address,
                last_endpoint_id,
                capability_flags,
                auto_accept_files,
                paired_at,
            },
        )
}

fn status_strategy() -> impl Strategy<Value = DaemonStatus> {
    (
        (token(), token()),
        (prop::collection::vec(token(), 0..=3), visibility_strategy()),
        (any::<bool>(), (0u32..=10u32, 0u32..=10u32)),
        (
            prop::option::of(0u64..=1_000u64),
            (any::<bool>(), free_text()),
        ),
    )
        .prop_map(
            |(
                (device_id, endpoint_id),
                (listen_addrs, visibility_mode),
                (mdns_degraded, (active_transfer_count, paired_device_count)),
                (everyone_remaining_secs, (auto_accept_from_paired, incoming_directory)),
            )| DaemonStatus {
                device_id,
                endpoint_id,
                listen_addrs,
                visibility_mode,
                mdns_degraded,
                active_transfer_count,
                paired_device_count,
                everyone_remaining_secs,
                auto_accept_from_paired,
                incoming_directory,
            },
        )
}

fn request_strategy() -> impl Strategy<Value = IpcRequest> {
    prop_oneof![
        (token(), prop::collection::vec(token(), 1..=3)).prop_map(|(device_id, paths)| {
            IpcRequest::SendFiles {
                device_id,
                paths: paths.into_iter().map(PathBuf::from).collect(),
            }
        }),
        token().prop_map(|transfer_id| IpcRequest::AcceptTransfer { transfer_id }),
        token().prop_map(|transfer_id| IpcRequest::DeclineTransfer { transfer_id }),
        token().prop_map(|transfer_id| IpcRequest::CancelTransfer { transfer_id }),
        Just(IpcRequest::ListTransfers),
        Just(IpcRequest::Status),
        Just(IpcRequest::ListDevices),
        visibility_strategy().prop_map(|mode| IpcRequest::SetVisibility { mode }),
        Just(IpcRequest::GetConfig),
        token().prop_map(|dir| IpcRequest::UpdateConfig {
            config: DaemonConfig {
                incoming_directory: dir,
                ..DaemonConfig::default()
            },
        }),
        (token(), token()).prop_map(|(device_id, display_name)| IpcRequest::RenameDevice {
            device_id,
            display_name,
        }),
        (token(), any::<bool>()).prop_map(|(device_id, enabled)| IpcRequest::SetDeviceAutoAccept {
            device_id,
            enabled
        }),
        token().prop_map(|device_id| IpcRequest::UnpairDevice { device_id }),
        any::<bool>().prop_map(|approve| IpcRequest::RespondPairing { approve }),
    ]
}

fn response_strategy() -> impl Strategy<Value = IpcResponse> {
    prop_oneof![
        session_strategy().prop_map(IpcResponse::Transfer),
        prop::collection::vec(session_strategy(), 0..=3).prop_map(IpcResponse::Transfers),
        status_strategy().prop_map(IpcResponse::Status),
        prop::collection::vec(device_strategy(), 0..=3).prop_map(IpcResponse::Devices),
        device_strategy().prop_map(IpcResponse::Device),
        token().prop_map(|dir| IpcResponse::Config(DaemonConfig {
            incoming_directory: dir,
            ..DaemonConfig::default()
        })),
        token().prop_map(|device_id| IpcResponse::Unpaired { device_id }),
        any::<bool>().prop_map(|approved| IpcResponse::PairingDecision { approved }),
        free_text().prop_map(|message| IpcResponse::Error { message }),
    ]
}

proptest! {
    #[test]
    fn generated_request_roundtrips(request in request_strategy()) {
        let encoded = encode_request(&request).expect("encode request");
        prop_assert_eq!(decode_request(&encoded).expect("decode request"), request);
    }

    #[test]
    fn generated_response_roundtrips(response in response_strategy()) {
        let encoded = encode_response(&response).expect("encode response");
        prop_assert_eq!(decode_response(&encoded).expect("decode response"), response);
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
