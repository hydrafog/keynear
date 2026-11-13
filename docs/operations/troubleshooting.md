# Troubleshooting

Use these checks to diagnose discovery, local network, clipboard, and keyring issues.

## BLE Discovery Issues

- Confirm Bluetooth is powered on (`bluetoothctl show`) and `keynear-linux` is running (`systemctl --user status keynear.service`).
- Verify visibility is set to `Paired devices only` or `Everyone nearby` rather than `Hidden`.

## Android Local Network Access

- On Android 17+, ensure `ACCESS_LOCAL_NETWORK` is granted. If local discovery is blocked (`ANDROID_NETWORK_BLOCKED_REASON_LNP`), use the `NsdManager` picker fallback or grant local network access in system settings.

## Wayland Clipboard Permissions

- Compositors supporting `ext-data-control-v1` or `zwlr-data-control-v1` (KDE KWin, COSMIC, Sway, Hyprland) work directly. Under GNOME Mutter, enable the lightweight GNOME Shell D-Bus clipboard bridge.

## Keyring Recovery

- Pairwise `irk` and `reconnectSecret` keys live in `oo7` (`org.freedesktop.secrets`) on Linux and Android Keystore on mobile. If a keyring entry is missing or migrated, unpair the peer in settings and re-pair with the 6-digit Noise XX verification code.

