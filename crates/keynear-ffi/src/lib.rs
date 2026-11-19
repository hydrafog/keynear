pub mod callbacks;
pub mod clipboard;
pub mod pairing;
pub mod transfer;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use keynear_core::{
    parse_endpoint_id_bytes, ClipboardEngine, ClipboardPolicy, OrionNodeConfig, OrionQuicNode,
    TransferEngine,
};
use keynear_protocol::{read_stream_kind, KeynearError, QuicStreamKind};
use zeroize::Zeroizing;

pub use callbacks::{
    ble_service_uuid16, ble_service_uuid_string, default_config_summary,
    everyone_nearby_timeout_secs, get_core_status, lan_endpoint_char_uuid16,
    lan_endpoint_char_uuid_string, pairing_timeout_secs, CoreStatus, FfiDaemonConfig,
    FfiDeviceRole, FfiError, FfiLanEndpoint, FfiPairedDevice, FfiVisibilityMode,
};
pub use clipboard::FfiClipboardItem;
pub use pairing::{
    FfiEveryoneNearbyBeacon, FfiNoisePairingSession, FfiPairedBeaconState, FfiPairingCompleted,
    FfiPairingRole, FfiPairingState, FfiPairingStep, FfiStaticKeypair,
};
pub use transfer::{FfiTransferDirection, FfiTransferItem, FfiTransferSession, FfiTransferState};

use callbacks::{
    cycle_visibility_mode_impl, generate_static_keypair_impl, load_daemon_config_impl,
    load_paired_devices_impl, remaining_secs_impl, remove_paired_device_impl,
    rename_paired_device_impl, save_daemon_config_impl, save_paired_device_impl,
    set_auto_accept_files_impl, set_visibility_mode_impl, to_key_32, validate_config_toml_impl,
    validate_control_frame_impl, visibility_mode_impl, VisibilityStateInner,
};
use clipboard::{
    fetch_clipboard_offer_impl, poll_incoming_clipboard_impl, send_clipboard_blob_impl,
    send_clipboard_text_impl, set_clipboard_config_impl,
};
use pairing::{
    decode_everyone_nearby_beacon_impl, decode_lan_endpoint_impl, decode_paired_beacon_impl,
    derive_presence_token_bytes_impl, encode_everyone_nearby_beacon_impl, encode_lan_endpoint_impl,
    encode_paired_beacon_impl,
};
use transfer::send_paths_impl;
#[cfg(unix)]
use transfer::{provide_receive_fd_impl, send_fd_impl, SendFdParams};

uniffi::setup_scaffolding!();

#[derive(Debug, Default, uniffi::Object)]
pub struct CoreHandle {
    visibility: Mutex<VisibilityStateInner>,
}

#[uniffi::export]
impl CoreHandle {
    #[uniffi::constructor]
    pub fn new() -> Self {
        Self {
            visibility: Mutex::new(VisibilityStateInner::default()),
        }
    }

    pub fn status(&self) -> CoreStatus {
        get_core_status()
    }

    pub fn visibility_mode(&self) -> FfiVisibilityMode {
        visibility_mode_impl(&self.visibility)
    }

    pub fn set_visibility_mode(&self, mode: FfiVisibilityMode) -> FfiVisibilityMode {
        set_visibility_mode_impl(&self.visibility, mode)
    }

    pub fn cycle_visibility_mode(&self) -> FfiVisibilityMode {
        cycle_visibility_mode_impl(&self.visibility)
    }

    pub fn everyone_nearby_remaining_secs(&self) -> u64 {
        remaining_secs_impl(&self.visibility)
    }

    pub fn validate_config_toml(&self, raw_toml: String) -> Result<FfiVisibilityMode, FfiError> {
        validate_config_toml_impl(raw_toml)
    }

    pub fn load_daemon_config(
        &self,
        config_toml_path: String,
    ) -> Result<FfiDaemonConfig, FfiError> {
        load_daemon_config_impl(&self.visibility, config_toml_path)
    }

    pub fn save_daemon_config(
        &self,
        config_toml_path: String,
        config: FfiDaemonConfig,
    ) -> Result<FfiDaemonConfig, FfiError> {
        save_daemon_config_impl(&self.visibility, config_toml_path, config)
    }

    pub fn validate_control_frame(&self, bytes: Vec<u8>) -> Result<String, FfiError> {
        validate_control_frame_impl(bytes)
    }

    pub fn generate_static_keypair(&self) -> FfiStaticKeypair {
        let (private_key, public_key_hex) = generate_static_keypair_impl();
        FfiStaticKeypair {
            private_key,
            public_key_hex,
        }
    }

