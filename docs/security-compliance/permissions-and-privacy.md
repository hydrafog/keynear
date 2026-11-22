# Permissions and Privacy

Keynear (`1.2.0-carina`) requests only the OS permissions needed for local peer discovery, notifications, clipboard continuity, direct Wi-Fi QUIC transfers, and negotiated proximity features.

## Android Permissions

Android requests the following permissions.

- `BLUETOOTH_SCAN`, `BLUETOOTH_CONNECT`, and `BLUETOOTH_ADVERTISE` support four BLE functions.
  - `0xFE6D` BLE presence advertisements.
  - `BleScanReceiver` `PendingIntent` background scanning.
  - Noise XX GATT pairing (`0xFE6E`/`0xFE6F`).
  - Encrypted LAN endpoint exchange (`0xFE70`).

  Keynear never requests fine location.
- `POST_NOTIFICATIONS` (API 33 and later) supports the following alerts.
  - `PairingNotifier` 6-digit pairing prompts.
  - Incoming file transfer consent (`Accept`/`Decline`).
  - Live `ProgressStyle` / `NotificationCompat` transfer updates.
  - Completion and error alerts.
  - Local-network-blocked alerts.
  - Silent `connectedDevice` continuity visibility.
- Notification listener access is granted through Android settings, not a runtime permission. `KeynearNotificationListenerService` stays inactive until the user opts in, and excluded packages are filtered before any envelope is built. No notification body, call number, or media metadata is written to disk.
- `ANSWER_PHONE_CALLS` supports Answer and Decline through `TelecomManager`. When it is unavailable, `KeynearCallScreeningService` rejects the call as a fallback, and Mute applies only during an active call.
- `FOREGROUND_SERVICE` and `FOREGROUND_SERVICE_CONNECTED_DEVICE` keep `KeynearForegroundService` active for BLE and `OrionNodeHandle` transport after Bluetooth permissions are granted.
- `RUN_USER_INITIATED_JOBS` allows `KeynearTransferJobService` to schedule `User-Initiated Data Transfer Jobs` (`setUserInitiated(true)` on API 34 and later) for active file transfers.
- `REQUEST_IGNORE_BATTERY_OPTIMIZATIONS` lets the collapsible onboarding card in `MainActivity` invite users to exempt Keynear from OEM background restriction when needed.
- `REQUEST_OBSERVE_COMPANION_DEVICE_PRESENCE` enables `KeynearCompanionDeviceService` hardware presence callbacks through `AssociationInfo` on API 33 to 35 and `ObservingDevicePresenceRequest` on API 36 and later.
- `NEARBY_WIFI_DEVICES`, `ACCESS_LOCAL_NETWORK`, `ACCESS_NETWORK_STATE`, and `INTERNET` support local `NsdManager` (`_keynear._udp.`) LAN discovery and direct peer-to-peer `iroh` QUIC transfers without cloud relays. On `compileSdk = 36`, `LanManager` handles local-network access.
  - It checks `ACCESS_LOCAL_NETWORK`.
  - It guards `NsdManager.DiscoveryRequest` and `FLAG_SHOW_PICKER` behind `SDK_INT >= 37` reflection.
  - It monitors `ConnectivityManager.NetworkCallback.onBlockedStatusChanged` (`ANDROID_NETWORK_BLOCKED_REASON_LNP`) to surface actionable settings and retry recovery.

## Lockscreen and Desktop Portal Boundaries

The lockscreen and desktop portals add the following boundaries.

- `KeynearTileService` inspects `isLocked` before changing visibility from Quick Settings. While locked, taps switch only between `Hidden` and `Paired devices only`. Activating `Everyone nearby` requires unlocking through `unlockAndRun`.
- `keynear-settings` accesses user-chosen files, folders, and incoming download locations through `ashpd` (`xdg-desktop-portal`) file-chooser requests rather than ambient filesystem indexing.
- File-manager context-menu integrations (`packaging/linux/integrations/nautilus`, `packaging/linux/integrations/dolphin`, `packaging/linux/integrations/yazi`) invoke `keynear-settings --share --` with explicit option-end delimiters and verified path arguments.

## Privacy and Licensing

Keynear limits data storage and exposure in the following ways.

- `devices.toml` stores only non-secret device metadata.
  - `device_id`, `display_name`, `role`, `public_key_fingerprint`.
  - `last_ble_address`, `last_lan_address`, `last_endpoint_id`.
  - `auto_accept_files`, `paired_at`.

  Pairwise `irk`, `reconnect_secret`, local static X25519 keys, and the local Ed25519 `iroh` key (`quic_ed25519`) stay inside `oo7` (`org.freedesktop.secrets`) on Linux and Android Keystore on mobile. Two-step unpair removes both the metadata and the stored secrets.
- Incoming Android file receives write directly to `MediaStore.Downloads` with `IS_PENDING = 1`. Keynear clears `IS_PENDING = 0` on completion or deletes the pending row on error or cancellation. It does not request broad external storage permissions.
- Clipboard policy inspects offered MIME targets before reading any payload bytes and skips `x-kde-passwordManagerHint`, `application/x-password`, `ClipDescription.EXTRA_IS_SENSITIVE`, and user-configured `ignoredMimeTypes`.
- Vega ranging and direct-path fallbacks require no new Android permission.
  - `RangingManager` (`android.ranging.RangingManager`) is used only on API 36 or later, only when both peers negotiated the capability, and only when the hardware reports support.
  - `AwarePairingConfig` and `WifiP2pPairingBootstrappingConfig` are used only through the existing `NEARBY_WIFI_DEVICES` and local-network permissions, and degrade to an unavailable result on unsupported hardware.
  - Audio handoff reads BlueZ and MPRIS2 D-Bus objects without storing credentials.
  - No peer-supplied peripheral identifier or ranging measurement is written to `config.toml` or `devices.toml`.
- The Android `DocumentsProvider` is reached through Storage Access Framework intents and URI permission grants. It is protected by `MANAGE_DOCUMENTS` and requires no new Android permission. Two boundaries apply.
  - Remote root and document names are bounded to 128 characters and pass through `sanitize_relative_path` before use, so a peer cannot inject a path or read another peer's tree.
  - Checkpoint bitmaps, rolling digests, and resume byte counts stay in memory and are never serialized to `config.toml` or `devices.toml`.
- Android OTP forwarding relies on Google Play Services SMS User Consent (`SmsRetriever.SEND_PERMISSION` and a `com.google.android.gms` `EXTRA_CONSENT_INTENT` check) without requesting `READ_SMS`. Verified OTPs stay in Linux daemon memory only and are never written to `config.toml`, `devices.toml`, `DaemonStatus`, or logs.
- Keynear ships under the [GNU General Public License v3.0](../../LICENSE).
