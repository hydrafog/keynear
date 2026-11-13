# Linux systemd Service

`keynear-linux` runs as an unprivileged `systemd --user` service and reads configuration from `~/.config/keynear/config.toml` (or `$KEYNEAR_CONFIG_DIR/config.toml`).

## Unit Configuration

Create `~/.config/systemd/user/keynear.service`:

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
systemctl --user daemon-reload
systemctl --user enable --now keynear.service
```

## Configuration File

`DaemonConfig` reads camelCase keys from `~/.config/keynear/config.toml` (`visibilityMode`, `incomingDirectory`, `autoAcceptFromPaired`, `clipboardSyncEnabled`, `clipboardImagesEnabled`, `otpPassthroughEnabled`, `audioHandoffEnabled`, `ignoredMimeTypes`, `notifyAutoAcceptComplete`) and falls back to safe defaults when the file is absent.

