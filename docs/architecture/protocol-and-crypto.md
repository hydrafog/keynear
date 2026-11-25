# Protocol and Cryptography

Keynear discovery, pairing, and continuity transport run peer-to-peer. They need no cloud account and no relay server.

## Pairing and Transport Flow

```mermaid
sequenceDiagram
  participant L as Linux Daemon
  participant A as Android App
  L->>A: BLE 0xFE6D Beacon (EveryoneNearby or PairedOnly 24-byte token)
  A->>L: Noise_XX_25519_ChaChaPoly_BLAKE2s over BLE GATT (0xFE6E / 0xFE6F)
  L-->>A: 6-digit verification code match (120s timeout)
  A-->>L: SessionExportContribution saved to Keystore / oo7
  L<-->A: LanEndpoint via mdns-sd (_keynear._udp.) or encrypted BLE 0xFE70
  L<-->A: Direct iroh QUIC (RelayMode::Disabled, rustls X25519MLKEM768) + Envelope streams
```

## Proximity Discovery and Pairing

Keynear uses the following BLE discovery and pairing rules.

- Keynear advertises and filters on the 16-bit BLE Service UUID `0xFE6D` (`0000FE6D-0000-1000-8000-00805F9B34FB`). The GATT Pairing TX characteristic `0xFE6E` supports `Write` and `WriteWithoutResponse`. The Pairing RX characteristic `0xFE6F` supports `Notify` up to `MAX_GATT_PAIRING_FRAME_BYTES = 512`.
- Beacons stay within `MAX_BLE_SERVICE_DATA_BYTES = 24`. Their contents depend on the visibility mode.
  - Everyone nearby carries the protocol version, device role, visibility flag, truncated UTF-8 display name, and capability bitmask for open pairing discovery. It reverts to paired devices only after `600` seconds.
  - Paired devices only carries a 6-byte rotating HMAC-SHA256 presence token (`device_id || u32 counter` keyed by the pairwise `irk`), a 4-byte little-endian `u32` counter, and a 14-byte AES-256-GCM encrypted state payload. The payload holds `availability` and optional `battery_level`, authenticated with `reconnect_secret` and a deterministic 96-bit nonce.
- Initial pairing runs `Noise_XX_25519_ChaChaPoly_BLAKE2s` (`snow`) over BLE GATT. After the handshake, both sides read the first four bytes of the Noise handshake hash as a big-endian `u32`, reduce it modulo `1_000_000`, and format the result as a 6-digit verification code (`000000`–`999999`). Both users must confirm the code within `120` seconds before the devices commit `SessionExportContribution` to `PairedDeviceStore` and OS secret storage. The contribution carries:
  - `device_id` and `display_name`.
  - `role` and `capability_flags`.
  - `irk` and `reconnect_secret`.

## Relay-Free P2P QUIC and LAN Discovery

Keynear keeps all data traffic on direct peer addresses.

- The `OrionQuicNode` in `crates/keynear-core/src/transport/quic.rs` constructs an `iroh` endpoint from `Endpoint::builder(presets::Empty)`. It sets `RelayMode::Disabled` and uses no DNS or pkarr discovery. All traffic stays on local or direct peer addresses under ALPN `keynear/orion/1`.
- A custom `rustls::crypto::CryptoProvider` pins the hybrid post-quantum `X25519MLKEM768` (RFC 10024) key exchange. It prioritizes `TLS13_AES_256_GCM_SHA384` and `TLS13_CHACHA20_POLY1305_SHA256` ahead of `TLS13_AES_128_GCM_SHA256`.
- `PairedPeerHooks::after_handshake` rejects connections from `EndpointId`s absent from the paired allowlist. After QUIC connection setup, both peers exchange a `HandshakePayload` containing `auth_proof`. The proof is a 32-byte `HMAC-SHA256` tag with these inputs.
  - The pairwise `reconnect_secret` as the key.
  - `"keynear-orion-handshake-v1" || local_endpoint_id || remote_endpoint_id || local_device_id` as the message.

  The receiver verifies it in constant time through `subtle::ConstantTimeEq`.
- Ephemeral `LanEndpoint` records propagate over `mdns-sd` (`_keynear._udp.local.` on Linux, `NsdManager` `_keynear._udp.` on Android) and over the encrypted BLE GATT characteristic `0xFE70` (`LAN_ENDPOINT_CHAR_UUID`). `LanEndpointCodec` encrypts `LanEndpoint` protobuf payloads with `AES-256-GCM` keyed by `reconnect_secret`, using a random 96-bit nonce and AAD `"KN_LAN1" || device_id`.

## Bounded Framing and FFI Boundary

Bounded framing and the FFI boundary keep untrusted input from reaching sensitive code.

