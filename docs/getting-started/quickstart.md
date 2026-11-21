# Quickstart

Pair a Linux desktop running `keynear-linux` and `keynear-settings` with an Android phone running `org.keynear.app` (`1.0.0-vega`), then share files, clipboard items, and one-time codes over direct Wi-Fi QUIC.

## Pairing

1. Enable Bluetooth on both devices and start `keynear-linux`.
2. Switch visibility to Everyone nearby from the Linux system tray (`ksni`), `keynear-settings`, the Android Quick Settings tile (`KeynearTileService`, unlocking first if the screen is locked), or the Cycle visibility mode action in `MainActivity`. Open discovery stays active for 10 minutes (`600` seconds) and counts down as `Everyone (MM:SS)` before it reverts to Paired devices only.
3. On Android, use the onboarding permissions card in `MainActivity` to grant `NEARBY_DEVICES`, `POST_NOTIFICATIONS`, and background running permission, wait for the desktop to appear under Nearby peers, and tap Pair.
4. Both devices run `Noise_XX_25519_ChaChaPoly_BLAKE2s` over BLE GATT (`0xFE6D`, `0xFE6E`, `0xFE6F`) and display a 6-digit verification code.
5. Confirm that the two codes match, then tap Confirm or Approve within 120 seconds on both devices.
6. Non-secret metadata is saved to `devices.toml`, while `irk`, `reconnect_secret`, static X25519 keys, and `quic_ed25519` remain in `oo7` (`org.freedesktop.secrets`) on Linux and Android Keystore on mobile.

## Visibility Modes

- **Hidden** disables BLE presence advertisements and incoming discovery.
- **Paired devices only** advertises rotating 6-byte HMAC-SHA256 presence tokens and AES-256-GCM encrypted availability state that only paired peers recognize. While the Android lockscreen is active, `KeynearTileService` toggles directly between Hidden and Paired devices only.
- **Everyone nearby** enables 10-minute (`600`-second) open discovery for pairing new devices and reverts to paired devices only when the window closes. Switching to Everyone nearby from a locked Android screen prompts to unlock via `unlockAndRun`.

## Sharing Files over Wi-Fi QUIC

1. Connect both paired devices to a shared local Wi-Fi or LAN network so `mdns-sd` / `NsdManager` (`_keynear._udp.`) or encrypted BLE GATT (`0xFE70`) can exchange ephemeral `LanEndpoint` addresses.
2. From Android, share files or text directly to a paired peer target via system Direct Share (`DirectShareManager`) or the `ShareActivity` bottom sheet (`ACTION_SEND` or `ACTION_SEND_MULTIPLE`), and tap Send.
3. From Linux, send files or folders via the `ksni` tray, the `ashpd` file/folder choosers in `keynear-settings` (`--send-picker`), Nautilus/Dolphin/Yazi context menus (`keynear-settings --share -- <path>...`), or `keynear-linux --send <device_id> <path>...` to stream files in 60 KiB chunks or directories as uncompressed `STORED` zip archives over direct `iroh` QUIC (`X25519MLKEM768`, `RelayMode::Disabled`).
4. Incoming drops wait in `PendingConsent` for `Accept` or `Decline` via desktop (`zbus`) or Android notifications, `keynear-settings`, `MainActivity`, or CLI (`keynear-linux --transfers`, `--accept <transfer_id>`, `--decline <transfer_id>`, `--cancel <transfer_id>`), unless `auto_accept_from_paired` or per-device `Auto-accept files` is enabled.

## Clipboard and OTP Continuity

- Desktop clipboard changes sync in real time over Wayland (`ext-data-control-v1` / `zwlr-data-control-v1`) or X11 to the paired Android phone in the background, skipping password-manager MIME targets (`x-kde-passwordManagerHint`, `application/x-password`) and `ignoredMimeTypes`.
- Use **Copy phone clipboard** on the Android continuity notification or Quick Settings tile (`ClipboardTileService`) to read the phone clipboard and send it to the desktop.
- Confirm Android SMS User Consent prompts to forward verified one-time codes to the desktop clipboard and enable **Copy latest OTP** in the Linux tray menu.

## Audio Handoff and Fallbacks

- `1.0.0-vega` is the current production release. Smart audio handoff pauses MPRIS2 playback and releases the matching `bap_sink`, `a2dp_sink`, or `hfp_hf` profile before the phone claims a shared headset, then reclaims it on release.
- When no shared Wi-Fi network exists, peers negotiate Wi-Fi Aware NAN ahead of Wi-Fi Direct R2. `RangingManager` closeness gating activates only when both peers advertise the capability, the hardware supports it, and the device runs API 36 or later.
- Under GNOME Mutter, install the optional bridge in `packaging/gnome/keynear-clipboard@hydrafog.github.com/` for background clipboard sync. Every backend degrades to a typed unavailable state when its OS service is missing.
