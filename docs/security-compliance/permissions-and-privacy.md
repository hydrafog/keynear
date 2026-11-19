# Permissions and Privacy

`Keynear` (`0.5.0-draco`) requests only the OS permissions needed for local peer discovery, notifications, clipboard continuity, and direct Wi-Fi QUIC transfers.

## Android Permissions

- `BLUETOOTH_SCAN`, `BLUETOOTH_CONNECT`, and `BLUETOOTH_ADVERTISE` support `0xFE6D` BLE presence advertisements, `BleScanReceiver` `PendingIntent` background scanning, Noise XX GATT pairing (`0xFE6E`/`0xFE6F`), and encrypted LAN endpoint exchange (`0xFE70`). Keynear never requests fine location.
- `POST_NOTIFICATIONS` (API 33 and later) supports `PairingNotificationManager` 6-digit pairing prompts, incoming file transfer consent (`Accept`/`Decline`), live `ProgressStyle` / `NotificationCompat` transfer updates, completion/error alerts, local-network-blocked alerts, and silent `connectedDevice` continuity visibility.
- `FOREGROUND_SERVICE` and `FOREGROUND_SERVICE_CONNECTED_DEVICE` keep `KeynearForegroundService` active for BLE and `OrionNodeHandle` transport after Bluetooth permissions are granted.
- `RUN_USER_INITIATED_JOBS` allows `KeynearTransferJobService` to schedule `User-Initiated Data Transfer Jobs` (`setUserInitiated(true)` on API 34 and later) for active file transfers.
- `REQUEST_IGNORE_BATTERY_OPTIMIZATIONS` lets the collapsible onboarding card in `MainActivity` invite users to exempt Keynear from OEM background restriction when needed.
- `REQUEST_OBSERVE_COMPANION_DEVICE_PRESENCE` enables `KeynearCompanionDeviceService` hardware presence callbacks through `AssociationInfo` on API 33 to 35 and `ObservingDevicePresenceRequest` on API 36 and later.
- `NEARBY_WIFI_DEVICES`, `ACCESS_LOCAL_NETWORK`, `ACCESS_NETWORK_STATE`, and `INTERNET` support local `NsdManager` (`_keynear._udp.`) LAN discovery and direct peer-to-peer `iroh` QUIC transfers without cloud relays. On `compileSdk = 36`, `LocalNetworkManager` checks `ACCESS_LOCAL_NETWORK`, guards `NsdManager.DiscoveryRequest` and `FLAG_SHOW_PICKER` behind `SDK_INT >= 37` reflection, and monitors `ConnectivityManager.NetworkCallback.onBlockedStatusChanged` (`ANDROID_NETWORK_BLOCKED_REASON_LNP`) to surface actionable settings and retry recovery.

## Lockscreen and Desktop Portal Boundaries

- `KeynearTileService` inspects `isLocked` before mutating visibility from Quick Settings: while locked, taps switch only between `Hidden` and `Paired devices only`, and activating `Everyone nearby` requires unlocking through `unlockAndRun`.
- `keynear-settings` accesses user-chosen files, folders, and incoming download locations through `ashpd` (`xdg-desktop-portal`) file-chooser requests rather than ambient filesystem indexing.
- File-manager context-menu integrations (`integrations/nautilus`, `integrations/dolphin`, `integrations/yazi`) invoke `keynear-settings --share --` with explicit option-end delimiters and verified path arguments.

## Privacy and Licensing

- `devices.toml` stores only non-secret device metadata (`device_id`, `display_name`, `role`, `public_key_fingerprint`, `last_ble_address`, `last_lan_address`, `last_endpoint_id`, `auto_accept_files`, `paired_at`). Pairwise `irk`, `reconnect_secret`, local static X25519 keys, and the local Ed25519 `iroh` key (`quic_ed25519`) stay inside `oo7` (`org.freedesktop.secrets`) on Linux and Android Keystore on mobile. Two-step unpair removes both the metadata and the stored secrets.
- Incoming Android file receives write directly to `MediaStore.Downloads` with `IS_PENDING = 1`, clearing `IS_PENDING = 0` upon completion or deleting the pending row on error or cancellation without requesting broad external storage permissions.
- Clipboard policy (`0.5.0-draco`) inspects offered MIME targets before reading any payload bytes and skips `x-kde-passwordManagerHint`, `application/x-password`, `ClipDescription.EXTRA_IS_SENSITIVE`, and user-configured `ignoredMimeTypes`.
- Android OTP forwarding relies on Google Play Services SMS User Consent (`SmsRetriever.SEND_PERMISSION` + `com.google.android.gms` `EXTRA_CONSENT_INTENT` check) without requesting `READ_SMS`. Verified OTPs stay in Linux daemon memory only and are never written to `config.toml`, `devices.toml`, `DaemonStatus`, or logs.
- `Keynear` ships under the [GNU General Public License v3.0](../../LICENSE).
