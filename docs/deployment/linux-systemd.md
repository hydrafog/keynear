# Linux systemd Service

`keynear-linux` runs as an unprivileged `systemd --user` service and stores configuration and paired device metadata in `~/.config/keynear/` (or `$KEYNEAR_CONFIG_DIR/`).

## Build and Runtime Prerequisites

Building `keynear-core` (`linux-oo7`), `keynear-linux`, and `keynear-settings` requires D-Bus development headers (`libdbus-1-dev` on Debian/Ubuntu, provided automatically by the repository Nix devShell) alongside GTK4 and BlueZ runtime services (`bluetoothd` and an `org.freedesktop.secrets` Secret Service provider such as GNOME Keyring or KWallet). When BlueZ or Secret Service is unavailable at startup, `keynear-linux` logs a warning and keeps local daemon control active without BLE/keyring operations.

## Unit Configuration

Install the repository unit file `systemd/keynear.service` to `~/.config/systemd/user/keynear.service`:

```ini
[Unit]
Description=Keynear background continuity daemon
After=graphical-session.target bluetooth.target

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

## CLI Flags, Config, and Paired Device Store

- **Flags**: `keynear-linux` supports `--config <path>`, `--version`, and `--help`. When `--config <path>` is given, the file must exist and pass `DaemonConfig::validate()`.
- **Configuration (`config.toml`)**: Loads from `~/.config/keynear/config.toml` (`visibilityMode`, `incomingDirectory`, `autoAcceptFromPaired`, `clipboardSyncEnabled`, `clipboardImagesEnabled`, `otpPassthroughEnabled`, `audioHandoffEnabled`, `ignoredMimeTypes`, `notifyAutoAcceptComplete`) and falls back to safe defaults when absent.
- **Paired Metadata (`devices.toml`)**: `PairedDeviceStore` persists non-secret paired device metadata (`device_id`, `display_name`, `role`, `public_key_fingerprint`, `last_ble_address`, `paired_at`) in `~/.config/keynear/devices.toml` (`0600`), while pairwise `irk` and `reconnect_secret` keys and the monotonic presence counter remain exclusively in `oo7` (`org.freedesktop.secrets`).
- **Visibility Auto-Revert**: Setting `visibilityMode` to `EveryoneNearby` starts a 10-minute (`600`-second) countdown that automatically reverts to `PairedOnly` when elapsed. `keynear-linux` handles `SIGINT` and `SIGTERM` cleanly through a unified shutdown loop.
