# Linux systemd Service

`keynear` runs as an unprivileged `systemd --user` service (`keynear daemon`). It stores configuration and paired device metadata in `~/.config/keynear/` or the directory named by `KEYNEAR_CONFIG_DIR`. A non-empty `KEYNEAR_DATA_DIR` points `config.toml` at a separate data directory, and an invalid or empty value falls back to the config directory.

## Build and Runtime Prerequisites

Building `keynear` (`oo7`) and the Rust workspace requires D-Bus development headers. On Debian and Ubuntu, install `libdbus-1-dev` and `libpipewire-0.3-dev`. The `pipewire` crate also needs `libclang-dev` for its bindgen build. The repository Nix devShell provides these automatically.

At runtime, Keynear uses these services:

- `bluetoothd` for BlueZ.
- An `org.freedesktop.secrets` Secret Service provider such as GNOME Keyring or KWallet.
- An `xdg-desktop-portal` backend when opening file dialogs from `keynear send --picker` or the tray menu.

When BlueZ or Secret Service is unavailable at startup, `keynear` logs a warning and keeps local daemon control active without BLE or keyring operations.

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
ExecStart=%h/.local/bin/keynear daemon
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

## Environment Overrides

The daemon and CLI honor these environment variables.

- `KEYNEAR_LOG` sets the tracing filter when it holds a non-empty value. An unset or invalid value falls back to `RUST_LOG`, then to `info`.
- `KEYNEAR_CONFIG_DIR` sets the base directory for `config.toml` and `devices.toml`. An unset or empty value falls back to `XDG_CONFIG_HOME/keynear` or `~/.config/keynear`.
- `KEYNEAR_DATA_DIR` points `config.toml` at a separate data directory when it holds a non-empty, NUL-free path. An invalid or empty value falls back to the config directory.
- `KEYNEAR_FUSE` selects the FUSE mount for `1`, `true`, `TRUE`, or `yes` when the daemon is built with the `fuse` feature. Other values keep the mount unavailable.

Unavailable backends degrade to warnings rather than stopping the daemon. BlueZ, Secret Service, the FUSE mount, and the clipboard bridge each log a typed unavailable event and keep local control and transfers active. Audio handoff calls BlueZ `org.bluez.MediaTransport1.Release` and `Acquire` through `zbus`, and logs `vega_sink_release_unavailable` or `vega_sink_claim_unavailable` when the bus or transport is absent.

On NixOS, `nixosModules.default` (`packaging/nixos/nixos-module.nix`) provisions the hardened user unit, the Bluetooth default, and the desktop entry and icons. It also adds these options.

- FUSE3 support (`services.keynear.fuse.enable`).
- mDNS/UDP firewall rules (`services.keynear.openFirewall`).
- A GNOME extension.

On Home Manager, `homeManagerModules.default` (`packaging/nixos/home-manager-module.nix`, re-exported by `packaging/nixos/default.nix`) adds declarative non-secret `services.keynear.settings` (`config.toml`) and secret-key assertions. It also exposes `services.keynear.fuse`, `services.keynear.logLevel`, and `services.keynear.integrations.{nautilus,dolphin,yazi,gnomeExtension}.enable`.

## CLI Subcommands, Tray, Config, and Paired Device Store

Keynear uses the bare primary product noun `keynear` for the executable crate and binary with subcommands for user surfaces (`keynear settings`, `keynear daemon`, `keynear send`), while internal workspace libraries use `<product>-<layer>` (`keynear-protocol`, `keynear-core`, `keynear-ipc`, `keynear-ffi`).

Keynear exposes the following command-line, tray, and storage behavior.

- `keynear` uses a subcommand-first CLI (also accepting double-dash flags for compatibility):
  - `keynear [daemon] [--config <path>]` starts the daemon and tray (default when no subcommand is given).
  - `keynear settings [show|validate] [--config <path>]` prints or validates `DaemonConfig`.
  - `keynear status`, `keynear devices`, `keynear transfers`.
  - `keynear visibility <hidden|paired|everyone>`, `keynear rename <device_id> <display_name>`.
  - `keynear send [--device <device_id>] [--] <path>...`, `keynear send <device_id> [--] <path>...`, `keynear send --picker [--device <device_id>]`.
  - `keynear accept <transfer_id>`, `keynear decline <transfer_id>`, `keynear cancel <transfer_id>`.
  - `keynear version`, `keynear help`.

  A `--config <path>` file must exist and pass `DaemonConfig::validate()`.
