# Release Timeline

Keynear organizes development into dependency-ordered milestones without calendar dates. Each milestone pairs a monotonic `versionCode` integer with a constellation-named `versionName` release, depends on the milestone before it, and unlocks the next layer of the *continuity stack*.

| Version code | Version name | Focus |
| ---: | :--- | :--- |
| 1 | 0.1.0-lyra | Foundation and core contracts |
| 2 | 0.2.0-cygnus | BLE discovery and Noise pairing |
| 3 | 0.3.0-orion | Encrypted QUIC and file streaming |
| 4 | 0.4.0-aquila | Native OS surfaces and settings |
| 5 | 0.5.0-draco | Universal clipboard and OTP sync |
| 6 | 1.0.0-vega | Smart audio handoff and fallbacks |
| 7 | 1.1.0-perseus | Resumable transfers and virtual filesystem |
| 8 | 1.2.0-carina | Notification mirror and media controls |
| 9 | 1.3.0-pegasus | Ephemeral camera, mic, and screen pipes |
| 10 | 1.4.0-phoenix | Proximity lock, SSH agent, and FIDO2 bridge |
| 11 | 1.5.0-andromeda | Multi-peer mesh and input handoff |

## Lyra

Lyra (0.1.0-lyra, `versionCode` 1) establishes the reproducible monorepo, shared wire contracts, and platform entrypoints that every later milestone builds on.

- The `keynear-core` crate defines `PairedDevice`, `NearbyPeer`, `TransferSession`, `TransferItem`, `DaemonConfig`, typed `KeynearError` handling, and bounded 65,536-byte `proto3` control frames (`ControlFrame`).
- The `proto/` schema publishes `keynear.v1.Envelope` with `HandshakePayload`, `TransferOffer`, `ClipboardPayload`, `AudioStatePayload`, and `ControlAck`.
- Platform entrypoints set up `keynear-linux`, `keynear-settings`, and the Android `org.keynear.app` module (`MainActivity`, `KeynearForegroundService`, `KeynearTileService`, `ShareActivity`) with 16 KB ELF page alignment enabled.
- Reproducible tooling provides `flake.nix`, `.envrc`, quality validation scripts, Lefthook hooks, and CI workflows.

## Cygnus

Cygnus (0.2.0-cygnus, `versionCode` 2) implements proximity discovery, privacy-preserving BLE advertisements, and authenticated pairing on top of Lyra's domain models and `proto3` envelope.

- The daemon derives 6-byte HMAC-SHA256 presence tokens from `DeviceId + Counter` and the pairwise `ReconnectKey`, pairing them with a compact AES-GCM state payload inside the 31-byte BLE service-data limit.
- Pairing executes `Noise_XX_25519_ChaChaPoly_BLAKE2s` over BLE GATT (`bluer` on Linux, Android BLE GATT on mobile), generating a 6-digit verification code from the handshake transcript hash under a 120-second timeout.
- The keyring stores `irk`, `reconnectSecret`, and static X25519 peer identities in `oo7` (`org.freedesktop.secrets`) on Linux and Android Keystore on mobile, keeping secrets out of TOML configuration.
- Background wakeups use Android `CompanionDeviceManager` (`ObservingDevicePresenceRequest` and `CompanionDeviceService`) for paired devices and `BluetoothLeScanner` `PendingIntent` filters for `Everyone nearby` discovery.

## Orion

Orion (0.3.0-orion, `versionCode` 3) adds the high-throughput, relay-free data plane once Cygnus establishes trusted peer keys and ephemeral LAN address exchange.

- The QUIC transport connects peers directly over `iroh` with public relays and DNS/pkarr disabled, using `rustls` hybrid post-quantum `X25519MLKEM768` (RFC 10024) with AES-256-GCM or ChaCha20-Poly1305.
- Local discovery exchanges ephemeral LAN endpoints over `mdns-sd` or the encrypted BLE GATT control channel, handling Android `ACCESS_LOCAL_NETWORK` gating and the `NsdManager` `FLAG_SHOW_PICKER` fallback.
- Zero-copy file streaming sends files in 60 KB chunks and directories as uncompressed `STORED` zip archives with 1 MB buffered I/O, passing raw file descriptors through `ParcelFileDescriptor.detachFd()` into `MediaStore.Downloads` on Android.
- Incoming drops wait for consent (`pendingConsent` to `active` to `completed`) unless a per-device `autoAcceptFiles` override applies.

