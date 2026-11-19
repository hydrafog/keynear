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
- If the Android Quick Settings tile prompts to unlock the screen when switching to `Everyone nearby`, complete the device unlock (`unlockAndRun`); toggles between `Hidden` and `Paired devices only` apply without unlocking.
- When transfers stall in the background on Android, expand the permissions onboarding card in `MainActivity` and tap Allow on `Allow background running` (`REQUEST_IGNORE_BATTERY_OPTIMIZATIONS`) or open app battery settings.

## Linux Desktop Tray, Notifications, and Portal Integration

- On headless Linux sessions or compositors without `org.kde.StatusNotifierWatcher` or `org.freedesktop.Notifications`, `keynear-linux` logs `tray_unavailable` or `notifications_unavailable` and continues running local daemon IPC and QUIC/BLE continuity headlessly.
- If GTK4 file or folder pickers fail to open in `keynear-settings`, verify that a desktop `xdg-desktop-portal` backend is running for your session (`ashpd`) and confirm `daemon.sock` exists (`0o600` inside a `0o700` config directory) or retry via the inline offline banner.
- Ensure Nautilus (`~/.local/share/nautilus/scripts/Send via Keynear`), Dolphin (`~/.local/share/kio/servicemenus/keynear-send.desktop`), or Yazi (`~/.config/yazi/plugins/keynear.yazi/main.lua`) can resolve `keynear-settings` on `PATH` (for example under `~/.local/bin`).

## Wayland Clipboard Permissions (`0.5.0-draco`)

- Compositors with `ext-data-control-v1` or `zwlr-data-control-v1` support (KDE KWin, COSMIC, Sway, Hyprland) work directly through `crates/keynear-linux/src/clipboard.rs`, which logs `clipboard_backend_selected` (`ext-data-control-v1`, `zwlr-data-control-v1`, `x11`, or `headless-unavailable`). GNOME Mutter requires the GNOME Shell D-Bus clipboard bridge arriving in `1.0.0-vega`.
- When clipboard images or password-manager entries appear skipped, verify `clipboardImagesEnabled` and `ignoredMimeTypes` (`x-kde-passwordManagerHint`, `application/x-password`, max `128` IPC entries) in `keynear-settings`. On Android devices without Google Play services, SMS User Consent posts an availability status notification while manual clipboard sync continues working.