    pub fn encode_everyone_nearby_beacon(
        &self,
        role: FfiDeviceRole,
        capability_flags: u32,
        display_name: String,
    ) -> Result<Vec<u8>, FfiError> {
        encode_everyone_nearby_beacon_impl(role, capability_flags, display_name)
    }

    pub fn decode_everyone_nearby_beacon(
        &self,
        service_data: Vec<u8>,
    ) -> Result<FfiEveryoneNearbyBeacon, FfiError> {
        decode_everyone_nearby_beacon_impl(service_data)
    }

    pub fn encode_paired_beacon(
        &self,
        device_id: String,
        counter: u32,
        reconnect_key: Vec<u8>,
        available: bool,
        battery_level: Option<u8>,
    ) -> Result<Vec<u8>, FfiError> {
        encode_paired_beacon_impl(device_id, counter, reconnect_key, available, battery_level)
    }

    pub fn decode_paired_beacon(
        &self,
        service_data: Vec<u8>,
        device_id: String,
        expected_counter: u32,
        last_accepted_counter: Option<u32>,
        reconnect_key: Vec<u8>,
    ) -> Result<FfiPairedBeaconState, FfiError> {
        decode_paired_beacon_impl(
            service_data,
            device_id,
            expected_counter,
            last_accepted_counter,
            reconnect_key,
        )
    }

    pub fn derive_presence_token_bytes(
        &self,
        device_id: String,
        counter: u32,
        reconnect_key: Vec<u8>,
    ) -> Result<Vec<u8>, FfiError> {
        derive_presence_token_bytes_impl(device_id, counter, reconnect_key)
    }

    pub fn load_paired_devices(
        &self,
        devices_toml_path: String,
    ) -> Result<Vec<FfiPairedDevice>, FfiError> {
        load_paired_devices_impl(devices_toml_path)
    }

    pub fn save_paired_device(
        &self,
        devices_toml_path: String,
        device: FfiPairedDevice,
    ) -> Result<(), FfiError> {
        save_paired_device_impl(devices_toml_path, device)
    }

    pub fn remove_paired_device(
        &self,
        devices_toml_path: String,
        device_id: String,
    ) -> Result<bool, FfiError> {
        remove_paired_device_impl(devices_toml_path, device_id)
    }

    pub fn set_auto_accept_files(
        &self,
        devices_toml_path: String,
        device_id: String,
        enabled: bool,
    ) -> Result<FfiPairedDevice, FfiError> {
        set_auto_accept_files_impl(devices_toml_path, device_id, enabled)
    }

    pub fn rename_paired_device(
        &self,
        devices_toml_path: String,
        device_id: String,
        new_display_name: String,
    ) -> Result<FfiPairedDevice, FfiError> {
        rename_paired_device_impl(devices_toml_path, device_id, new_display_name)
    }

    pub fn encode_lan_endpoint(
        &self,
        endpoint: FfiLanEndpoint,
        reconnect_key: Vec<u8>,
    ) -> Result<Vec<u8>, FfiError> {
        encode_lan_endpoint_impl(endpoint, reconnect_key)
    }

    pub fn decode_lan_endpoint(
        &self,
        encoded: Vec<u8>,
        expected_device_id: String,
        reconnect_key: Vec<u8>,
    ) -> Result<FfiLanEndpoint, FfiError> {
        decode_lan_endpoint_impl(encoded, expected_device_id, reconnect_key)
    }
}

#[derive(uniffi::Object)]
pub struct OrionNodeHandle {
    runtime: Arc<tokio::runtime::Runtime>,
    node: OrionQuicNode,
    engine: Arc<TransferEngine>,
    clipboard: Arc<ClipboardEngine>,
    synced_peers: Mutex<Vec<String>>,
    accept_task: Mutex<Option<tokio::task::JoinHandle<()>>>,
}

struct OrionNodeInit {
    device_id: String,
    display_name: String,
    capability_flags: u32,
    quic_secret_key: Vec<u8>,
    incoming_directory: String,
    auto_accept_from_paired: bool,
    require_explicit_fd_for_single_file: bool,
    bind_addr: Option<SocketAddr>,
}

