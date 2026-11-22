# System Overview

Keynear (`1.2.0-carina`) is a single monorepo with a shared Rust core, a unified Linux daemon and CLI binary, Protocol Buffers schemas, and an Android companion app.

## Module Topology

```mermaid
flowchart LR
  proto["proto/keynear/v1/envelope.proto"] --> protocol["crates/keynear-protocol"]
  protocol --> core["crates/keynear-core"]
  protocol --> ipc["crates/keynear-ipc"]
  core --> linux["crates/keynear"]
  ipc --> linux
  core --> ffi["crates/keynear-ffi"]
  ffi --> uniffi["android:core-ffi (UniFFI JNI)"]
  ds["android:designsystem"] --> android["android:app (org.keynear.app)"]
  uniffi --> android
```

## Workspace Layout

Workspace naming separates executables from internal libraries. The executable crate and binary use the bare primary product noun `keynear` with subcommands for user surfaces (`keynear settings`, `keynear daemon`, `keynear send`), while internal workspace libraries use `<product>-<layer>` (`keynear-protocol`, `keynear-core`, `keynear-ipc`, `keynear-ffi`).

The workspace is organized into the following crates and modules.

- `crates/keynear-protocol` is the wire and domain contract crate. It defines:
  - `proto3` codegen and the domain models `PairedDevice`, `PairedDeviceStore`, `NearbyPeer`, `DaemonConfig`, and `TransferSession`.
  - Display-name validation (`MAX_DISPLAY_NAME_LEN`, `validate_display_name`).
  - Atomic config and device TOML persistence (`DaemonConfig::save_to_path`, `PairedDeviceStore::rename`).
  - `KeynearError` and constants.
- `crates/keynear-core` is the cross-platform engine, with no UniFFI or `oo7` dependency. It provides:
  - BLE presence and AES-256-GCM LAN endpoint codecs (`transport/presence.rs`, UUIDs `0xFE6D` and `0xFE70`).
  - `Noise_XX_25519_ChaChaPoly_BLAKE2s` pairing (`pairing/`, 120-second timeout, 6-digit verification codes).
  - Relay-free `iroh` P2P QUIC transport with `rustls` `X25519MLKEM768` (`transport/quic.rs`).
  - `mdns-sd` local discovery (`transport/lan_discovery.rs`).
  - 60 KiB chunked and `STORED` zip file streaming with `OwnedFd` zero-copy I/O and consent gating (`transfer/`).
  - A delayed-render clipboard engine and MIME policy filter (`clipboard/`).
  - Named capability bits and pure negotiation (`capability.rs`).
  - The ordered audio handoff state machine (`audio.rs`).
  - Wi-Fi Aware-before-Direct fallback selection (`fallback.rs`).
  - Bounded checkpoint bitmaps and a rolling SHA-256 chain with a pure resume decision (`resume.rs`).
  - A virtual-filesystem model of opaque remote roots, documents, and on-demand read plans (`vfs/`).
  - The platform-agnostic `SecretStore` trait and `InMemorySecretStore` (`keyring.rs`).