- The build script `crates/keynear-protocol/build.rs` compiles `proto/keynear/v1/envelope.proto` with `prost-build` into `keynear_protocol::proto::v1`.
- `Envelope` groups its payloads by release.
  - Tags `1..14` carry `HandshakePayload`, `TransferOffer`, `ClipboardPayload`, `AudioStatePayload`, and `ControlAck`.
  - Tags `15..18` carry `LanEndpoint`, `TransferConsent`, `TransferCancel`, and `TransferComplete`.
  - Tags `19..20` carry `ClipboardOffer` and `ClipboardFetch`.
  - The Vega tags `21..23` carry `AudioHandoff`, `CapabilityNegotiation`, and `FallbackProbe`.
  - The Perseus tags `24..26` carry `TransferCheckpoint`, `TransferResumeRequest`, and `TransferResumeAck`.
  - The Carina tags `27..32` carry `MirroredNotification`, `NotificationAction`, `CallAlert`, `CallAction`, `MediaSessionState`, and `MediaCommand`.
  - The Phoenix tags `33..36` carry `AudioContinuityObservation`, `AudioContinuityCommand`, `RangingReport`, and `ProximityState`.
- Vega capability bits extend the existing `capability_flags` bitmask above `0x07`.
  - `0x08` audio handoff.
  - `0x10` BLE Channel Sounding ranging.
  - `0x20` UWB ranging.
  - `0x40` Wi-Fi Aware NAN.
  - `0x80` Wi-Fi Direct R2.

  Perseus adds `0x100` for resumable transfers, and lower bits stay unchanged. Carina adds `0x200` for notification mirroring, `0x400` for media bridging, and `0x800` for call alerts. Phoenix adds `0x1000` for audio continuity, `0x2000` for proximity lock, and `0x4000` for presence gating. All earlier bits stay unchanged.
- A `TransferCheckpoint` carries a bit-packed verified-chunk bitmap bounded at `16_384` bytes, a rolling SHA-256 chain, and a byte count that must match the bitmap. This lets a peer resume a 60 KiB chunk stream after a LAN-to-Wi-Fi-Aware transition without re-sending verified chunks.
- Control and stream framing applies fixed bounds.
  - The `ControlFrame` helpers `encode_control_frame`, `decode_control_frame`, `envelope_to_control_frame`, and `control_frame_to_envelope` validate `MAX_CONTROL_FRAME_BYTES` (`65_536` bytes) before allocation or decoding.
  - They reject empty payloads and unknown frame kinds.
  - QUIC stream helpers define `QuicStreamKind` values `0x01` Control, `0x02` RawFileChunk, `0x03` StoredZipArchive, and `0x04` ClipboardBlob, along with `write_control_envelope` and `read_control_envelope`.
  - `ClipboardOffer` defers outgoing image reads until a peer requests `ClipboardFetch`.
  - `ClipboardBlob` streams rendered images up to `MAX_CLIPBOARD_IMAGE_BYTES` (`16 MiB`) with a SHA-256 trailer outside the `65_536`-byte control frame bound.
- The FFI layer in `crates/keynear-ffi/src/` contains `lib.rs`, `pairing/`, `transfer.rs`, `clipboard.rs`, `vega.rs`, `vfs.rs`, `continuity.rs`, `phoenix.rs`, and `callbacks/`. It exports the following over `libkeynear_core.so` (`cdylib`) to `:core-ffi` and `org.keynear.app` on Android.
  - `CoreHandle` and `OrionNodeHandle`.
  - `OrionNodeHandle` includes `set_clipboard_config`, `send_clipboard_text`, `send_clipboard_blob`, `poll_incoming_clipboard`, and `fetch_clipboard_offer`.
  - `FfiNoisePairingSession`, `FfiTransferSession`, `FfiClipboardItem`, `FfiLanEndpoint`, and `FfiPairedDevice`.
  - `CoreStatus` and `FfiVisibilityMode`.
  - Beacon and LAN codecs, store helpers, `OwnedFd` single-owner FD wrappers (`send_fd`, `provide_receive_fd`), and `FfiError`.
  - Resumable-transfer and VFS exports: `FfiChunkCheckpoint`, `FfiReadPlan`, `FfiVfsNode`, `FfiResumeDecision`, `resumable_transfer_capability_flags`, `transfer_resume_decision`, `vfs_read_plan`, `vfs_root_node`, and `provide_vfs_read_fd`.
  - Continuity exports: `FfiNotificationDescriptor`, `FfiMirroredNotification`, `FfiMediaState`, `FfiCapabilityPair`, `FfiCallAction`, `FfiCallPhase`, `FfiNotificationAction`, `FfiPlayback`, `FfiMediaCommand`, `continuity_capability_flags`, `mirror_notification`, `validate_notification_reply`, `call_action_allowed`, and `media_command_allowed`.