impl OrionNodeHandle {
    fn build_with_init(init: OrionNodeInit) -> Result<Self, FfiError> {
        let key_bytes = to_key_32(&init.quic_secret_key, "quicSecretKey")?;
        let runtime = Arc::new(
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
                .map_err(|err| KeynearError::Transport(format!("tokio runtime failed: {err}")))?,
        );
        let config = OrionNodeConfig {
            device_id: init.device_id,
            display_name: init.display_name,
            capability_flags: init.capability_flags,
            quic_secret_key: Zeroizing::new(key_bytes),
        };
        let node = runtime.block_on(OrionQuicNode::bind(config, init.bind_addr))?;
        let engine = Arc::new(TransferEngine::new(
            PathBuf::from(init.incoming_directory),
            init.auto_accept_from_paired,
        ));
        engine.set_require_explicit_fd_for_single_file(init.require_explicit_fd_for_single_file);
        let clipboard = Arc::new(ClipboardEngine::new(ClipboardPolicy::default()));

        let accept_node = node.clone();
        let accept_engine = Arc::clone(&engine);
        let accept_clipboard = Arc::clone(&clipboard);
        let handle = runtime.spawn(async move {
            loop {
                match accept_node.accept_authenticated().await {
                    Ok(Some((peer_id, conn))) => {
                        let eng = Arc::clone(&accept_engine);
                        let clip = Arc::clone(&accept_clipboard);
                        tokio::spawn(async move {
                            while let Ok((send, mut recv)) = conn.accept_bi().await {
                                match read_stream_kind(&mut recv).await {
                                    Ok(
                                        kind @ (QuicStreamKind::RawFileChunk
                                        | QuicStreamKind::StoredZipArchive),
                                    ) => {
                                        let _ = eng
                                            .handle_incoming_stream_with_kind(
                                                &peer_id, kind, send, recv,
                                            )
                                            .await;
                                    }
                                    Ok(
                                        kind @ (QuicStreamKind::Control
                                        | QuicStreamKind::ClipboardBlob),
                                    ) => {
                                        let _ = clip
                                            .handle_incoming_stream_with_kind(
                                                &peer_id, kind, send, recv,
                                            )
                                            .await;
                                    }
                                    Err(_) => break,
                                }
                            }
                        });
                    }
                    Ok(None) => break,
                    Err(_) => continue,
                }
            }
        });

        Ok(Self {
            runtime,
            node,
            engine,
            clipboard,
            synced_peers: Mutex::new(Vec::new()),
            accept_task: Mutex::new(Some(handle)),
        })
    }
}

#[uniffi::export]
impl OrionNodeHandle {
    #[uniffi::constructor]
    pub fn new(
        device_id: String,
        display_name: String,
        capability_flags: u32,
        quic_secret_key: Vec<u8>,
        incoming_directory: String,
        auto_accept_from_paired: bool,
        require_explicit_fd_for_single_file: bool,
    ) -> Result<Self, FfiError> {
        Self::build_with_init(OrionNodeInit {
            device_id,
            display_name,
            capability_flags,
            quic_secret_key,
            incoming_directory,
            auto_accept_from_paired,
            require_explicit_fd_for_single_file,
            bind_addr: None,
        })
    }

    pub fn local_lan_endpoint(&self) -> FfiLanEndpoint {
        FfiLanEndpoint::from(self.node.local_lan_endpoint())
    }

    pub fn sync_peer(
        &self,
        device_id: String,
        endpoint_id: Vec<u8>,
        socket_addrs: Vec<String>,
        reconnect_secret: Vec<u8>,
        auto_accept_files: bool,
    ) -> Result<(), FfiError> {
        let eid = parse_endpoint_id_bytes(&endpoint_id)?;
        let secret = to_key_32(&reconnect_secret, "reconnectSecret")?;
        let parsed_addrs: Vec<SocketAddr> = socket_addrs
            .iter()
            .filter_map(|s| s.trim().parse::<SocketAddr>().ok())
            .collect();
        self.node
            .register_peer(&device_id, eid, parsed_addrs, secret)?;
        self.engine
            .set_peer_auto_accept(&device_id, auto_accept_files)?;
        let mut peers = self.synced_peers.lock().unwrap_or_else(|p| p.into_inner());
        if !peers.contains(&device_id) {
            peers.push(device_id);
        }
        Ok(())
    }

    pub fn remove_peer(&self, device_id: String) -> Result<(), FfiError> {
        self.node.remove_peer(&device_id)?;
        self.engine.remove_peer_policy(&device_id);
        let mut peers = self.synced_peers.lock().unwrap_or_else(|p| p.into_inner());
        peers.retain(|id| id != &device_id);
        Ok(())
    }

    pub fn set_auto_accept_from_paired(&self, enabled: bool) {
        self.engine.set_auto_accept_from_paired(enabled);
    }

    pub fn set_incoming_directory(&self, dir: String) -> Result<(), FfiError> {
        self.engine
            .set_incoming_directory(PathBuf::from(dir))
            .map_err(FfiError::from)
    }

    pub fn list_transfers(&self) -> Vec<FfiTransferSession> {
        self.engine
            .list_transfers()
            .into_iter()
            .map(FfiTransferSession::from)
            .collect()
    }

