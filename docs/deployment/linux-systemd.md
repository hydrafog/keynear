# Linux systemd Service

`keynear-linux` runs as an unprivileged `systemd --user` service and stores configuration and paired device metadata in `~/.config/keynear/` (or `$KEYNEAR_CONFIG_DIR/`).

## Build and Runtime Prerequisites

Building `keynear-core` (`linux-oo7`), `keynear-linux`, and `keynear-settings` requires D-Bus development headers (`libdbus-1-dev` on Debian and Ubuntu, provided automatically by the repository Nix devShell) alongside GTK4 and BlueZ runtime services (`bluetoothd` and an `org.freedesktop.secrets` Secret Service provider such as GNOME Keyring or KWallet). When BlueZ or Secret Service is unavailable at startup, `keynear-linux` logs a warning and keeps local daemon control active without BLE or keyring operations.

## Unit Configuration

Install the repository unit file `systemd/keynear.service` to `~/.config/systemd/user/keynear.service`:

```ini
[Unit]
Description=Keynear background continuity daemon
Wants=graphical-session.target
After=graphical-session.target bluetooth.target
StartLimitIntervalSec=0

[Service]
ExecStart=%h/.local/bin/keynear-linux
Restart=on-failure
RestartSec=3

[Install]
WantedBy=default.target
```

Enable and start the service:

```bash
mkdir -p ~/.config/systemd/user
cp systemd/keynear.service ~/.config/systemd/user/keynear.service
systemctl --user daemon-reload
systemctl --user enable --now keynear.service
```

## CLI Flags, Tray, Config, and Paired Device Store

- `keynear-linux` accepts `--config <path>`, `--status`, `--devices`, `--set-visibility <hidden|paired|everyone>`, `--rename-device <device_id> <display_name>`, `--send <device_id> <path>...`, `--accept <transfer_id>`, `--decline <transfer_id>`, `--cancel <transfer_id>`, `--transfers`, `--version`, and `--help`. A `--config <path>` file must exist and pass `DaemonConfig::validate()`.
- `keynear-settings` accepts `--validate`, `--send-picker [--device <id>]`, `--share [--device <id>] [--] <path>...`, `--config <path>`, `--version`, and `--help`. File-manager integrations under `integrations/nautilus/send-via-keynear`, `integrations/dolphin/keynear-send.desktop`, and `integrations/yazi/keynear.yazi/main.lua` launch `keynear-settings --share --` with selected paths.
- When running inside a graphical D-Bus session, `keynear-linux` registers a `ksni` `StatusNotifierItem` tray menu (`Everyone (MM:SS)` live countdown and `keynear-settings` launcher) and publishes interactive `org.freedesktop.Notifications` alerts (`Accept`/`Decline`/`Cancel`, pairing confirmation) via `zbus`; headless environments without a tray watcher or notification daemon continue serving IPC and P2P transfers.
- `config.toml` loads from `~/.config/keynear/config.toml` with `visibilityMode`, `incomingDirectory`, `autoAcceptFromPaired`, `clipboardSyncEnabled`, `clipboardImagesEnabled`, `otpPassthroughEnabled`, `audioHandoffEnabled`, `ignoredMimeTypes`, and `notifyAutoAcceptComplete`, falling back to safe defaults when the file is absent and serializing updates atomically via `DaemonConfig::save_to_path`.
- `PairedDeviceStore` persists non-secret paired device metadata (`device_id`, `display_name`, `role`, `public_key_fingerprint`, `last_ble_address`, `paired_at`) in `~/.config/keynear/devices.toml` with `0600` permissions and validates display-name updates (`MAX_DISPLAY_NAME_LEN = 64`) atomically through `PairedDeviceStore::rename`. Pairwise `irk` and `reconnect_secret` keys and the monotonic presence counter stay exclusively in `oo7` (`org.freedesktop.secrets`).
- Setting `visibilityMode` to `EveryoneNearby` starts a 10-minute (`600`-second) countdown that reverts to `PairedOnly` when it elapses. `keynear-linux` handles `SIGINT` and `SIGTERM` through a unified shutdown loop.
