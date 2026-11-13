# Release Timeline

`Keynear` organizes development into dependency-ordered milestones without calendar dates. Each milestone pairs a monotonic `versionCode` integer with a constellation-named `versionName` release and unlocks the next layer of the continuity stack.

## Milestone Summary

| Order | `versionCode` | `versionName` | Focus | Upstream Dependency |
| :--- | :--- | :--- | :--- | :--- |
| 1 | `1` | `0.1.0-lyra` | Foundation & Core Contracts | `none` |
| 2 | `2` | `0.2.0-cygnus` | BLE Discovery & Noise Pairing | `0.1.0-lyra` |
| 3 | `3` | `0.3.0-orion` | Encrypted QUIC & File Streaming | `0.2.0-cygnus` |
| 4 | `4` | `0.4.0-aquila` | Native OS Surfaces & Settings | `0.3.0-orion` |
| 5 | `5` | `0.5.0-draco` | Universal Clipboard & OTP Sync | `0.4.0-aquila` |
| 6 | `6` | `1.0.0-vega` | Smart Audio Handoff & Fallbacks | `0.5.0-draco` |

## 1. Lyra (`0.1.0-lyra`, `versionCode` 1)

Lyra establishes the reproducible monorepo, shared wire contracts, and platform entrypoints that every later milestone builds on.

- **Shared Core (`keynear-core`)**: Defines `PairedDevice`, `NearbyPeer`, `TransferSession`, `TransferItem`, `DaemonConfig`, typed `KeynearError` handling, and bounded `65_536`-byte `proto3` control framing (`ControlFrame`).
- **Wire Schema (`proto/`)**: Publishes `keynear.v1.Envelope` with `HandshakePayload`, `TransferOffer`, `ClipboardPayload`, `AudioStatePayload`, and `ControlAck`.
- **Platform Entrypoints**: Sets up `keynear-linux`, `keynear-settings`, and the Android `org.keynear.app` module (`MainActivity`, `KeynearForegroundService`, `KeynearTileService`, `ShareActivity`) with 16 KB ELF page alignment enabled.
- **Reproducible Tooling**: Provides `flake.nix`, `.envrc`, quality validation scripts, Lefthook hooks, and CI workflows.

## 2. Cygnus (`0.2.0-cygnus`, `versionCode` 2)

Cygnus implements proximity discovery, privacy-preserving BLE advertisements, and authenticated pairing on top of Lyra's domain models and `proto3` envelope.

- **Rotating Presence Tokens**: Derives 6-byte HMAC-SHA256 presence tokens from `DeviceId + Counter` and the pairwise `ReconnectKey`, paired with a compact AES-GCM state payload inside the 31-byte BLE service-data limit.
- **Noise XX Handshake**: Executes `Noise_XX_25519_ChaChaPoly_BLAKE2s` over BLE GATT (`bluer` on Linux and Android BLE GATT), generating a 6-digit verification code from the handshake transcript hash under a 120-second timeout.
- **Keyring Persistence**: Stores `irk`, `reconnectSecret`, and static X25519 peer identities in `oo7` (`org.freedesktop.secrets`) on Linux and Android Keystore on mobile, keeping secrets out of TOML configuration.
- **Background Wakeups**: Integrates Android `CompanionDeviceManager` (`ObservingDevicePresenceRequest` and `CompanionDeviceService`) for paired devices and `BluetoothLeScanner` `PendingIntent` filters for `Everyone nearby` discovery.

## 3. Orion (`0.3.0-orion`, `versionCode` 3)

Orion adds the high-throughput, relay-free data plane once Cygnus establishes trusted peer keys and ephemeral LAN address exchange.

