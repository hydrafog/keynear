# Linux systemd Service

`keynear-linux` runs as an unprivileged `systemd --user` service and stores configuration and paired device metadata in `~/.config/keynear/` (or `$KEYNEAR_CONFIG_DIR/`).

## Build and Runtime Prerequisites

Building `keynear-linux` (`oo7`), `keynear-settings`, and the Rust workspace requires D-Bus development headers (`libdbus-1-dev` on Debian and Ubuntu, provided automatically by the repository Nix devShell) alongside GTK4 and BlueZ runtime services (`bluetoothd` and an `org.freedesktop.secrets` Secret Service provider such as GNOME Keyring or KWallet). When BlueZ or Secret Service is unavailable at startup, `keynear-linux` logs a warning and keeps local daemon control active without BLE or keyring operations.

## Unit Configuration

Install `packaging/linux/systemd/keynear.service` to `~/.config/systemd/user/keynear.service`:

```ini
[Unit]
Description=Keynear background continuity daemon
PartOf=graphical-session.target
Wants=graphical-session.target
After=graphical-session.target bluetooth.target
StartLimitIntervalSec=0

[Service]
ExecStart=%h/.local/bin/keynear-linux
Restart=on-failure
RestartSec=3
Environment=KEYNEAR_LOG=info
NoNewPrivileges=yes
PrivateTmp=yes
LockPersonality=yes
RestrictRealtime=yes
RestrictSUIDSGID=yes

[Install]
WantedBy=graphical-session.target
```

Enable and start the service:

```bash
mkdir -p ~/.config/systemd/user
cp packaging/linux/systemd/keynear.service ~/.config/systemd/user/keynear.service
systemctl --user daemon-reload
systemctl --user enable --now keynear.service
```

On NixOS, `nixosModules.default` (`packaging/nixos/nixos-module.nix`) provisions the hardened user unit, Bluetooth default, optional FUSE3 support (`services.keynear.fuse.enable`), optional mDNS/UDP firewall rules (`services.keynear.openFirewall`), desktop entry, icons, and optional GNOME extension. On Home Manager, `homeManagerModules.default` (`packaging/nixos/home-manager-module.nix`, re-exported by `packaging/nixos/default.nix`) adds declarative non-secret `services.keynear.settings` (`config.toml`), secret-key assertions, `services.keynear.fuse`, `services.keynear.logLevel`, and `services.keynear.integrations.{nautilus,dolphin,yazi,gnomeExtension}.enable`.

## CLI Flags, Tray, Config, and Paired Device Store

- `keynear-linux` accepts `--config <path>`, `--status`, `--devices`, `--set-visibility <hidden|paired|everyone>`, `--rename-device <device_id> <display_name>`, `--send <device_id> <path>...`, `--accept <transfer_id>`, `--decline <transfer_id>`, `--cancel <transfer_id>`, `--transfers`, `--version`, and `--help`. A `--config <path>` file must exist and pass `DaemonConfig::validate()`.
- `keynear-settings` uses GTK4 application ID `org.keynear.Keynear` (matching `org.keynear.Keynear.desktop` and the `hicolor` Geometric K-Wave icon) and accepts `--validate`, `--send-picker [--device <id>]`, `--share [--device <id>] [--] <path>...`, `--config <path>`, `--version`, and `--help`. File-manager integrations under `packaging/linux/integrations/nautilus/send-via-keynear`, `packaging/linux/integrations/dolphin/keynear-send.desktop`, and `packaging/linux/integrations/yazi/keynear.yazi/main.lua` launch `keynear-settings --share --` with selected paths.
- When running inside a graphical D-Bus session, `keynear-linux` registers a `ksni` `StatusNotifierItem` tray menu (`Everyone (MM:SS)` live countdown and `keynear-settings` launcher) and publishes interactive `org.freedesktop.Notifications` alerts (`Accept`/`Decline`/`Cancel`, pairing confirmation) via `zbus`; headless environments without a tray watcher or notification daemon continue serving IPC and P2P transfers.
- `config.toml` loads from `~/.config/keynear/config.toml` with `visibilityMode`, `incomingDirectory`, `autoAcceptFromPaired`, `clipboardSyncEnabled`, `clipboardImagesEnabled`, `otpPassthroughEnabled`, `audioHandoffEnabled`, `ignoredMimeTypes`, and `notifyAutoAcceptComplete`, falling back to safe defaults when the file is absent and serializing updates atomically via `DaemonConfig::save_to_path`.
- The on-demand virtual filesystem is a typed backend in `crates/keynear-linux/src/fuse/`. The daemon logs a graded availability probe at startup and, when kernel FUSE or `libfuse3` is absent, emits `perseus_fuse_unavailable` and continues serving IPC and transfers. Enabling `services.keynear.fuse.enable = true` (NixOS) or `services.keynear.fuse = true` (Home Manager) sets `KEYNEAR_FUSE=1` and enables the optional `fuse` Cargo feature on the default Nix package; the default headless Cargo build does not require `libfuse3`.
- `PairedDeviceStore` persists non-secret paired device metadata (`device_id`, `display_name`, `role`, `public_key_fingerprint`, `last_ble_address`, `paired_at`) in `~/.config/keynear/devices.toml` with `0600` permissions and validates display-name updates (`MAX_DISPLAY_NAME_LEN = 64`) atomically through `PairedDeviceStore::rename`. Pairwise `irk` and `reconnect_secret` keys and the monotonic presence counter stay exclusively in `oo7` (`org.freedesktop.secrets`).
- Setting `visibilityMode` to `EveryoneNearby` starts a 10-minute (`600`-second) countdown that reverts to `PairedOnly` when it elapses. `keynear-linux` handles `SIGINT` and `SIGTERM` through a unified shutdown loop.
