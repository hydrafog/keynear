# Quickstart

Pair a Linux desktop running `keynear-linux` and `keynear-settings` with an Android phone running `org.keynear.app` (`0.3.0-orion`), then share files over direct Wi-Fi QUIC.

## Pairing

1. Enable Bluetooth on both devices and start `keynear-linux`.
2. Switch visibility to Everyone nearby from `keynear-settings`, the Android Quick Settings tile (`KeynearTileService`), or the Cycle visibility mode action in `MainActivity`. Open discovery stays active for 10 minutes (`600` seconds) and counts down as `Everyone (MM:SS)` before it reverts to Paired devices only.
3. On Android, grant the requested `BLUETOOTH_SCAN`, `BLUETOOTH_CONNECT`, `BLUETOOTH_ADVERTISE`, and notification permissions, wait for the desktop to appear under Nearby peers, and tap Pair.
4. Both devices run `Noise_XX_25519_ChaChaPoly_BLAKE2s` over BLE GATT (`0xFE6D`, `0xFE6E`, `0xFE6F`) and display a 6-digit verification code.
5. Confirm that the two codes match, then tap Confirm or Approve within 120 seconds on both devices.
6. Non-secret metadata is saved to `devices.toml`, while `irk`, `reconnect_secret`, static X25519 keys, and `quic_ed25519` remain in `oo7` (`org.freedesktop.secrets`) on Linux and Android Keystore on mobile.

## Visibility Modes

- **Hidden** disables BLE presence advertisements and incoming discovery.
- **Paired devices only** advertises rotating 6-byte HMAC-SHA256 presence tokens and AES-256-GCM encrypted availability state that only paired peers recognize.
- **Everyone nearby** enables 10-minute (`600`-second) open discovery for pairing new devices and reverts to paired devices only when the window closes.

## Sharing Files over Wi-Fi QUIC

1. Connect both paired devices to a shared local Wi-Fi or LAN network so `mdns-sd` / `NsdManager` (`_keynear._udp.`) or encrypted BLE GATT (`0xFE70`) can exchange ephemeral `LanEndpoint` addresses.
2. From Android, share files or text through `ShareActivity` (`ACTION_SEND` or `ACTION_SEND_MULTIPLE`), select a paired device, and tap Send.
3. From Linux, run `keynear-linux --send <device_id> <path>...` to stream files in 60 KiB chunks or directories as uncompressed `STORED` zip archives over direct `iroh` QUIC (`X25519MLKEM768`, `RelayMode::Disabled`).
4. Incoming drops wait in `PendingConsent` for `Accept` or `Decline` via notifications, `keynear-settings`, `MainActivity`, or CLI (`keynear-linux --transfers`, `--accept <transfer_id>`, `--decline <transfer_id>`, `--cancel <transfer_id>`), unless `Auto-accept files` is enabled for the sending device.

## Upcoming Milestones

- `0.4.0-aquila` adds `ksni` `StatusNotifierItem` tray menus, `ashpd` (`xdg-desktop-portal`) file selection, Nautilus/Dolphin/Yazi context menus, Direct Share targets, and `Notification.ProgressStyle` Live Updates.
- `0.5.0-draco` adds Wayland/X11 clipboard synchronization and Android SMS User Consent OTP sync.
- `1.0.0-vega` adds PipeWire/MPRIS2 Bluetooth audio handoff, the GNOME Mutter bridge, Wi-Fi Aware/Direct fallbacks, and `RangingManager` closeness gating.