- **P2P QUIC Transport**: Connects peers directly over `iroh` with public relays and DNS/pkarr disabled, using `rustls` hybrid post-quantum `X25519MLKEM768` (RFC 10024) with AES-256-GCM or ChaCha20-Poly1305.
- **Local Discovery**: Exchanges ephemeral LAN endpoints over `mdns-sd` or the encrypted BLE GATT control channel, handling Android `ACCESS_LOCAL_NETWORK` gating and `NsdManager` `FLAG_SHOW_PICKER` fallback.
- **Zero-Copy File Streaming**: Streams files in 60 KB chunks and directories as uncompressed `STORED` zip archives with 1 MB buffered I/O, passing raw file descriptors via `ParcelFileDescriptor.detachFd()` into `MediaStore.Downloads` on Android.
- **Consent & Auto-Accept**: Enforces incoming drop consent prompts (`pendingConsent` -> `active` -> `completed`) and per-device `autoAcceptFiles` overrides.

## 4. Aquila (`0.4.0-aquila`, `versionCode` 4)

Aquila surfaces Orion's transport and Cygnus's discovery controls through native OS interfaces without adding dashboard bloat.

- **Linux Tray & Daemon**: Runs `keynear-linux` as a `systemd --user` service with a `ksni` `StatusNotifierItem` tray menu (visibility modes, device list, active transfer progress, quick toggles) and `org.freedesktop.Notifications` action buttons.
- **Linux Settings & Portals**: Launches `keynear-settings` (`GTK4`/`relm4`) on demand from the tray menu, integrates `ashpd` (`xdg-desktop-portal`) file selection, and adds Nautilus, Dolphin, and Yazi context-menu actions.
- **Android System Surfaces**: Implements the silent `connectedDevice` foreground notification, lock-aware `KeynearTileService` Quick Settings tile, translucent `ShareActivity` with Direct Share targets, and `Notification.ProgressStyle` Live Updates backed by `User-Initiated Data Transfer Jobs`.
- **Companion Onboarding**: Completes the minimal Material 3 Compose screen in `MainActivity` for one-time permissions (`NEARBY_DEVICES`, `POST_NOTIFICATIONS`, battery-optimization exemption) and paired-device management.

## 5. Draco (`0.5.0-draco`, `versionCode` 5)

Draco builds real-time clipboard and one-time password continuity over Aquila's tray/tile surfaces and Orion's multiplexed QUIC streams.

- **Wayland & X11 Clipboard**: Monitors clipboard changes via `ext-data-control-v1` (with `zwlr-data-control-v1` and X11 fallbacks), using delayed rendering for outgoing clipboard images.
- **Password MIME Filtering**: Inspects offered MIME targets before reading payload bytes and skips `x-kde-passwordManagerHint`, `application/x-password`, and user-configured `ignoredMimeTypes`.
- **Android Clipboard & OTPs**: Receives Linux clipboard updates in the background, supports notification/tile-triggered phone clipboard reads, and integrates Android SMS User Consent to forward verified one-time passwords to the desktop clipboard and the tray **Copy latest OTP** action.

## 6. Vega (`1.0.0-vega`, `versionCode` 6)

Vega completes the production release with conflict-free Bluetooth audio handoff and resilient connectivity fallbacks.

- **Smart Audio Handoff**: Monitors BlueZ device/transport objects alongside PipeWire (`bap_sink` LE Audio, `a2dp_sink`, and `hfp_hf`) and MPRIS2 media players, pausing playback and releasing the Linux sink cleanly before the phone claims shared headphones or earbuds.
- **GNOME Mutter Bridge**: Adds an optional lightweight GNOME Shell extension D-Bus bridge for background clipboard sync under Mutter.
- **No-LAN P2P Fallback**: Probes `nl80211` and `wpa_supplicant` for Wi-Fi Aware NAN Data Paths (`AwarePairingConfig`) ahead of Wi-Fi Direct R2 (`WifiP2pPairingBootstrappingConfig`) when no shared Wi-Fi network is available.
- **Ranging Readiness**: Activates `capabilityFlags` negotiation for Android `RangingManager` (BLE Channel Sounding / UWB) closeness gating where hardware and BlueZ support are present.
