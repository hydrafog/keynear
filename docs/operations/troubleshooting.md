# Troubleshooting

Use these checks to diagnose discovery, Noise XX pairing, keyring, local network, and build issues.

## BLE Discovery & Pairing Issues

- Confirm Bluetooth is powered on (`bluetoothctl show`) and `keynear-linux` is running (`systemctl --user status keynear.service`).
- Verify visibility is set to `Paired devices only` or `Everyone nearby` rather than `Hidden`. Note that `Everyone nearby` automatically reverts to `Paired devices only` after 10 minutes (`600` seconds).
- If Android displays a Bluetooth warning banner, tap **Allow Bluetooth** to grant `BLUETOOTH_SCAN`/`BLUETOOTH_CONNECT`/`BLUETOOTH_ADVERTISE` or **Open Bluetooth settings** to power on the adapter.
- If a Noise XX pairing prompt disappears before confirmation, the `120`-second pairing window (`PAIRING_TIMEOUT_SECS`) elapsed; re-initiate pairing from **Nearby peers**.
- On Linux build hosts (`keynear-core` `--features linux-oo7`, `keynear-linux`, `keynear-settings`), install `libdbus-1-dev` (or enter `nix develop`) if `dbus-sys` fails to link.

## Keyring & Paired Device Recovery

- Pairwise `irk`, `reconnect_secret`, and local static keys live in `oo7` (`org.freedesktop.secrets`) on Linux and Android Keystore on mobile, separate from `~/.config/keynear/devices.toml`.
- If BlueZ or `org.freedesktop.secrets` is unavailable at startup, `keynear-linux` degrades gracefully and logs a warning while keeping local daemon state responsive.
- If a keyring entry is missing, reset, or out of sync after reinstall, use the two-step **Unpair** -> **Confirm unpair** action in `keynear-settings` or the Android app and re-pair with the 6-digit Noise XX verification code.

## Android Local Network Access (`0.3.0-orion`+)

- On Android 17+, ensure `ACCESS_LOCAL_NETWORK` is granted when local Wi-Fi QUIC transport activates in `0.3.0-orion`.

## Wayland Clipboard Permissions (`0.3.0-orion`+)

- Compositors supporting `ext-data-control-v1` or `zwlr-data-control-v1` (KDE KWin, COSMIC, Sway, Hyprland) work directly once clipboard sync activates; GNOME Mutter requires the GNOME Shell D-Bus clipboard bridge.
