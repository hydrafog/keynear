# Quickstart

Follow these steps to pair a Linux desktop (`keynear-linux` + `keynear-settings`) and an Android phone (`org.keynear.app`).

## Pairing (`0.2.0-cygnus`)

1. Ensure Bluetooth is enabled on both devices and start `keynear-linux`.
2. Switch visibility to **Everyone nearby** from `keynear-settings`, the `Keynear` system tray menu, the **Keynear** Quick Settings tile, or the Android app (**Cycle visibility mode**). Open discovery stays active for 10 minutes (`Everyone (MM:SS)`) before reverting to **Paired devices only**.
3. On Android, grant requested Bluetooth (`BLUETOOTH_SCAN`, `BLUETOOTH_CONNECT`, `BLUETOOTH_ADVERTISE`) and notification permissions when prompted, then wait for the desktop to appear under **Nearby peers** and tap **Pair**.
4. Both devices run `Noise_XX_25519_ChaChaPoly_BLAKE2s` over BLE GATT (`0xFE6D` / `0xFE6E` / `0xFE6F`) and display a 6-digit verification code.
5. Verify both 6-digit codes match, then tap **Confirm** / **Approve** within 120 seconds on both devices.
6. Non-secret device metadata is saved to `devices.toml`, while pairwise `irk` and `reconnect_secret` keys are stored in `oo7` (`org.freedesktop.secrets`) on Linux and Android Keystore (AES-256-GCM) on the phone.

## Visibility Modes

- **Hidden**: Disables BLE presence advertisements and incoming discovery.
- **Paired devices only**: Advertises rotating 6-byte HMAC-SHA256 presence tokens and AES-256-GCM encrypted availability state recognizable only by paired peers.
- **Everyone nearby**: Enables 10-minute (`600`-second) open discovery for pairing new devices, then automatically reverts to **Paired devices only**.

## Next Milestone Capabilities (`0.3.0-orion`+)

- Direct peer-to-peer Wi-Fi QUIC transport (`rustls` `X25519MLKEM768`) for **Send via Keynear** file delivery (`ShareActivity` / desktop file manager actions) activates after pairing in `0.3.0-orion`.
