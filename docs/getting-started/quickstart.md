# Quickstart

Follow these steps to pair a Linux desktop and an Android phone and send your first file.

## Pairing

1. Open the `Keynear` system tray menu on Linux and select **Pair new device…**, or tap the **Keynear** Quick Settings tile on Android to set visibility to **Everyone nearby**.
2. Both devices exchange ephemeral X25519 keys over BLE GATT using `Noise_XX_25519_ChaChaPoly_BLAKE2s`.
3. Confirm that the six-digit verification code matches on both native notifications and tap **Approve**.
4. Pairwise keys are stored in `oo7` (`org.freedesktop.secrets`) on Linux and Android Keystore on the phone.

## Visibility Modes

- **Hidden**: Disables BLE presence advertisements and incoming discovery.
- **Paired devices only**: Advertises rotating HMAC-SHA256 presence tokens recognizable only by paired peers.
- **Everyone nearby**: Enables 10-minute open discovery for pairing new devices.

## Sending Files

- **Linux to Android**: Right-click a file in your file manager and select **Send via Keynear**, or choose **Send files…** from the tray menu.
- **Android to Linux**: Tap **Share** in any Android app and select the target desktop from the Android Share Sheet.

