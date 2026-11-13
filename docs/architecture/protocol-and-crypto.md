# Protocol and Cryptography

All `Keynear` discovery, pairing, and data transport operate peer-to-peer without cloud accounts or relay servers.

## Pairing and Transport Flow

```mermaid
sequenceDiagram
  participant L as Linux Daemon
  participant A as Android App
  L->>A: BLE rotating HMAC-SHA256 token
  A->>L: Noise_XX_25519_ChaChaPoly_BLAKE2s over BLE GATT
  L-->>A: 6-digit verification code match
  A-->>L: SessionExportContribution saved to keyring
  L->>A: Direct P2P QUIC with rustls X25519MLKEM768
  A->>L: Multiplexed proto3 Envelope streams
```

## Proximity Discovery and Pairing

- **Presence Tokens**: Paired devices derive rotating 6-byte HMAC-SHA256 presence tokens from `DeviceId + Counter` and the pairwise `ReconnectKey`, preventing passive MAC or identity tracking.
- **BLE Payload**: An AES-GCM encrypted state payload carries availability and battery status within the 31-byte BLE service-data limit.
- **Pairing Handshake**: Initial pairing executes `Noise_XX_25519_ChaChaPoly_BLAKE2s` over BLE GATT, deriving a 6-digit verification code from the handshake transcript hash and exchanging a `SessionExportContribution` for subsequent silent reconnects.

## Encrypted QUIC Transport and FFI Boundary

- **Proto3 Schema Compilation**: `crates/keynear-core/build.rs` compiles `proto/keynear/v1/envelope.proto` with `prost-build` into `keynear_core::proto::v1`.
- **Bounded Control Framing**: Control and continuity messages serialize as `proto3` `Envelope` payloads (`HandshakePayload`, `TransferOffer`, `ClipboardPayload`, `AudioStatePayload`, `ControlAck`) wrapped in `ControlFrame` (`encode_control_frame`, `decode_control_frame`, `envelope_to_control_frame`, `control_frame_to_envelope`). Decoders validate the `MAX_CONTROL_FRAME_BYTES` (`65_536`-byte) limit before decoding and reject empty `Envelope` payloads or unknown frame kinds.
- **UniFFI Bridge**: `crates/keynear-core/src/uniffi_bridge.rs` exports `CoreHandle`, `CoreStatus`, `FfiVisibilityMode`, and `FfiError` over `libkeynear_core.so` (`cdylib`) to `org.keynear.app` on Android.
- **Post-Quantum TLS 1.3**: Wi-Fi QUIC links use `rustls` with hybrid `X25519MLKEM768` (RFC 10024) key exchange, selecting AES-256-GCM when hardware-accelerated on both peers and ChaCha20-Poly1305 otherwise.
