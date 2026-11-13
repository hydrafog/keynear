# Linux systemd Service

`keynear-linux` runs as an unprivileged `systemd --user` service and reads configuration from `~/.config/keynear/config.toml` (or `$KEYNEAR_CONFIG_DIR/config.toml`).

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

## CLI Flags and Configuration File

`keynear-linux` supports `--config <path>`, `--version`, and `--help`. When `--config <path>` is specified, the file must exist and pass `DaemonConfig::validate()`. When omitted, `DaemonConfig` loads from `~/.config/keynear/config.toml` (`visibilityMode`, `incomingDirectory`, `autoAcceptFromPaired`, `clipboardSyncEnabled`, `clipboardImagesEnabled`, `otpPassthroughEnabled`, `audioHandoffEnabled`, `ignoredMimeTypes`, `notifyAutoAcceptComplete`) and falls back to safe defaults when absent. `keynear-linux` handles `SIGINT` and `SIGTERM` cleanly through a unified shutdown loop.
