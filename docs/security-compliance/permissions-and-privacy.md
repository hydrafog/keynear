# Permissions and Privacy

`Keynear` requests only the OS permissions that local peer discovery, notifications, and upcoming continuity transport need.

## Android Permissions (`0.2.0-cygnus`)

- `BLUETOOTH_SCAN` (`usesPermissionFlags="neverForLocation"`), `BLUETOOTH_CONNECT`, and `BLUETOOTH_ADVERTISE` support `0xFE6D` BLE presence advertisements, `BleScanReceiver` `PendingIntent` background scanning, and Noise XX GATT pairing. Keynear never requests fine location.
- `POST_NOTIFICATIONS` (API 33 and later) supports `PairingNotificationManager` 6-digit pairing verification prompts and foreground service status visibility.
- `FOREGROUND_SERVICE` and `FOREGROUND_SERVICE_CONNECTED_DEVICE` keep `KeynearForegroundService` active only after the required Bluetooth runtime permissions are granted.
- `REQUEST_OBSERVE_COMPANION_DEVICE_PRESENCE` enables `KeynearCompanionDeviceService` hardware presence callbacks through `AssociationInfo` on API 33 to 35 and `ObservingDevicePresenceRequest` on API 36 and later.
- `NEARBY_WIFI_DEVICES` (`0.3.0-orion`+) is declared for the upcoming local Wi-Fi QUIC transport.

## Privacy and Licensing

- `devices.toml` stores only non-secret device metadata (`device_id`, `display_name`, `role`, `public_key_fingerprint`, `last_ble_address`, `paired_at`). The `irk`, `reconnect_secret`, and local static private keys stay inside `oo7` (`org.freedesktop.secrets`) on Linux and Android Keystore on mobile. Two-step unpair removes both the metadata and the stored secrets.
- Clipboard policy (`0.3.0-orion`+) ignores offers that carry `x-kde-passwordManagerHint` or `application/x-password` before any payload bytes are read.
- `Keynear` ships under the [GNU General Public License v3.0](../../LICENSE).
