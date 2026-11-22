# Troubleshooting

Use these checks to diagnose discovery, Noise XX pairing, keyring, local network, file transfer, and build issues.

## BLE Discovery and Pairing Issues

Check these items when Bluetooth discovery or pairing fails.

- Confirm Bluetooth is powered on with `bluetoothctl show`. Confirm `keynear daemon` is running with `systemctl --user status keynear.service`.
- Set visibility to `Paired devices only` or `Everyone nearby` rather than `Hidden`. `Everyone nearby` reverts to `Paired devices only` after 10 minutes (`600` seconds).
- When Android shows a Bluetooth warning banner, tap Allow Bluetooth to grant `BLUETOOTH_SCAN`, `BLUETOOTH_CONNECT`, and `BLUETOOTH_ADVERTISE`. Tap Open Bluetooth settings to power on the adapter.
- If a Noise XX pairing prompt disappears before confirmation, the 120-second pairing window (`PAIRING_TIMEOUT_SECS`) elapsed. Start pairing again from Nearby peers.
- On Linux build hosts (`keynear`), install `libdbus-1-dev` or enter `nix develop` when `dbus-sys` fails to link.

## Keyring and Paired Device Recovery

Use these checks when keyring or paired-device state is lost.

- Pairwise `irk`, `reconnect_secret`, local static keys, and `quic_ed25519` live in `oo7` (`org.freedesktop.secrets`) on Linux and Android Keystore on mobile. They are separate from `~/.config/keynear/devices.toml`.
- When BlueZ or `org.freedesktop.secrets` is unavailable at startup, `keynear` logs a warning and keeps local daemon state responsive.
- When a keyring entry is missing, reset, or out of sync after a reinstall, use the two-step Unpair and Confirm unpair actions in the Android app (or send `IpcRequest::UnpairDevice` over `daemon.sock`). Then pair again with the 6-digit Noise XX verification code.

## Android Local Network Access and File Transfers

Use these checks for Android local-network and transfer problems.

- On Android 17 and later (`SDK_INT >= 37`), `LanManager` handles local-network access.
  - It checks `ACCESS_LOCAL_NETWORK`.
  - It uses reflection-guarded `NsdManager.DiscoveryRequest` (`FLAG_SHOW_PICKER`) and watches `NetworkCallback.onBlockedStatusChanged` for `ANDROID_NETWORK_BLOCKED_REASON_LNP`.

  When local network access is blocked, tap Open settings in the Android warning banner or notification, then tap Retry.
- When multicast mDNS (`_keynear._udp.local.`) is unavailable on a restricted network or headless host (`mdns_degraded`), `keynear` and Android exchange ephemeral `LanEndpoint` coordinates over the encrypted BLE GATT characteristic `0xFE70` (`LAN_ENDPOINT_CHAR_UUID`).
- If an Android download is cancelled or fails mid-transfer, `TransferRepo` deletes the incomplete `MediaStore.Downloads` row (`IS_PENDING = 1`). `OrionNodeHandle` closes the detached `OwnedFd` automatically.
- If the Android Quick Settings tile prompts you to unlock the screen when switching to `Everyone nearby`, complete the device unlock (`unlockAndRun`). Toggles between `Hidden` and `Paired devices only` apply without unlocking.
- When transfers stall in the background on Android, expand the permissions onboarding card in `MainActivity`. Tap Allow on `Allow background running` (`REQUEST_IGNORE_BATTERY_OPTIMIZATIONS`) or open the app battery settings.

## Linux Desktop Tray, Notifications, and Portal Integration

Check these items when desktop integration fails.

