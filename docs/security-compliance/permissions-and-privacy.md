# Permissions and Privacy

`Keynear` requests only the OS permissions required for local peer discovery, notifications, and file delivery.

## Android Permissions

- `BLUETOOTH_SCAN`, `BLUETOOTH_CONNECT`, `BLUETOOTH_ADVERTISE`, and `NEARBY_WIFI_DEVICES`: Required for BLE presence advertisements, GATT pairing, and local Wi-Fi QUIC transport.
- `POST_NOTIFICATIONS`: Required for pairing verification codes, incoming drop consent prompts, and `Notification.ProgressStyle` transfer progress.
- `FOREGROUND_SERVICE_CONNECTED_DEVICE`: Keeps active peer transfers alive under `connectedDevice` foreground-service rules.

## Privacy & Licensing

- **Clipboard Filtering**: Clipboard offers carrying `x-kde-passwordManagerHint` or `application/x-password` are ignored before reading payload bytes.
- **Open Source License**: `Keynear` is distributed under the [GNU General Public License v3.0](../../LICENSE).

