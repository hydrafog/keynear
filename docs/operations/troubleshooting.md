# Troubleshooting

Use these checks to diagnose discovery, Noise XX pairing, keyring, local network, file transfer, and build issues.

## BLE Discovery and Pairing Issues

- Confirm Bluetooth is powered on with `bluetoothctl show` and `keynear-linux` is running with `systemctl --user status keynear.service`.
- Verify that visibility is set to `Paired devices only` or `Everyone nearby` rather than `Hidden`. `Everyone nearby` reverts to `Paired devices only` after 10 minutes (`600` seconds).
- When Android shows a Bluetooth warning banner, tap Allow Bluetooth to grant `BLUETOOTH_SCAN`, `BLUETOOTH_CONNECT`, and `BLUETOOTH_ADVERTISE`, or tap Open Bluetooth settings to power on the adapter.
- If a Noise XX pairing prompt disappears before confirmation, the 120-second pairing window (`PAIRING_TIMEOUT_SECS`) elapsed. Start pairing again from Nearby peers.
- On Linux build hosts (`keynear-core` with `--features linux-oo7`, `keynear-linux`, and `keynear-settings`), install `libdbus-1-dev` or enter `nix develop` when `dbus-sys` fails to link.

## Keyring and Paired Device Recovery

- Pairwise `irk`, `reconnect_secret`, local static keys, and `quic_ed25519` live in `oo7` (`org.freedesktop.secrets`) on Linux and Android Keystore on mobile, separate from `~/.config/keynear/devices.toml`.
- When BlueZ or `org.freedesktop.secrets` is unavailable at startup, `keynear-linux` logs a warning and keeps local daemon state responsive.
- When a keyring entry is missing, reset, or out of sync after a reinstall, use the two-step Unpair and Confirm unpair actions in `keynear-settings` or the Android app, then pair again with the 6-digit Noise XX verification code.

## Android Local Network Access and File Transfers

- On Android 17 and later (`SDK_INT >= 37`), `LocalNetworkManager` checks `ACCESS_LOCAL_NETWORK`, uses reflection-guarded `NsdManager.DiscoveryRequest` (`FLAG_SHOW_PICKER`), and watches `NetworkCallback.onBlockedStatusChanged` for `ANDROID_NETWORK_BLOCKED_REASON_LNP`. When local network access is blocked, tap Open settings in the Android warning banner or notification to allow local network access, then tap Retry.
- When multicast mDNS (`_keynear._udp.local.`) is unavailable on a restricted network or headless host (`mdns_degraded`), `keynear-linux` and Android fall back to exchanging ephemeral `LanEndpoint` coordinates over the encrypted BLE GATT characteristic `0xFE70` (`LAN_ENDPOINT_CHAR_UUID`).
- If an Android download is cancelled or fails mid-transfer, `CoreClient` deletes the incomplete `MediaStore.Downloads` row (`IS_PENDING = 1`) and `OrionNodeHandle` closes the detached `OwnedFd` automatically.
- On Linux, verify `daemon.sock` exists inside the configuration directory (`0o600` socket inside a `0o700` directory) if `keynear-linux --transfers` or `keynear-settings` reports the daemon is unreachable.

## Wayland Clipboard Permissions (Upcoming `0.5.0-draco`)

- Compositors with `ext-data-control-v1` or `zwlr-data-control-v1` support (KDE KWin, COSMIC, Sway, Hyprland) work directly once clipboard sync activates in `0.5.0-draco`. GNOME Mutter requires the GNOME Shell D-Bus clipboard bridge in `1.0.0-vega`.