- `org.keynear.Keynear` matches `org.keynear.Keynear.desktop` and the `hicolor` Geometric K-Wave symbolic icon (`org.keynear.Keynear-symbolic`). File-manager integrations at `packaging/linux/integrations/nautilus/send-via-keynear`, `packaging/linux/integrations/dolphin/keynear-send.desktop`, and `packaging/linux/integrations/yazi/keynear.yazi/main.lua` launch `keynear send --` with selected paths.
- Inside a graphical D-Bus session, `keynear daemon` registers a `ksni` `StatusNotifierItem` tray menu displaying `org.keynear.Keynear-symbolic` with `Keynear` as title and tooltip, opening the popup menu on click (`MENU_ON_ACTIVATE`). The menu shows `Keynear — <visibility>`, visibility mode radios with an `Everyone (MM:SS)` live countdown, active transfers, paired devices, `Now playing`, and `Send files...` (`keynear send --picker`). It also publishes interactive `org.freedesktop.Notifications` alerts through `zbus` for `Accept`, `Decline`, `Cancel`, and pairing confirmation. Headless environments without a tray watcher or notification daemon keep serving IPC and P2P transfers.
- `config.toml` loads from `~/.config/keynear/config.toml`. It holds these settings.
  - `visibilityMode`, `incomingDirectory`, `autoAcceptFromPaired`.
  - `clipboardSyncEnabled`, `clipboardImagesEnabled`, `otpPassthroughEnabled`.
  - `audioHandoffEnabled`, `ignoredMimeTypes`, `notifyAutoAcceptComplete`.

  Keynear falls back to safe defaults when the file is absent and saves updates atomically through `DaemonConfig::save_to_path`.
- The on-demand virtual filesystem is a typed backend in `crates/keynear/src/fuse/`. `crates/keynear/src/fuse/probe.rs` selects the `FuserAdapter` mount only when `KEYNEAR_FUSE` enables it and the `fuse` feature is compiled, and otherwise selects the unavailable backend. The guard logs a graded availability probe (`fuse_mount_probe`, `fuse_mount_state`), calls `mount` and `unmount` through its lifecycle, and logs `fuse_mount_shutdown` on drop. An unusable mountpoint logs `fuse_mountpoint_unavailable`, and a missing kernel FUSE or `libfuse3` keeps IPC and transfers active.
- Notification mirroring is opt-in on the phone. Excluded packages are configuration and contain no message content. The desktop notification client logs `notifications_unavailable` when the session bus is absent and keeps the daemon running. MPRIS2 media observation logs `media_bridge_unavailable` when no player or bus exists.
  - On NixOS, `services.keynear.fuse.enable = true` sets `KEYNEAR_FUSE=1` and enables the optional `fuse` Cargo feature on the default Nix package.
  - On Home Manager, use `services.keynear.fuse = true`.

  The default headless Cargo build does not require `libfuse3`.
- `PairedDeviceStore` persists non-secret paired device metadata in `~/.config/keynear/devices.toml` with `0600` permissions. The metadata includes:
  - `device_id`, `display_name`, `role`, `public_key_fingerprint`.
  - `last_ble_address` and `paired_at`.

  Display-name updates are validated against `MAX_DISPLAY_NAME_LEN = 64` and written atomically through `PairedDeviceStore::rename`. Pairwise `irk` and `reconnect_secret` keys and the monotonic presence counter stay exclusively in `oo7` (`org.freedesktop.secrets`).
- Setting `visibilityMode` to `EveryoneNearby` starts a 10-minute (`600`-second) countdown that reverts to `PairedOnly` when it elapses. `keynear daemon` handles `SIGINT` and `SIGTERM` through a unified shutdown loop.
