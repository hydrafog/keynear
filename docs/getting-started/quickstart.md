# Quickstart

Pair a Linux desktop running `keynear-linux` and `keynear-settings` with an Android phone running `org.keynear.app` (`1.1.0-perseus`). You can then share files, clipboard items, and one-time codes over direct Wi-Fi QUIC.

## Pairing

Follow these steps to pair the two devices.

1. Enable Bluetooth on both devices and start `keynear-linux`.
2. Switch visibility to Everyone nearby from the Linux system tray (`ksni`), `keynear-settings`, the Android Quick Settings tile (`KeynearTileService`), or the Cycle visibility mode action in `MainActivity`. Two details apply.
   - Unlock the screen first if you use the Android tile while the screen is locked.
   - Open discovery stays active for 10 minutes (`600` seconds) and counts down as `Everyone (MM:SS)` before visibility reverts to Paired devices only.
3. On Android, grant `NEARBY_DEVICES`, `POST_NOTIFICATIONS`, and background running permission from the onboarding permissions card in `MainActivity`. Wait for the desktop to appear under Nearby peers, then tap Pair.
4. Both devices run `Noise_XX_25519_ChaChaPoly_BLAKE2s` over BLE GATT (`0xFE6D`, `0xFE6E`, `0xFE6F`) and display a 6-digit verification code.
5. Check that the two codes match, then tap Confirm or Approve within 120 seconds on both devices.
6. Keynear saves non-secret metadata to `devices.toml`. The `irk`, `reconnect_secret`, static X25519 keys, and `quic_ed25519` key stay in `oo7` (`org.freedesktop.secrets`) on Linux and in Android Keystore on mobile.

## Visibility Modes

Keynear supports three visibility modes.

- **Hidden** disables BLE presence advertisements and incoming discovery.
- **Paired devices only** advertises rotating 6-byte HMAC-SHA256 presence tokens and AES-256-GCM encrypted availability state that only paired peers recognize. While the Android lockscreen is active, `KeynearTileService` toggles directly between Hidden and Paired devices only.
- **Everyone nearby** enables 10-minute (`600`-second) open discovery for pairing new devices and reverts to paired devices only when the window closes. Switching to Everyone nearby from a locked Android screen prompts you to unlock the device through `unlockAndRun`.

## Sharing Files over Wi-Fi QUIC

Use these steps to share files over direct Wi-Fi QUIC.

1. Connect both paired devices to a shared local Wi-Fi or LAN network. Keynear then uses `mdns-sd` on Linux or `NsdManager` on Android (`_keynear._udp.`) or encrypted BLE GATT (`0xFE70`) to exchange ephemeral `LanEndpoint` addresses.
2. On Android, share files or text with a paired peer through system Direct Share (`DirectShareManager`) or the `ShareActivity` bottom sheet (`ACTION_SEND` or `ACTION_SEND_MULTIPLE`), then tap Send.
3. On Linux, send files or folders through any of these entrypoints.
   - The `ksni` tray.
   - The `ashpd` file and folder choosers in `keynear-settings` (`--send-picker`).
   - Nautilus, Dolphin, and Yazi context menus (`keynear-settings --share -- <path>...`).
   - `keynear-linux --send <device_id> <path>...`.

   Keynear streams files in 60 KiB chunks and directories as uncompressed `STORED` zip archives over direct `iroh` QUIC (`X25519MLKEM768`, `RelayMode::Disabled`).
4. Incoming drops wait in `PendingConsent` for `Accept` or `Decline`. Respond through desktop (`zbus`) or Android notifications, `keynear-settings`, `MainActivity`, or the CLI (`keynear-linux --transfers`, `--accept <transfer_id>`, `--decline <transfer_id>`, `--cancel <transfer_id>`). The prompt is skipped when `auto_accept_from_paired` or per-device `Auto-accept files` is enabled.

## Clipboard and OTP Continuity

Keynear handles clipboard and one-time-code continuity in the background.

- Desktop clipboard changes sync in real time over Wayland (`ext-data-control-v1` / `zwlr-data-control-v1`) or X11 to the paired Android phone. Keynear skips password-manager MIME targets (`x-kde-passwordManagerHint`, `application/x-password`) and `ignoredMimeTypes`.
- Use **Copy phone clipboard** on the Android continuity notification or Quick Settings tile (`ClipboardTileService`) to read the phone clipboard and send it to the desktop.
- Confirm Android SMS User Consent prompts to forward verified one-time codes to the desktop clipboard. The **Copy latest OTP** action in the Linux tray menu reads the most recent code.

## Audio Handoff and Fallbacks

The current release supports the following handoff and fallback behavior.

- `1.1.0-perseus` is the current production release. Two features stand out.
  - Interrupted multi-gigabyte drops resume from the last verified 60 KiB chunk, and paired shared directories are browsable on demand through a Linux FUSE mount and an Android `DocumentsProvider`.
  - Smart audio handoff pauses MPRIS2 playback and releases the matching `bap_sink`, `a2dp_sink`, or `hfp_hf` profile before the phone claims a shared headset, then reclaims it on release.
- When no shared Wi-Fi network exists, peers negotiate Wi-Fi Aware NAN ahead of Wi-Fi Direct R2. `RangingManager` closeness gating activates only when both peers advertise the capability, the hardware supports it, and the device runs API 36 or later.
- Under GNOME Mutter, install the optional bridge in `packaging/gnome/keynear-clipboard@hydrafog.github.com/` for background clipboard sync. Every backend degrades to a typed unavailable state when its OS service is missing.