## Aquila

Aquila (0.4.0-aquila, `versionCode` 4) surfaces Orion's transport and Cygnus's discovery controls through native OS interfaces without adding dashboard bloat.

- The Linux daemon runs `keynear-linux` as a `systemd --user` service with a `ksni` `StatusNotifierItem` tray menu (visibility modes, device list, active transfer progress, quick toggles) and `org.freedesktop.Notifications` action buttons.
- `keynear-settings` (`GTK4`/`relm4`) launches on demand from the tray menu, integrates `ashpd` (`xdg-desktop-portal`) file selection, and adds Nautilus, Dolphin, and Yazi context-menu actions.
- Android surfaces provide the silent `connectedDevice` foreground notification, a lock-aware `KeynearTileService` Quick Settings tile, a translucent `ShareActivity` with Direct Share targets, and `Notification.ProgressStyle` live updates backed by `User-Initiated Data Transfer Jobs`.
- Onboarding completes the minimal Material 3 Compose screen in `MainActivity` for one-time permissions (`NEARBY_DEVICES`, `POST_NOTIFICATIONS`, battery-optimization exemption) and paired-device management.

## Draco

Draco (0.5.0-draco, `versionCode` 5) builds real-time clipboard and one-time password continuity over Aquila's tray and tile surfaces and Orion's multiplexed QUIC streams.

- Clipboard monitoring watches Wayland and X11 changes through `ext-data-control-v1` with `zwlr-data-control-v1` and X11 fallbacks, and renders outgoing clipboard images on demand.
- Password MIME filtering inspects offered MIME targets before reading payload bytes and skips `x-kde-passwordManagerHint`, `application/x-password`, and user-configured `ignoredMimeTypes`.
- Android clipboard support receives Linux updates in the background, reads the phone clipboard from a notification or tile, and forwards verified one-time passwords from Android SMS User Consent to the desktop clipboard and the tray **Copy latest OTP** action.

## Vega

Vega (1.0.0-vega, `versionCode` 6) completes the production release with conflict-free Bluetooth audio handoff and resilient connectivity fallbacks.

- Audio handoff monitors BlueZ device and transport objects alongside PipeWire (`bap_sink` LE Audio, `a2dp_sink`, and `hfp_hf`) and MPRIS2 media players, pausing playback and releasing the Linux sink cleanly before the phone claims shared headphones or earbuds.
- An optional lightweight GNOME Shell extension adds a D-Bus bridge for background clipboard sync under Mutter.
- The no-LAN fallback probes `nl80211` and `wpa_supplicant` for Wi-Fi Aware NAN Data Paths (`AwarePairingConfig`) ahead of Wi-Fi Direct R2 (`WifiP2pPairingBootstrappingConfig`) when no shared Wi-Fi network is available.
- Ranging negotiation activates `capabilityFlags` for Android `RangingManager` (BLE Channel Sounding or UWB) closeness gating where hardware and BlueZ support are present.

## Perseus

Perseus (1.1.0-perseus, `versionCode` 7) extends Orion's streaming engine and Vega's transport fallbacks with checkpointed transfer resumption and on-demand virtual filesystem browsing.

- Resumable transfer negotiation adds chunk-bitmap checkpoints and rolling SHA-256 verification to `keynear.v1.Envelope` so interrupted multi-gigabyte drops resume from the last verified 60 KB chunk across LAN and Wi-Fi Aware transitions.
- The Linux daemon exposes paired Android `MediaStore` and Storage Access Framework roots as an on-demand user-space mount via FUSE3 (`fuser`), allowing Nautilus, Dolphin, and Yazi to browse and stream remote files without copying full archives upfront.
- Android implements a native `DocumentsProvider` backed by `keynear-core` QUIC streams, surfacing paired Linux shared directories directly inside the system `Files` app and document pickers.
- Release packaging bundles offline `Plus Jakarta Sans` font assets and adds reproducible NixOS Home Manager (`services.keynear`), Flathub, and F-Droid build definitions.

