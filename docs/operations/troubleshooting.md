# Troubleshooting

Use these checks to diagnose discovery, Noise XX pairing, keyring, local network, and build issues.

## BLE Discovery and Pairing Issues

- Confirm Bluetooth is powered on with `bluetoothctl show` and `keynear-linux` is running with `systemctl --user status keynear.service`.
- Verify that visibility is set to `Paired devices only` or `Everyone nearby` rather than `Hidden`. `Everyone nearby` reverts to `Paired devices only` after 10 minutes (`600` seconds).
- When Android shows a Bluetooth warning banner, tap Allow Bluetooth to grant `BLUETOOTH_SCAN`, `BLUETOOTH_CONNECT`, and `BLUETOOTH_ADVERTISE`, or tap Open Bluetooth settings to power on the adapter.
- If a Noise XX pairing prompt disappears before confirmation, the 120-second pairing window (`PAIRING_TIMEOUT_SECS`) elapsed. Start pairing again from Nearby peers.
- On Linux build hosts (`keynear-core` with `--features linux-oo7`, `keynear-linux`, and `keynear-settings`), install `libdbus-1-dev` or enter `nix develop` when `dbus-sys` fails to link.

## Keyring and Paired Device Recovery

- Pairwise `irk`, `reconnect_secret`, and local static keys live in `oo7` (`org.freedesktop.secrets`) on Linux and Android Keystore on mobile, separate from `~/.config/keynear/devices.toml`.
- When BlueZ or `org.freedesktop.secrets` is unavailable at startup, `keynear-linux` logs a warning and keeps local daemon state responsive.
- When a keyring entry is missing, reset, or out of sync after a reinstall, use the two-step Unpair and Confirm unpair actions in `keynear-settings` or the Android app, then pair again with the 6-digit Noise XX verification code.

## Android Local Network Access (`0.3.0-orion`+)

- On Android 17 and later, grant `ACCESS_LOCAL_NETWORK` when local Wi-Fi QUIC transport activates in `0.3.0-orion`.

## Wayland Clipboard Permissions (`0.3.0-orion`+)

- Compositors with `ext-data-control-v1` or `zwlr-data-control-v1` support (KDE KWin, COSMIC, Sway, Hyprland) work directly once clipboard sync activates. GNOME Mutter requires the GNOME Shell D-Bus clipboard bridge.