    pub fn respond_consent(
        &self,
        transfer_id: String,
        accepted: bool,
    ) -> Result<FfiTransferSession, FfiError> {
        let session = self.engine.respond_consent(&transfer_id, accepted)?;
        Ok(FfiTransferSession::from(session))
    }

    pub fn cancel_transfer(&self, transfer_id: String) -> Result<FfiTransferSession, FfiError> {
        let session = self.engine.cancel_transfer(&transfer_id)?;
        Ok(FfiTransferSession::from(session))
    }

    pub fn send_paths(
        &self,
        peer_device_id: String,
        paths: Vec<String>,
    ) -> Result<FfiTransferSession, FfiError> {
        send_paths_impl(
            &self.runtime,
            &self.engine,
            &self.node,
            peer_device_id,
            paths,
        )
    }

    #[cfg(unix)]
    pub fn send_fd(
        &self,
        peer_device_id: String,
        file_name: String,
        mime_type: String,
        size_bytes: u64,
        raw_fd: i32,
    ) -> Result<FfiTransferSession, FfiError> {
        send_fd_impl(
            &self.runtime,
            &self.engine,
            &self.node,
            SendFdParams {
                peer_device_id,
                file_name,
                mime_type,
                size_bytes,
                raw_fd,
            },
        )
    }

    #[cfg(unix)]
    pub fn provide_receive_fd(&self, transfer_id: String, raw_fd: i32) -> Result<(), FfiError> {
        provide_receive_fd_impl(&self.engine, transfer_id, raw_fd)
    }

    pub fn set_clipboard_config(
        &self,
        clipboard_sync_enabled: bool,
        clipboard_images_enabled: bool,
        otp_passthrough_enabled: bool,
        ignored_mime_types: Vec<String>,
    ) {
        set_clipboard_config_impl(
            &self.clipboard,
            clipboard_sync_enabled,
            clipboard_images_enabled,
            otp_passthrough_enabled,
            ignored_mime_types,
        );
    }

    pub fn send_clipboard_text(&self, text: String, is_otp: bool) -> Result<(), FfiError> {
        send_clipboard_text_impl(
            &self.runtime,
            &self.clipboard,
            &self.node,
            &self.synced_peers,
            text,
            is_otp,
        )
    }

    pub fn send_clipboard_blob(&self, mime_type: String, data: Vec<u8>) -> Result<(), FfiError> {
        send_clipboard_blob_impl(
            &self.runtime,
            &self.clipboard,
            &self.node,
            &self.synced_peers,
            mime_type,
            data,
        )
    }

    pub fn poll_incoming_clipboard(&self) -> Vec<FfiClipboardItem> {
        poll_incoming_clipboard_impl(&self.clipboard)
    }

    pub fn fetch_clipboard_offer(
        &self,
        peer_device_id: String,
        offer_id: String,
        mime_type: String,
    ) -> Result<FfiClipboardItem, FfiError> {
        fetch_clipboard_offer_impl(
            &self.runtime,
            &self.clipboard,
            &self.node,
            peer_device_id,
            offer_id,
            mime_type,
        )
    }

    pub fn shutdown(&self) {
        if let Some(task) = self
            .accept_task
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .take()
        {
            task.abort();
        }
        self.runtime.block_on(self.node.close());
    }
}

