# Protocol and Cryptography

`Keynear` discovery, pairing, and continuity transport run peer-to-peer without cloud accounts or relay servers.

## Pairing and Transport Flow

```mermaid
sequenceDiagram
  participant L as Linux Daemon
  participant A as Android App
  L->>A: BLE 0xFE6D Beacon (EveryoneNearby or PairedOnly 24-byte token)
  A->>L: Noise_XX_25519_ChaChaPoly_BLAKE2s over BLE GATT (0xFE6E / 0xFE6F)
  L-->>A: 6-digit BLAKE2s verification code match (120s timeout)
  A-->>L: SessionExportContribution saved to Keystore / oo7
  L<-->A: LanEndpoint via mdns-sd (_keynear._udp.) or encrypted BLE 0xFE70
  L<-->A: Direct iroh QUIC (RelayMode::Disabled, rustls X25519MLKEM768) + Envelope streams
```

## Proximity Discovery and Pairing

- Keynear advertises and filters on the 16-bit BLE Service UUID `0xFE6D` (`0000FE6D-0000-1000-8000-00805F9B34FB`). The GATT Pairing TX characteristic `0xFE6E` supports `Write` and `WriteWithoutResponse`, and the Pairing RX characteristic `0xFE6F` supports `Notify` up to `MAX_GATT_PAIRING_FRAME_BYTES = 512`.
- Beacons stay within `MAX_BEACON_DATA_BYTES = 24`. Everyone nearby carries the protocol version, device role, visibility flag, truncated UTF-8 display name, and capability bitmask for open pairing discovery, and reverts to paired devices only after `600` seconds. Paired devices only carries a 6-byte rotating HMAC-SHA256 presence token (`device_id || u32 counter` keyed by the pairwise `irk`), a 4-byte little-endian `u32` counter, and a 14-byte AES-256-GCM encrypted state payload (`availability`, optional `battery_level`, authenticated with `reconnect_secret` and a deterministic 96-bit nonce).
- Initial pairing executes `Noise_XX_25519_ChaChaPoly_BLAKE2s` (`snow`) over BLE GATT. After the handshake both sides derive a 6-digit verification code (`000000`–`999999`) from `BLAKE2s(handshake_hash || "keynear-verify-v1")` and require user confirmation within `120` seconds before committing `SessionExportContribution` (`device_id`, `display_name`, `role`, `capability_flags`, `irk`, `reconnect_secret`) to `PairedDeviceStore` and OS secret storage.

## Relay-Free P2P QUIC and LAN Discovery

- `OrionQuicNode` (`crates/keynear-core/src/transport/quic.rs`) constructs an `iroh` endpoint from `Endpoint::builder(presets::Empty)` with `RelayMode::Disabled` and no DNS or pkarr discovery, keeping all traffic on local or direct peer addresses under ALPN `keynear/orion/1`.
- A custom `rustls::crypto::CryptoProvider` pins hybrid post-quantum `X25519MLKEM768` (RFC 10024) key exchange and prioritizes `TLS13_AES_256_GCM_SHA384` and `TLS13_CHACHA20_POLY1305_SHA256` ahead of `TLS13_AES_128_GCM_SHA256`.
- `PairedPeerHooks::after_handshake` rejects connections from `EndpointId`s absent from the paired allowlist. After QUIC connection setup, both peers exchange `HandshakePayload` containing `auth_proof`, a 32-byte `HMAC-SHA256` tag keyed by pairwise `reconnect_secret` over `"keynear-orion-handshake-v1" || local_endpoint_id || remote_endpoint_id || local_device_id`, verified in constant time via `subtle::ConstantTimeEq`.
- Ephemeral `LanEndpoint` records propagate over `mdns-sd` (`_keynear._udp.local.` on Linux, `NsdManager` `_keynear._udp.` on Android) and over the encrypted BLE GATT characteristic `0xFE70` (`LAN_ENDPOINT_CHAR_UUID`). `LanEndpointCodec` encrypts `LanEndpoint` protobuf payloads with `AES-256-GCM` keyed by `reconnect_secret` using a random 96-bit nonce and AAD `"KN_LAN1" || device_id`.

## Bounded Framing and FFI Boundary

- `crates/keynear-protocol/build.rs` compiles `proto/keynear/v1/envelope.proto` with `prost-build` into `keynear_protocol::proto::v1`. `Envelope` carries tags `1..14` (`HandshakePayload`, `TransferOffer`, `ClipboardPayload`, `AudioStatePayload`, `ControlAck`), tags `15..18` (`LanEndpoint`, `TransferConsent`, `TransferCancel`, `TransferComplete`), tags `19..20` (`ClipboardOffer`, `ClipboardFetch`), and the Vega tags `21..23` (`AudioHandoff`, `CapabilityNegotiation`, `FallbackProbe`). Vega capability bits extend the existing `capability_flags` bitmask above `0x07`: `0x08` audio handoff, `0x10` BLE Channel Sounding ranging, `0x20` UWB ranging, `0x40` Wi-Fi Aware NAN, and `0x80` Wi-Fi Direct R2.
- `ControlFrame` (`encode_control_frame`, `decode_control_frame`, `envelope_to_control_frame`, `control_frame_to_envelope`) and QUIC stream helpers (`QuicStreamKind`: `0x01` Control, `0x02` RawFileChunk, `0x03` StoredZipArchive, `0x04` ClipboardBlob; `write_control_envelope` / `read_control_envelope`) validate `MAX_CONTROL_FRAME_BYTES` (`65_536` bytes) before allocation or decoding and reject empty payloads and unknown frame kinds. `ClipboardOffer` defers outgoing image reads until a peer requests `ClipboardFetch`, and `ClipboardBlob` streams rendered images up to `MAX_CLIPBOARD_IMAGE_BYTES` (`16 MiB`) with a SHA-256 trailer outside the `65_536`-byte control frame bound.
- `crates/keynear-ffi/src/` (`lib.rs`, `pairing.rs`, `transfer.rs`, `clipboard.rs`, `callbacks.rs`) exports `CoreHandle`, `OrionNodeHandle` (including `set_clipboard_config`, `send_clipboard_text`, `send_clipboard_blob`, `poll_incoming_clipboard`, and `fetch_clipboard_offer`), `FfiNoisePairingSession`, `FfiTransferSession`, `FfiClipboardItem`, `FfiLanEndpoint`, `FfiPairedDevice`, `CoreStatus`, `FfiVisibilityMode`, beacon/LAN codecs, store helpers, `OwnedFd` single-owner FD wrappers (`send_fd`, `provide_receive_fd`), and `FfiError` over `libkeynear_core.so` (`cdylib`) to `:core-ffi` and `org.keynear.app` on Android.
