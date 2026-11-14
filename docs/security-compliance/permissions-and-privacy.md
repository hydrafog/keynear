# Permissions and Privacy

`Keynear` (`0.3.0-orion`) requests only the OS permissions needed for local peer discovery, notifications, and direct Wi-Fi QUIC transfers.

## Android Permissions

- `BLUETOOTH_SCAN`, `BLUETOOTH_CONNECT`, and `BLUETOOTH_ADVERTISE` support `0xFE6D` BLE presence advertisements, `BleScanReceiver` `PendingIntent` background scanning, Noise XX GATT pairing (`0xFE6E`/`0xFE6F`), and encrypted LAN endpoint exchange (`0xFE70`). Keynear never requests fine location.
- `POST_NOTIFICATIONS` (API 33 and later) supports `PairingNotificationManager` 6-digit pairing prompts, incoming file transfer consent (`Accept`/`Decline`), completion/error notifications, local-network-blocked alerts, and foreground service status visibility.
- `FOREGROUND_SERVICE` and `FOREGROUND_SERVICE_CONNECTED_DEVICE` keep `KeynearForegroundService` active for BLE and `OrionNodeHandle` transport after Bluetooth permissions are granted.
- `REQUEST_OBSERVE_COMPANION_DEVICE_PRESENCE` enables `KeynearCompanionDeviceService` hardware presence callbacks through `AssociationInfo` on API 33 to 35 and `ObservingDevicePresenceRequest` on API 36 and later.
- `NEARBY_WIFI_DEVICES`, `ACCESS_LOCAL_NETWORK`, `ACCESS_NETWORK_STATE`, and `INTERNET` support local `NsdManager` (`_keynear._udp.`) LAN discovery and direct peer-to-peer `iroh` QUIC transfers without cloud relays. On `compileSdk = 36`, `LocalNetworkManager` checks `ACCESS_LOCAL_NETWORK`, guards `NsdManager.DiscoveryRequest` and `FLAG_SHOW_PICKER` behind `SDK_INT >= 37` reflection, and monitors `ConnectivityManager.NetworkCallback.onBlockedStatusChanged` (`ANDROID_NETWORK_BLOCKED_REASON_LNP`) to surface actionable settings and retry recovery.

## Privacy and Licensing

- `devices.toml` stores only non-secret device metadata (`device_id`, `display_name`, `role`, `public_key_fingerprint`, `last_ble_address`, `last_lan_address`, `last_endpoint_id`, `auto_accept_files`, `paired_at`). Pairwise `irk`, `reconnect_secret`, local static X25519 keys, and the local Ed25519 `iroh` key (`quic_ed25519`) stay inside `oo7` (`org.freedesktop.secrets`) on Linux and Android Keystore on mobile. Two-step unpair removes both the metadata and the stored secrets.
- Incoming Android file receives write directly to `MediaStore.Downloads` with `IS_PENDING = 1`, clearing `IS_PENDING = 0` upon completion or deleting the pending row on error or cancellation without requesting broad external storage permissions.
- Upcoming clipboard policy (`0.5.0-draco`) skips offers carrying `x-kde-passwordManagerHint` or `application/x-password` before payload bytes are read.
- `Keynear` ships under the [GNU General Public License v3.0](../../LICENSE).