impl Drop for OrionNodeHandle {
    fn drop(&mut self) {
        if let Some(task) = self
            .accept_task
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .take()
        {
            task.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::Arc;
    use std::thread;
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

    use keynear_protocol::{
        encode_control_frame, envelope_to_control_frame, v1, ControlFrame, VisibilityMode,
        MAX_CONTROL_FRAME_BYTES,
    };
    use prost::Message;

    use super::*;
    use transfer::wrap_raw_fd;

    #[test]
    fn core_status_reports_ready() {
        let handle = CoreHandle::new();
        let status = handle.status();
        assert!(status.ready);
        assert_eq!(
            status.max_control_frame_bytes,
            MAX_CONTROL_FRAME_BYTES as u32
        );
        assert_eq!(handle.visibility_mode(), FfiVisibilityMode::PairedOnly);
        assert_eq!(ble_service_uuid16(), 0xFE6D);
        assert_eq!(everyone_nearby_timeout_secs(), 600);
        assert_eq!(pairing_timeout_secs(), 120);
    }

    #[test]
    fn core_handle_cycles_visibility_thread_safely() {
        let handle = Arc::new(CoreHandle::new());
        let mut workers = Vec::new();
        for _ in 0..6 {
            let shared = Arc::clone(&handle);
            workers.push(thread::spawn(move || {
                shared.cycle_visibility_mode();
            }));
        }
        for worker in workers {
            worker.join().expect("thread completes");
        }
        assert_eq!(handle.visibility_mode(), FfiVisibilityMode::PairedOnly);
    }

    #[test]
    fn everyone_nearby_deadline_auto_reverts_to_paired_only() {
        let now = Instant::now();
        let mut inner = VisibilityStateInner::default();
        inner.set_mode(VisibilityMode::EveryoneNearby, now);
        assert_eq!(inner.remaining_secs(now + Duration::from_secs(1)), 599);

        let expired_remaining = inner.remaining_secs(now + Duration::from_secs(600));
        assert_eq!(expired_remaining, 0);
        assert_eq!(inner.mode, VisibilityMode::PairedOnly);
    }

    #[test]
    fn ffi_beacons_and_noise_pairing_roundtrip() {
        let handle = CoreHandle::new();
        let beacon_bytes = handle
            .encode_everyone_nearby_beacon(FfiDeviceRole::Mobile, 0x03, "Pixel 9".to_string())
            .expect("encode everyone nearby");
        let decoded_beacon = handle
            .decode_everyone_nearby_beacon(beacon_bytes)
            .expect("decode everyone nearby");
        assert_eq!(decoded_beacon.role, FfiDeviceRole::Mobile);
        assert_eq!(decoded_beacon.display_name, "Pixel 9");

        let init_kp = handle.generate_static_keypair();
        let resp_kp = handle.generate_static_keypair();

        let initiator = FfiNoisePairingSession::new_initiator(
            init_kp.private_key,
            "phone-1".to_string(),
            "Pixel 9".to_string(),
            FfiDeviceRole::Mobile,
            0x03,
        )
        .expect("init session");
        let responder = FfiNoisePairingSession::new_responder(
            resp_kp.private_key,
            "linux-1".to_string(),
            "Workstation".to_string(),
            FfiDeviceRole::Desktop,
            0x07,
        )
        .expect("resp session");

        let m1 = initiator.start_handshake().expect("m1");
        let m2 = responder
            .step(m1.outbound_frame.expect("m1 bytes"))
            .expect("m2");
        let m3 = initiator
            .step(m2.outbound_frame.expect("m2 bytes"))
            .expect("m3");
        let fin = responder
            .step(m3.outbound_frame.expect("m3 bytes"))
            .expect("m3 read");
        assert_eq!(m3.state, FfiPairingState::AwaitingConfirmation);
        assert_eq!(fin.state, FfiPairingState::AwaitingConfirmation);
        assert_eq!(
            initiator.formatted_verification_code().expect("init code"),
            responder.formatted_verification_code().expect("resp code")
        );

        let init_export = initiator.approve_and_build_export().expect("init export");
        let resp_export = responder.approve_and_build_export().expect("resp export");
        let init_done = initiator
            .complete_with_peer_export(resp_export)
            .expect("init done");
        let resp_done = responder
            .complete_with_peer_export(init_export)
            .expect("resp done");
        assert_eq!(init_done.reconnect_secret, resp_done.reconnect_secret);

        let key_debug = format!("{init_done:?}");
        assert!(key_debug.contains("[REDACTED]"));
        assert!(!key_debug.contains(&hex::encode(&init_done.reconnect_secret)));

        let paired_bytes = handle
            .encode_paired_beacon(
                "phone-1".to_string(),
                5,
                init_done.reconnect_secret.clone(),
                true,
                Some(88),
            )
            .expect("encode paired beacon");
        let decoded_paired = handle
            .decode_paired_beacon(
                paired_bytes,
                "phone-1".to_string(),
                5,
                None,
                init_done.reconnect_secret,
            )
            .expect("decode paired beacon");
        assert_eq!(decoded_paired.counter, 5);
        assert!(decoded_paired.available);
        assert_eq!(decoded_paired.battery_level, Some(88));
    }

    #[test]
    fn ffi_paired_device_store_roundtrip() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("keynear-uniffi-test-{nonce}"));
        fs::create_dir_all(&dir).expect("tempdir");
        let path = dir.join("devices.toml").display().to_string();
        let handle = CoreHandle::new();

        handle
            .save_paired_device(
                path.clone(),
                FfiPairedDevice {
                    device_id: "linux-1".to_string(),
                    display_name: "Workstation".to_string(),
                    device_role: FfiDeviceRole::Desktop,
                    static_x25519_pubkey: "ab".repeat(32),
                    last_ble_address: "AA:BB:CC:DD:EE:FF".to_string(),
                    last_lan_address: String::new(),
                    last_endpoint_id: String::new(),
                    capability_flags: 0x03,
                    auto_accept_files: false,
                    paired_at: "2026-11-13T20:00:00Z".to_string(),
                },
            )
            .expect("save device");

        let loaded = handle
            .load_paired_devices(path.clone())
            .expect("load devices");
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].device_id, "linux-1");
        assert!(handle
            .remove_paired_device(path.clone(), "linux-1".to_string())
            .expect("remove"));
        assert!(handle
            .load_paired_devices(path)
            .expect("empty after remove")
            .is_empty());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn validate_config_toml_checks_schema_and_validation() {
        let handle = CoreHandle::new();
        let mode = handle
            .validate_config_toml("visibilityMode = \"everyoneNearby\"\n".to_string())
            .expect("valid toml succeeds");
        assert_eq!(mode, FfiVisibilityMode::EveryoneNearby);

        let err = handle
            .validate_config_toml("incomingDirectory = \"\"\n".to_string())
            .expect_err("empty incoming directory fails");
        assert!(matches!(err, FfiError::Validation { .. }));
    }

    #[test]
    fn validate_control_frame_checks_envelope_and_size() {
        let handle = CoreHandle::new();
        let env = v1::Envelope {
            version: 1,
            sequence: 11,
            timestamp_ms: 1_763_000_000_000,
            payload: Some(v1::envelope::Payload::ControlAck(v1::ControlAck {
                acked_sequence: 10,
                accepted: true,
                reason: String::new(),
            })),
        };
        let frame = envelope_to_control_frame(&env).expect("envelope to frame");
        let bytes = encode_control_frame(&frame).expect("encode frame");
        let kind = handle
            .validate_control_frame(bytes)
            .expect("valid control frame accepted");
        assert_eq!(kind, "control_ack");

        let oversized = vec![0u8; MAX_CONTROL_FRAME_BYTES + 1];
        assert!(matches!(
            handle.validate_control_frame(oversized),
            Err(FfiError::FrameTooLarge { .. })
        ));

        let empty_payload = v1::ControlAck {
            acked_sequence: 1,
            accepted: true,
            reason: String::new(),
        }
        .encode_to_vec();
        let bad_kind = ControlFrame {
            version: 1,
            sequence: 1,
            timestamp_ms: 1,
            kind: "invalid".to_string(),
            payload: empty_payload,
        };
        let bad_bytes = encode_control_frame(&bad_kind).expect("encode bad kind frame");
        assert!(matches!(
            handle.validate_control_frame(bad_bytes),
            Err(FfiError::Protocol { .. })
        ));
    }

    #[cfg(unix)]
    #[test]
    fn owned_fd_closes_once_on_drop() {
        use std::os::fd::FromRawFd;
        use std::os::fd::IntoRawFd;

        assert!(matches!(
            wrap_raw_fd(-1),
            Err(KeynearError::Validation { field: "rawFd", .. })
        ));

        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("keynear-fd-test-{nonce}.bin"));
        let file = fs::File::create(&path).expect("create temp file");
        let raw_fd = file.into_raw_fd();

        {
            let _owned = wrap_raw_fd(raw_fd).expect("wrap valid raw fd");
        }

        let reopened = unsafe { fs::File::from_raw_fd(raw_fd) };
        let err = reopened.sync_all().expect_err("fd must already be closed");
        std::mem::forget(reopened);
        assert_eq!(err.raw_os_error(), Some(9));
        let _ = fs::remove_file(&path);
    }

    #[cfg(unix)]
    #[test]
    fn orion_node_handle_transfers_via_fd_and_cleans_up() {
        use std::io::Write;
        use std::net::{IpAddr, Ipv4Addr};
        use std::os::fd::IntoRawFd;

        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("keynear-orion-ffi-{nonce}"));
        let recv_dir = dir.join("recv");
        fs::create_dir_all(&recv_dir).expect("mkdir");

        let loopback = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
        let receiver = OrionNodeHandle::build_with_init(OrionNodeInit {
            device_id: "rx-node".to_string(),
            display_name: "Receiver".to_string(),
            capability_flags: 1,
            quic_secret_key: vec![0x71; 32],
            incoming_directory: recv_dir.to_string_lossy().to_string(),
            auto_accept_from_paired: false,
            require_explicit_fd_for_single_file: true,
            bind_addr: Some(loopback),
        })
        .expect("bind receiver");

        let sender = OrionNodeHandle::build_with_init(OrionNodeInit {
            device_id: "tx-node".to_string(),
            display_name: "Sender".to_string(),
            capability_flags: 1,
            quic_secret_key: vec![0x72; 32],
            incoming_directory: dir.to_string_lossy().to_string(),
            auto_accept_from_paired: false,
            require_explicit_fd_for_single_file: false,
            bind_addr: Some(loopback),
        })
        .expect("bind sender");

        let secret = vec![0x9A; 32];
        let rx_ep = receiver.local_lan_endpoint();
        let tx_ep = sender.local_lan_endpoint();

        receiver
            .sync_peer(
                "tx-node".to_string(),
                tx_ep.endpoint_id,
                tx_ep.socket_addrs,
                secret.clone(),
                true,
            )
            .expect("rx sync");
        sender
            .sync_peer(
                "rx-node".to_string(),
                rx_ep.endpoint_id,
                rx_ep.socket_addrs,
                secret,
                false,
            )
            .expect("tx sync");

        let src_path = dir.join("send.txt");
        let out_path = recv_dir.join("received.txt");
        let payload = b"zero-copy OwnedFd UniFFI stream";
        {
            let mut f = fs::File::create(&src_path).expect("create src");
            f.write_all(payload).expect("write src");
        }
        let send_file = fs::File::open(&src_path).expect("open src");
        let send_raw_fd = send_file.into_raw_fd();

        let tx_thread = thread::spawn(move || {
            let res = sender.send_fd(
                "rx-node".to_string(),
                "send.txt".to_string(),
                "text/plain".to_string(),
                payload.len() as u64,
                send_raw_fd,
            );
            sender.shutdown();
            res
        });

        let transfer_id = loop {
            if let Some(session) = receiver
                .list_transfers()
                .into_iter()
                .find(|s| s.state == FfiTransferState::Active)
            {
                break session.transfer_id;
            }
            thread::sleep(Duration::from_millis(10));
        };

        let out_file = fs::File::create(&out_path).expect("create out");
        receiver
            .provide_receive_fd(transfer_id.clone(), out_file.into_raw_fd())
            .expect("provide fd");

        let sent = tx_thread.join().expect("join tx").expect("send_fd ok");
        assert_eq!(sent.state, FfiTransferState::Completed);

        loop {
            if let Some(session) = receiver
                .list_transfers()
                .into_iter()
                .find(|s| s.transfer_id == transfer_id)
            {
                if session.state == FfiTransferState::Completed {
                    break;
                }
            }
            thread::sleep(Duration::from_millis(10));
        }

        assert_eq!(fs::read(&out_path).expect("read received"), payload);
        receiver.shutdown();
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rename_paired_device_updates_store_and_validates_input() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("keynear-uniffi-rename-{nonce}"));
        fs::create_dir_all(&dir).expect("mkdir");
        let devices_path = dir.join("devices.toml");
        let devices_str = devices_path.to_string_lossy().to_string();

        let handle = CoreHandle::new();
        assert_eq!(get_core_status().version, "0.5.0");

        let seeded = FfiPairedDevice {
            device_id: "phone-1".to_string(),
            display_name: "Pixel".to_string(),
            device_role: FfiDeviceRole::Mobile,
            static_x25519_pubkey: "abcd".to_string(),
            last_ble_address: "11:22:33:44:55:66".to_string(),
            last_lan_address: String::new(),
            last_endpoint_id: String::new(),
            capability_flags: 1,
            auto_accept_files: false,
            paired_at: "2026-11-14T00:00:00Z".to_string(),
        };
        handle
            .save_paired_device(devices_str.clone(), seeded)
            .expect("save device");

        let updated = handle
            .rename_paired_device(
                devices_str.clone(),
                "phone-1".to_string(),
                "Pixel Daily".to_string(),
            )
            .expect("rename via uniffi");
        assert_eq!(updated.display_name, "Pixel Daily");

        assert!(handle
            .rename_paired_device(devices_str, "phone-1".to_string(), "   ".to_string())
            .is_err());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn uniffi_bridge_daemon_config_roundtrips_all_prototype_fields() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("keynear-uniffi-cfg-{nonce}"));
        fs::create_dir_all(&dir).expect("mkdir");
        let cfg_path = dir.join("config.toml");
        let cfg_str = cfg_path.to_string_lossy().to_string();

        let handle = CoreHandle::new();
        let initial = handle
            .load_daemon_config(cfg_str.clone())
            .expect("load default daemon config");
        assert_eq!(initial.visibility_mode, FfiVisibilityMode::PairedOnly);
        assert!(initial.clipboard_sync_enabled);

        let updated = FfiDaemonConfig {
            visibility_mode: FfiVisibilityMode::EveryoneNearby,
            incoming_directory: "Downloads/".to_string(),
            auto_accept_from_paired: true,
            clipboard_sync_enabled: false,
            clipboard_images_enabled: false,
            otp_passthrough_enabled: false,
            audio_handoff_enabled: false,
            ignored_mime_types: vec![
                "application/x-secret".to_string(),
                "  x-kde-passwordManagerHint  ".to_string(),
            ],
            notify_auto_accept_complete: false,
        };
        let saved = handle
            .save_daemon_config(cfg_str.clone(), updated)
            .expect("save daemon config");
        assert_eq!(
            saved.ignored_mime_types,
            vec![
                "application/x-secret".to_string(),
                "x-kde-passwordManagerHint".to_string(),
            ]
        );
        assert_eq!(handle.visibility_mode(), FfiVisibilityMode::EveryoneNearby);

        let reloaded = handle
            .load_daemon_config(cfg_str)
            .expect("reload daemon config");
        assert_eq!(reloaded, saved);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn orion_node_handle_demuxes_clipboard_and_enforces_config() {
        use std::net::{IpAddr, Ipv4Addr};

        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("keynear-orion-clip-{nonce}"));
        fs::create_dir_all(&dir).expect("mkdir");

        let loopback = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
        let linux_node = OrionNodeHandle::build_with_init(OrionNodeInit {
            device_id: "linux-host".to_string(),
            display_name: "Linux".to_string(),
            capability_flags: 1,
            quic_secret_key: vec![0x31; 32],
            incoming_directory: dir.to_string_lossy().to_string(),
            auto_accept_from_paired: true,
            require_explicit_fd_for_single_file: false,
            bind_addr: Some(loopback),
        })
        .expect("bind linux");

        let phone_node = OrionNodeHandle::build_with_init(OrionNodeInit {
            device_id: "pixel-phone".to_string(),
            display_name: "Pixel".to_string(),
            capability_flags: 1,
            quic_secret_key: vec![0x32; 32],
            incoming_directory: dir.to_string_lossy().to_string(),
            auto_accept_from_paired: true,
            require_explicit_fd_for_single_file: false,
            bind_addr: Some(loopback),
        })
        .expect("bind phone");

        let secret = vec![0x88; 32];
        let linux_ep = linux_node.local_lan_endpoint();
        let phone_ep = phone_node.local_lan_endpoint();

        linux_node
            .sync_peer(
                "pixel-phone".to_string(),
                phone_ep.endpoint_id,
                phone_ep.socket_addrs,
                secret.clone(),
                true,
            )
            .expect("sync phone");
        phone_node
            .sync_peer(
                "linux-host".to_string(),
                linux_ep.endpoint_id,
                linux_ep.socket_addrs,
                secret,
                true,
            )
            .expect("sync linux");

        linux_node
            .send_clipboard_text("hello from desktop".to_string(), false)
            .expect("send clipboard text");

        let mut received_phone = Vec::new();
        for _ in 0..50 {
            received_phone.extend(phone_node.poll_incoming_clipboard());
            if !received_phone.is_empty() {
                break;
            }
            thread::sleep(Duration::from_millis(20));
        }
        assert_eq!(received_phone.len(), 1);
        assert_eq!(received_phone[0].data, b"hello from desktop");
        assert!(!received_phone[0].is_otp);

        phone_node
            .send_clipboard_text("847291".to_string(), true)
            .expect("send OTP");

        let mut received_linux = Vec::new();
        for _ in 0..50 {
            received_linux.extend(linux_node.poll_incoming_clipboard());
            if !received_linux.is_empty() {
                break;
            }
            thread::sleep(Duration::from_millis(20));
        }
        assert_eq!(received_linux.len(), 1);
        assert_eq!(received_linux[0].data, b"847291");
        assert!(received_linux[0].is_otp);

        let large_img = vec![0x5Au8; 64 * 1024];
        let offer = linux_node
            .runtime
            .block_on(linux_node.clipboard.offer_clipboard_image(
                &linux_node.node,
                "pixel-phone",
                vec!["image/png".to_string()],
                Arc::new({
                    let data = large_img.clone();
                    move |_| Ok(data.clone())
                }),
            ))
            .expect("offer delayed image");

        let mut phone_offers = Vec::new();
        for _ in 0..50 {
            phone_offers.extend(phone_node.poll_incoming_clipboard());
            if !phone_offers.is_empty() {
                break;
            }
            thread::sleep(Duration::from_millis(20));
        }
        assert_eq!(phone_offers.len(), 1);
        assert!(phone_offers[0].is_image_offer);
        assert!(phone_offers[0].data.is_empty());

        let fetched = phone_node
            .fetch_clipboard_offer(
                "linux-host".to_string(),
                offer.offer_id,
                "image/png".to_string(),
            )
            .expect("fetch delayed offer");
        assert_eq!(fetched.data, large_img);

        phone_node.set_clipboard_config(
            true,
            true,
            false,
            vec!["application/x-password".to_string()],
        );
        let otp_err = phone_node
            .send_clipboard_text("123456".to_string(), true)
            .expect_err("disabled otp passthrough must reject");
        assert!(matches!(otp_err, FfiError::Validation { .. }));

        linux_node.shutdown();
        phone_node.shutdown();
        let _ = fs::remove_dir_all(&dir);
    }
}
