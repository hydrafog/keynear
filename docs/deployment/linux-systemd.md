# Linux systemd Service

`keynear-linux` runs as an unprivileged `systemd --user` service. It stores configuration and paired device metadata in `~/.config/keynear/` or `$KEYNEAR_CONFIG_DIR/`.

## Build and Runtime Prerequisites

Building `keynear-linux` (`oo7`), `keynear-settings`, and the Rust workspace requires D-Bus development headers. On Debian and Ubuntu, install `libdbus-1-dev`. The repository Nix devShell provides the header automatically.

At runtime, Keynear needs GTK4 and these services:

- `bluetoothd` for BlueZ.
- An `org.freedesktop.secrets` Secret Service provider such as GNOME Keyring or KWallet.

When BlueZ or Secret Service is unavailable at startup, `keynear-linux` logs a warning and keeps local daemon control active without BLE or keyring operations.

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

On NixOS, `nixosModules.default` (`packaging/nixos/nixos-module.nix`) provisions the hardened user unit, the Bluetooth default, and the desktop entry and icons. It also adds these options.

- FUSE3 support (`services.keynear.fuse.enable`).
- mDNS/UDP firewall rules (`services.keynear.openFirewall`).
- A GNOME extension.

On Home Manager, `homeManagerModules.default` (`packaging/nixos/home-manager-module.nix`, re-exported by `packaging/nixos/default.nix`) adds declarative non-secret `services.keynear.settings` (`config.toml`) and secret-key assertions. It also exposes `services.keynear.fuse`, `services.keynear.logLevel`, and `services.keynear.integrations.{nautilus,dolphin,yazi,gnomeExtension}.enable`.

## CLI Flags, Tray, Config, and Paired Device Store

Keynear exposes the following command-line, tray, and storage behavior.

- `keynear-linux` accepts these flags.
  - `--config <path>`, `--status`, `--devices`.
  - `--set-visibility <hidden|paired|everyone>`, `--rename-device <device_id> <display_name>`.
  - `--send <device_id> <path>...`, `--accept <transfer_id>`, `--decline <transfer_id>`, `--cancel <transfer_id>`.
  - `--transfers`, `--version`, `--help`.

  A `--config <path>` file must exist and pass `DaemonConfig::validate()`.
- `keynear-settings` uses GTK4 application ID `org.keynear.Keynear`. The ID matches `org.keynear.Keynear.desktop` and the `hicolor` Geometric K-Wave icon. The binary accepts:
  - `--validate`, `--config <path>`, `--version`, `--help`.
  - `--send-picker [--device <id>]`.
  - `--share [--device <id>] [--] <path>...`.

  File-manager integrations at `packaging/linux/integrations/nautilus/send-via-keynear`, `packaging/linux/integrations/dolphin/keynear-send.desktop`, and `packaging/linux/integrations/yazi/keynear.yazi/main.lua` launch `keynear-settings --share --` with selected paths.
- Inside a graphical D-Bus session, `keynear-linux` registers a `ksni` `StatusNotifierItem` tray menu with an `Everyone (MM:SS)` live countdown and a `keynear-settings` launcher. It also publishes interactive `org.freedesktop.Notifications` alerts through `zbus` for `Accept`, `Decline`, `Cancel`, and pairing confirmation. Headless environments without a tray watcher or notification daemon keep serving IPC and P2P transfers.
- `config.toml` loads from `~/.config/keynear/config.toml`. It holds these settings.
  - `visibilityMode`, `incomingDirectory`, `autoAcceptFromPaired`.
  - `clipboardSyncEnabled`, `clipboardImagesEnabled`, `otpPassthroughEnabled`.
  - `audioHandoffEnabled`, `ignoredMimeTypes`, `notifyAutoAcceptComplete`.

  Keynear falls back to safe defaults when the file is absent and saves updates atomically through `DaemonConfig::save_to_path`.
- The on-demand virtual filesystem is a typed backend in `crates/keynear-linux/src/fuse/`. The daemon logs a graded availability probe at startup. When kernel FUSE or `libfuse3` is absent, it emits `perseus_fuse_unavailable` and keeps serving IPC and transfers.
- Notification mirroring is opt-in on the phone. Excluded packages are configuration and contain no message content. The desktop notification client logs `mirror_notifications_unavailable` when the session bus is absent and keeps the daemon running. MPRIS2 media observation logs `carina_media_unavailable` when no player or bus exists.
  - On NixOS, `services.keynear.fuse.enable = true` sets `KEYNEAR_FUSE=1` and enables the optional `fuse` Cargo feature on the default Nix package.
  - On Home Manager, use `services.keynear.fuse = true`.

  The default headless Cargo build does not require `libfuse3`.
- `PairedDeviceStore` persists non-secret paired device metadata in `~/.config/keynear/devices.toml` with `0600` permissions. The metadata includes:
  - `device_id`, `display_name`, `role`, `public_key_fingerprint`.
  - `last_ble_address` and `paired_at`.

  Display-name updates are validated against `MAX_DISPLAY_NAME_LEN = 64` and written atomically through `PairedDeviceStore::rename`. Pairwise `irk` and `reconnect_secret` keys and the monotonic presence counter stay exclusively in `oo7` (`org.freedesktop.secrets`).
- Setting `visibilityMode` to `EveryoneNearby` starts a 10-minute (`600`-second) countdown that reverts to `PairedOnly` when it elapses. `keynear-linux` handles `SIGINT` and `SIGTERM` through a unified shutdown loop.
