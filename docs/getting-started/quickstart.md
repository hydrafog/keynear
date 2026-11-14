# Quickstart

Pair a Linux desktop running `keynear-linux` and `keynear-settings` with an Android phone running `org.keynear.app`.

## Pairing (`0.2.0-cygnus`)

1. Enable Bluetooth on both devices and start `keynear-linux`.
2. Switch visibility to Everyone nearby from `keynear-settings`, the Keynear system tray menu, the Keynear Quick Settings tile, or the Cycle visibility mode action in the Android app. Open discovery stays active for 10 minutes and counts down as `Everyone (MM:SS)` before it reverts to Paired devices only.
3. On Android, grant the requested `BLUETOOTH_SCAN`, `BLUETOOTH_CONNECT`, and `BLUETOOTH_ADVERTISE` permissions and notification access, then wait for the desktop to appear under Nearby peers and tap Pair.
4. Both devices run `Noise_XX_25519_ChaChaPoly_BLAKE2s` over BLE GATT with service `0xFE6D` and characteristics `0xFE6E` and `0xFE6F`, and each displays a 6-digit verification code.
5. Confirm that the two codes match, then tap Confirm or Approve within 120 seconds on both devices.
6. Non-secret device metadata is saved to `devices.toml`. Pairwise `irk` and `reconnect_secret` keys stay in `oo7` (`org.freedesktop.secrets`) on Linux and in Android Keystore with AES-256-GCM on the phone.

## Visibility Modes

- **Hidden** disables BLE presence advertisements and incoming discovery.
- **Paired devices only** advertises rotating 6-byte HMAC-SHA256 presence tokens and AES-256-GCM encrypted availability state that only paired peers can recognize.
- **Everyone nearby** enables 10-minute (`600`-second) open discovery for pairing new devices and reverts to paired devices only when the window closes.

## Next Milestone Capabilities (`0.3.0-orion`+)

- Direct peer-to-peer Wi-Fi QUIC transport over `rustls` with `X25519MLKEM768` carries Send via Keynear file delivery from `ShareActivity` and desktop file manager actions once pairing completes in `0.3.0-orion`.