## Carina

Carina (1.2.0-carina, `versionCode` 8) layers ephemeral notification mirroring, inline replies, and bidirectional media transport controls over Aquila's native desktop and mobile surfaces.

- Android `NotificationListenerService` forwards opted-in messaging and system notifications over `keynear.v1.Envelope` with per-package exclusion filters and zero on-disk message persistence.
- `keynear-linux` bridges incoming peer notifications to `org.freedesktop.Notifications` with native inline reply (`RemoteInput`), mark-as-read, and dismissal actions synchronized in real time.
- Telephony call alerts surface `Answer`, `Decline`, and `Mute` actions on the desktop that coordinate directly with Android `TelecomManager` and Vega's PipeWire `hfp_hf` and `bap_sink` headset handoff.
- Bidirectional media session bridging links Linux `MPRIS2` players and Android `MediaSession` state so track metadata and playback controls appear in the Linux tray menu and Android Quick Settings media carousel.

## Pegasus

Pegasus (1.3.0-pegasus, `versionCode` 9) introduces low-latency audio and video peripheral continuity over Vega's PipeWire graph and Orion's QUIC streams.

- Android `Camera2` (`HardwareBuffer` and hardware `MediaCodec` H.264/AV1) and microphone capture stream over QUIC into a native Linux PipeWire virtual camera and audio source selectable through the `xdg-desktop-portal` Camera portal without kernel `v4l2loopback` modules.
- Linux `xdg-desktop-portal` `ScreenCast` and PipeWire DMA-BUF capture stream a selected desktop window or virtual monitor to the Android companion app for low-latency second-screen viewing.
- Congestion-aware media pacing adapts bitrate, resolution, and frame rate dynamically using `iroh` QUIC RTT and packet-loss metrics across Wi-Fi LAN and Wi-Fi Aware NAN Data Paths.
- Hardware-enforced privacy controls surface active camera, microphone, and screen-cast indicators with instant one-tap termination in both `KeynearTileService` and the Linux `StatusNotifierItem` tray menu.

## Phoenix

Phoenix (1.4.0-phoenix, `versionCode` 10) combines Vega's `RangingManager` closeness gating with Cygnus's hardware-backed keyrings to provide proximity session locking, remote SSH signing, and FIDO2 passkey bridging.

- Proximity session supervision integrates with `org.freedesktop.login1` (`logind`) on Linux to lock the desktop when the paired phone leaves the calibrated BLE Channel Sounding or UWB distance bound and ready unlocking upon return.
- `keynear-linux` exposes an optional `SSH_AUTH_SOCK` agent forwarding signing requests over QUIC to hardware-backed Ed25519 and ECDSA keys in Android Keystore (`StrongBox` and TEE) gated by `BiometricPrompt`.
- A Linux UHID CTAP2 virtual security key bridge routes WebAuthn and FIDO2 ceremony requests over the authenticated QUIC channel to Android `CredentialManager` passkeys.
- Cryptographic challenge binding ties every unlock, SSH signature, and FIDO2 assertion to both the active `X25519MLKEM768` TLS session transcript and a live hardware ranging measurement.

## Andromeda

Andromeda (1.5.0-andromeda, `versionCode` 11) scales Keynear from a single desktop-to-phone link into a multi-peer personal continuity mesh with cross-device input handoff.

- `PairedDeviceStore` and `OrionQuicNode` multiplex concurrent sessions across multiple paired Linux workstations, laptops, and Android phones or tablets, using `RangingManager` distance and BLE RSSI hysteresis to route proximity actions to the closest active peer.
- Cross-screen input and cursor handoff uses `libei` (Emulated Input Server) and `xdg-desktop-portal` `RemoteDesktop` on Wayland alongside an Android tablet pointer receiver to move keyboard and pointer focus smoothly across screen edges.
- Mesh key lifecycle management adds zero-downtime pairwise rotation (`irk`, `reconnect_secret`, and `quic_ed25519`) and signed revocation broadcasts across all paired devices in `oo7` and Android Keystore.
- Multi-target fan-out enables simultaneous clipboard broadcast and parallel QUIC file drops to selected device groups from the tray menu and Android Share Sheet.