- On headless Linux sessions or compositors without `org.kde.StatusNotifierWatcher` or `org.freedesktop.Notifications`, `keynear` logs `tray_unavailable` or `notifications_unavailable`. It keeps running local daemon IPC and QUIC/BLE continuity headlessly.
- If XDG portal file choosers fail to open from the tray `Send files...` item or `keynear send --picker`, verify that a desktop `xdg-desktop-portal` backend is running for your session (`ashpd`). Confirm that `daemon.sock` exists (`0o600` inside a `0o700` config directory) and verify configuration with `keynear settings validate`.
- Make sure Nautilus (`~/.local/share/nautilus/scripts/Send via Keynear`), Dolphin (`~/.local/share/kio/servicemenus/keynear-send.desktop`), or Yazi (`~/.config/yazi/plugins/keynear.yazi/main.lua`) can resolve `keynear` on `PATH`, for example under `~/.local/bin`.

## Resumable Transfers and Virtual Filesystem (`1.2.0-carina`)

Use these checks for resumable transfer and virtual filesystem issues.

- If an interrupted transfer restarts from chunk zero, verify that both peers advertise `CAP_RESUMABLE_TRANSFER` (`0x100`). The incoming `TransferCheckpoint` must carry a contiguous verified-chunk bitmap, a 32-byte rolling digest, and a byte count matching that bitmap. Keynear ignores a stale or duplicate checkpoint instead of applying it.
- If the on-demand mount does not appear, check the daemon log for `fuse_backend_unavailable`. Kernel FUSE and `fusermount3` must be present, and the optional `fuser` adapter must be built with the `fuse` Cargo feature. The default headless build intentionally reports unavailable instead of failing startup. Two options enable the mount.
  - `services.keynear.fuse.enable = true` with the NixOS module `nixosModules.default`.
  - `services.keynear.fuse = true` with the Home Manager module `homeManagerModules.default`.

  Either option sets `KEYNEAR_FUSE=1` and selects the `fuse`-enabled Nix package.
- If a paired Linux directory is missing from the Android `Files` app, confirm the provider is enabled for the `org.keynear.app.vfs.provider` authority and that at least one peer is paired. `queryRoots` returns an empty cursor when no peer is paired, and the provider adds no new Android permission.

## Notification Mirroring and Media Controls (`1.2.0-carina`)

Use these checks when mirrored notifications, call alerts, or media state do not appear.

- If no notification mirrors to the desktop, confirm notification access is granted to `org.keynear.app` and that mirroring is opted in. Excluded packages are filtered before any envelope is built, and no message body is written to disk.
- If reply, mark-as-read, or dismiss does nothing, confirm the desktop session bus is running and look for `notifications_unavailable` in the daemon log. The desktop keeps running when the bus is absent.
- If call controls do nothing, confirm `ANSWER_PHONE_CALLS` is granted. Decline falls back to `CallScreeningService` on permission-restricted builds, and the daemon logs `call_alert_action` for each applied action.
- If the `Now playing` group is empty, confirm a player publishes `org.mpris.MediaPlayer2` on the session bus. The daemon logs `media_bridge_unavailable` when no player exists and keeps the tray available.

## Wayland Clipboard Permissions

Check these items when clipboard sync misbehaves.

- Clipboard backends depend on the compositor.
  - Compositors with `ext-data-control-v1` or `zwlr-data-control-v1` support (KDE KWin, COSMIC, Sway, Hyprland) work directly through `crates/keynear/src/clipboard.rs`.
  - GNOME Mutter uses the shipped GNOME Shell D-Bus clipboard bridge in `packaging/gnome/keynear-clipboard@hydrafog.github.com/`. The bridge supports GNOME Shell 45–50 and ships as `packages.${system}.keynear-gnome-extension`.

  The module logs `clipboard_backend_selected` as `ext-data-control-v1`, `zwlr-data-control-v1`, `x11`, or `headless-unavailable`. Install and enable the bridge when it reports `headless-unavailable`.
- When clipboard images or password-manager entries appear skipped, verify `clipboardImagesEnabled` and `ignoredMimeTypes` (`x-kde-passwordManagerHint`, `application/x-password`, max `128` IPC entries) with `keynear settings show` or in `~/.config/keynear/config.toml`. On Android devices without Google Play services, SMS User Consent posts an availability status notification while manual clipboard sync continues working.
