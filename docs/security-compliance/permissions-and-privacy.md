# Permissions and Privacy

`Keynear` requests only the OS permissions required for local peer discovery, notifications, and upcoming continuity transport.

## Android Permissions (`0.2.0-cygnus`)

- `BLUETOOTH_SCAN` (`usesPermissionFlags="neverForLocation"`), `BLUETOOTH_CONNECT`, and `BLUETOOTH_ADVERTISE`: Required for `0xFE6D` BLE presence advertisements, `BleScanReceiver` `PendingIntent` background scanning, and Noise XX GATT pairing. Keynear does not request fine location permissions.
- `POST_NOTIFICATIONS` (API 33+): Required for `PairingNotificationManager` 6-digit pairing verification prompts and foreground service status visibility.
- `FOREGROUND_SERVICE` and `FOREGROUND_SERVICE_CONNECTED_DEVICE`: Keeps `KeynearForegroundService` active only after required Bluetooth runtime permissions are granted.
- `REQUEST_OBSERVE_COMPANION_DEVICE_PRESENCE`: Supports `KeynearCompanionDeviceService` hardware presence callbacks (`AssociationInfo` on API 33–35 and `ObservingDevicePresenceRequest` on API 36+).
- `NEARBY_WIFI_DEVICES` (`0.3.0-orion`+): Declared for upcoming local Wi-Fi QUIC transport.

## Privacy & Licensing

- **Secret Separation**: `devices.toml` stores only non-secret device metadata (`device_id`, `display_name`, `role`, `public_key_fingerprint`, `last_ble_address`, `paired_at`), while `irk`, `reconnect_secret`, and local static private keys stay inside `oo7` (`org.freedesktop.secrets`) on Linux and Android Keystore on mobile. Two-step unpair removes both metadata and stored secrets.
- **Clipboard Filtering (`0.3.0-orion`+)**: Clipboard policy enforces ignoring offers carrying `x-kde-passwordManagerHint` or `application/x-password` before reading payload bytes.
- **Open Source License**: `Keynear` is distributed under the [GNU General Public License v3.0](../../LICENSE).
