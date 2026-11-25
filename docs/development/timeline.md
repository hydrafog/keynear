# Release Timeline

Keynear organizes development into dependency-ordered milestones without calendar dates. Each milestone pairs a monotonic `versionCode` integer with a constellation-named `versionName` release. Each one depends on the milestone before it and unlocks the next layer of the *continuity stack*.

The milestones are listed in dependency order.

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
| 9 | 1.3.0-phoenix | Automatic audio continuity and proximity lock |
| 10 | 1.4.0-andromeda | Multi-device mesh and closest-peer routing |

## Lyra

Lyra (0.1.0-lyra, `versionCode` 1) establishes the reproducible monorepo, shared wire contracts, and platform entrypoints that every later milestone builds on.

- The `keynear-core` crate defines `PairedDevice`, `NearbyPeer`, `TransferSession`, `TransferItem`, `DaemonConfig`, typed `KeynearError` handling, and bounded 65,536-byte `proto3` control frames (`ControlFrame`).
- The `proto/` schema publishes `keynear.v1.Envelope` with `HandshakePayload`, `TransferOffer`, `ClipboardPayload`, `AudioStatePayload`, and `ControlAck`.
- Platform entrypoints set up `keynear` (Linux daemon, settings CLI, and desktop integrations) and the Android `org.keynear.app` module (`MainActivity`, `KeynearForegroundService`, `KeynearTileService`, `ShareActivity`) with 16 KB ELF page alignment enabled.
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

- The Linux daemon runs `keynear daemon` as a `systemd --user` service with a `ksni` `StatusNotifierItem` tray menu (visibility modes, device list, active transfer progress, quick toggles) and `org.freedesktop.Notifications` action buttons.
- `keynear` subcommands expose `keynear settings` configuration inspection, `keynear send --picker` with `ashpd` (`xdg-desktop-portal`) file selection, and Nautilus, Dolphin, and Yazi context-menu actions.
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
- `keynear daemon` bridges incoming peer notifications to `org.freedesktop.Notifications` with native inline reply (`RemoteInput`), mark-as-read, and dismissal actions synchronized in real time.
- Telephony call alerts surface `Answer`, `Decline`, and `Mute` actions on the desktop that coordinate directly with Android `TelecomManager` and Vega's PipeWire `hfp_hf` and `bap_sink` headset handoff.
- Bidirectional media session bridging links Linux `MPRIS2` players and Android `MediaSession` state so track metadata and playback controls appear in the Linux tray menu and Android Quick Settings media carousel.

## Phoenix

Phoenix (1.3.0-phoenix, `versionCode` 9) turns Vega's audio handoff and ranging primitives into automatic headset continuity and proximity-aware session locking.

- Automatic bidirectional audio continuity observes active playback and voice streams across PipeWire, MPRIS2, and Android audio sessions so shared LE Audio (`bap_sink`), `a2dp_sink`, and `hfp_hf` headsets follow whichever device starts playing without manual switching.
- Coordinated profile and volume handoff pairs BlueZ with PipeWire LE Audio (`bap_sink`, `bap_source`, and VCP) so headset volume and abort-and-reclaim recovery stay consistent across switches instead of stalling playback or dropping into degraded hands-free profiles.
- Proximity session supervision calls `org.freedesktop.login1` (`LockSession`) on Linux to lock the desktop when the paired phone leaves a calibrated distance bound, using background UWB `RangingManager` sessions on supported hardware and `CompanionDeviceManager` presence with smoothed BLE RSSI hysteresis everywhere else because Android restricts third-party background ranging on non-UWB radios.
- Presence-gated continuity pauses background clipboard sync, OTP forwarding, and auto-accept transfers while the phone is out of desk range or the desktop session is locked, resuming once the device returns.

## Andromeda

Andromeda (1.4.0-andromeda, `versionCode` 10) scales Keynear from a single desktop-to-phone link into a multi-device personal mesh routed by Phoenix's proximity layer.

- Concurrent multi-peer sessions extend `PairedDeviceStore` and `OrionQuicNode` to keep multiplexed `iroh` QUIC links open across multiple paired Linux workstations, laptops, phones, and tablets at once.
- Closest-peer routing applies Phoenix's calibrated UWB ranging and BLE RSSI hysteresis so clipboard updates, OTP codes, notification mirrors, and audio handoff target only the nearest active, unlocked machine.
- Multi-recipient file drops allow selecting more than one paired device from the existing Linux picker or Android `ShareActivity` to stream parallel QUIC transfers without adding group-management dashboards.
- Mesh key lifecycle management performs zero-downtime pairwise rotation (`irk`, `reconnect_secret`, and `quic_ed25519`) across `oo7` and Android Keystore and broadcasts signed revocations to remaining paired peers when a device is unpaired or lost.