- `crates/keynear-ffi` exposes the UniFFI bridge, split across `pairing/`, `transfer.rs`, `clipboard.rs`, `vega.rs`, `vfs.rs`, `continuity.rs`, `callbacks/`, and `bin/uniffi_bindgen.rs`. It builds `libkeynear_core.so` for Android `:core-ffi`.
- `crates/keynear-ipc` defines the shared Linux daemon and CLI IPC schema, the frame codec, and the Unix domain socket client used by `keynear` subcommands.
- `crates/keynear` provides the unified `keynear` executable, running an asynchronous `tokio` daemon (`keynear daemon`, `service/`) under `packaging/linux/systemd/keynear.service` alongside subcommand CLI tools (`cli/`) and XDG portal picking (`portal.rs`). It manages:
  - BlueZ (`bluer`) BLE advertising, scanning, and GATT pairing (`ble/`).
  - `mdns-sd` `_keynear._udp.local.` discovery.
  - `Oo7SecretStore` (`keyring_oo7/`).
  - Native Wayland (`ext-data-control-v1`, `zwlr-data-control-v1`) and X11 (`x11rb`) clipboard monitoring with pre-read MIME filtering and echo suppression (`clipboard.rs`).
  - Memory-only `latest_otp` retention.
  - The 10-minute `EveryoneNearby` auto-revert (`EVERYONE_NEARBY_TIMEOUT_SECS = 600`).
  - A `ksni` `StatusNotifierItem` tray popup menu (`tray.rs`, `tray/menu.rs`) displaying `org.keynear.Keynear-symbolic`, `Keynear` title and tooltip, click-to-open activation (`MENU_ON_ACTIVATE`), and `Send files...` via `keynear send --picker`.
  - `zbus` notifications (`notifications.rs`).
  - Subcommand-first CLI parsing (`cli.rs`, `cli/parse.rs`, `cli/send.rs`) for `daemon`, `settings`, `status`, `devices`, `transfers`, `send`, `accept`, `decline`, `cancel`, `visibility`, `rename`, `version`, and `help`, wired into Nautilus, Dolphin, and Yazi context menus (`packaging/linux/integrations/`).
  - `daemon.sock` (`0o600`) Unix IPC handlers (`ipc/`).

  The daemon degrades gracefully when BlueZ, mDNS, Wayland/X11 clipboard, D-Bus notifications or tray watcher, or Secret Service are unavailable. Three capability layers extend it further.

  - Audio handoff and path fallbacks add a `zbus` BlueZ/MPRIS2 sink observer and handoff controller (`audio/`), a graded `wpa_supplicant` no-LAN prober (`fallback.rs`), and startup capability advertisement (`vega.rs`). A headless GNOME Shell D-Bus clipboard bridge lives under `packaging/gnome/keynear-clipboard@hydrafog.github.com/`.
  - Resumable transfers and the virtual filesystem add a typed FUSE mount controller with an injectable backend and an explicit `fuse_backend_unavailable` log event when kernel FUSE or `libfuse3` is absent (`fuse/`, `fuse/probe.rs`, `fuse/backend.rs`). The real `fuser` adapter is optional behind the `fuse` Cargo feature so the default build stays headless.
  - Notification and media continuity extends the notification client with mirrored reply, mark-as-read, and dismiss actions (`notifications/`, `notifications/mirror.rs`), adds an MPRIS2 media observer and a `Now playing` tray group (`media.rs`, `tray/media_menu.rs`), and routes call actions through a typed dispatcher (`call.rs`, `ipc/handlers/actions.rs`).
- The wire schema at `proto/keynear/v1/envelope.proto` defines `Envelope` and four groups of payloads.
  - The base payloads are `HandshakePayload` (`auth_proof`), `TransferOffer`, `ClipboardPayload`, `AudioStatePayload`, `ControlAck`, `LanEndpoint`, `TransferConsent`, `TransferCancel`, `TransferComplete`, `ClipboardOffer`, and `ClipboardFetch`.
  - The audio and fallback messages (`1.0.0-vega`) are `AudioHandoff`, `CapabilityNegotiation`, and `FallbackProbe`.
  - The resumable transfer messages (`1.1.0-perseus`) are `TransferCheckpoint`, `TransferResumeRequest`, and `TransferResumeAck` at tags `24..26`.
  - The continuity messages (`1.2.0-carina`) are `MirroredNotification`, `NotificationAction`, `CallAlert`, `CallAction`, `MediaSessionState`, and `MediaCommand` at tags `27..32`.
- The Android companion in `android/` splits into three Gradle modules.
  - `:core-ffi` holds Gradle-generated UniFFI Kotlin bindings and 16 KB ELF page-aligned `jniLibs/`.
  - `:designsystem` holds `Tokens.kt`, `Theme.kt`, `KeynearIcons.kt`, and split `Controls*` components in `org.keynear.designsystem`.
  - `:app` (`org.keynear.app`, `versionCode = 8`, `versionName = "1.2.0-carina"`) separates several concerns:
    - BLE: `BleScanner`, `BleAdvertiser`, `GattSession`, and `PairingNotifier`.
    - Core bridge and event loop: `bridge/{CoreBridge,EventQueue,KeystoreSecretStore}`.
    - Repositories and LAN discovery: `data/{DeviceRepo,TransferRepo,PairingRepo,LanManager}`.
    - Background services: `service/`.
    - Share and Direct Share: `share/{ShareActivity,IntentResolver,DirectShareManager}`.
    - Pure ranging and fallback negotiation helpers: `vega/{VegaCapability,RangingPolicy,FallbackSelection}`.
    - A manifest-registered `DocumentsProvider` with pure descriptor and stream helpers: `vfs/{KeynearDocumentsProvider,VfsDescriptors,RemoteDocumentStream}`.
    - Notification mirroring behind an explicit opt-in with pure exclusion helpers: `notifications/{KeynearNotificationListenerService,NotificationMirror}`.
    - Call actions through `TelecomManager` with a screening fallback: `call/{CallAlertController,KeynearCallScreeningService}`.
    - Media session state for the Quick Settings carousel: `media/MediaBridgeController`.
    - Compose screens and UI state: `ui/{devices,navigation,receive,send,settings,model}`.
