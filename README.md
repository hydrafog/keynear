<div align="center">
  <h1>Keynear</h1>
  <p><strong>Background-first, end-to-end encrypted continuity between Linux and Android</strong></p>

  <p>
    <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Rust-Workspace-orange.svg" alt="Rust"></a>
    <a href="https://developer.android.com/"><img src="https://img.shields.io/badge/Android-API%2036-green.svg" alt="Android"></a>
    <a href="https://nixos.org/"><img src="https://img.shields.io/badge/Nix-Flake-5277C3.svg" alt="Nix"></a>
    <a href="LICENSE"><img src="https://img.shields.io/badge/License-GPLv3-blue.svg" alt="License: GPLv3"></a>
  </p>
</div>

Keynear links a Linux desktop and an Android phone over Bluetooth Low Energy (BLE) and local Wi-Fi QUIC with zero cloud accounts and zero relay servers. On Linux it runs as a `systemd --user` daemon with a native `StatusNotifierItem` tray menu and an on-demand GTK4/`relm4` preferences window; on Android it runs as a `connectedDevice` foreground service surfaced through a Quick Settings tile, the system Share Sheet, and native notifications.

## Documentation

<details>
<summary><strong>Getting Started</strong></summary>

| Document | Description |
|----------|-------------|
| [Overview](docs/getting-started/index.md) | Documentation home and navigation |
| [Installation](docs/getting-started/installation.md) | Linux (`systemd --user` / Nix) and Android setup |
| [Quickstart](docs/getting-started/quickstart.md) | First pairing, visibility modes, and file drop |

</details>

<details>
<summary><strong>Architecture</strong></summary>

| Document | Description |
|----------|-------------|
| [Overview](docs/architecture/index.md) | System boundaries and design |
| [System Overview](docs/architecture/overview.md) | Monorepo crates, `proto3`, and Android modules |
| [Protocol & Crypto](docs/architecture/protocol-and-crypto.md) | BLE tokens, Noise XX, QUIC, and `X25519MLKEM768` |

</details>

<details>
<summary><strong>Features</strong></summary>

| Document | Description |
|----------|-------------|
| [Overview](docs/features/index.md) | Core continuity capabilities |
| [File Sharing](docs/features/file-sharing.md) | Streaming transfers, Share Sheet, and portals |
| [Clipboard & OTP](docs/features/clipboard-and-otp.md) | Wayland clipboard sync and SMS User Consent |
| [Audio Handoff](docs/features/audio-handoff.md) | PipeWire and MPRIS2 headset handoff |

</details>

<details>
<summary><strong>Development</strong></summary>

| Document | Description |
|----------|-------------|
| [Overview](docs/development/index.md) | Contribution and engineering conventions |
| [Contributing](docs/development/contributing.md) | DevShell, `scripts/`, and commit conventions |
| [Release Timeline](docs/development/timeline.md) | Dependency-ordered constellation milestones |

</details>

<details>
<summary><strong>Deployment &amp; Operations</strong></summary>

| Document | Description |
|----------|-------------|
| [Deployment Overview](docs/deployment/index.md) | Linux and Android deployment guides |
| [Linux systemd](docs/deployment/linux-systemd.md) | User unit setup and TOML configuration |
| [Android Build](docs/deployment/android-build.md) | `cargo-ndk` and Gradle APK/AAB builds |
| [Operations Overview](docs/operations/index.md) | Runtime diagnostics and recovery |
| [Troubleshooting](docs/operations/troubleshooting.md) | BLE, local network, Wayland, and keyring fixes |

</details>

<details>
<summary><strong>Security &amp; Compliance</strong></summary>

| Document | Description |
|----------|-------------|
| [Overview](docs/security-compliance/index.md) | Security and compliance boundaries |
| [Threat Model](docs/security-compliance/threat-model.md) | Trust boundaries and post-quantum transport |
| [Permissions &amp; Privacy](docs/security-compliance/permissions-and-privacy.md) | OS permissions, MIME exclusion, and GPLv3 |

</details>

## License

All project source code, protocols, and scripts are licensed under the [GNU General Public License v3.0](LICENSE).
