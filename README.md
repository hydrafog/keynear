<div align="center">
  <img src="assets/brand/banner.jpg" width="100%" alt="Keynear banner">
</div>

<div align="center">
  <h1>Keynear</h1>
  <p><strong>Background-first, end-to-end encrypted continuity between Linux and Android</strong></p>

  <p>
    <a href="docs/development/timeline.md"><img src="https://img.shields.io/badge/Version-1.0.0--vega-004CED?style=flat-square" alt="Version 1.0.0-vega"></a>
    <a href="https://github.com/hydrafog/keynear/actions/workflows/quality.yml"><img src="https://img.shields.io/github/actions/workflow/status/hydrafog/keynear/quality.yml?branch=main&style=flat-square&logo=githubactions&logoColor=white&label=Quality%20Gate" alt="Quality Gate workflow status"></a>
    <a href="docs/deployment/linux-systemd.md"><img src="https://img.shields.io/badge/Linux-Wayland%20%26%20X11%20%C2%B7%20GTK4-0195D4?style=flat-square&logo=linux&logoColor=white" alt="Linux: Wayland, X11, and GTK4"></a>
    <a href="docs/deployment/android-build.md"><img src="https://img.shields.io/badge/Android-12--16%20(API%2031--36)-01B37D?style=flat-square&logo=android&logoColor=white" alt="Android 12 to 16 (API 31 to 36)"></a>
    <a href="docs/architecture/protocol-and-crypto.md"><img src="https://img.shields.io/badge/Crypto-Noise%20XX%20%2B%20ML--KEM--768-07132B?style=flat-square" alt="Cryptography: Noise XX and ML-KEM-768"></a>
    <a href="LICENSE"><img src="https://img.shields.io/github/license/hydrafog/keynear?style=flat-square&color=5277C3&label=License" alt="License: GPL-3.0"></a>
  </p>
</div>

Keynear links a Linux desktop and an Android phone over Bluetooth Low Energy (BLE) and local Wi-Fi QUIC with zero cloud accounts and zero relay servers. On Linux it runs as a `systemd --user` daemon with a native `StatusNotifierItem` tray menu and an on-demand GTK4/`relm4` preferences window; on Android it runs as a `connectedDevice` foreground service surfaced through a Quick Settings tile, the system Share Sheet, and native notifications.

<h3 id="documentation">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/brand/heading-documentation-dark.svg">
    <source media="(prefers-color-scheme: light)" srcset="assets/brand/heading-documentation-light.svg">
    <img src="assets/brand/heading-documentation-light.svg" alt="Documentation" width="100%">
  </picture>
</h3>

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

<h3 id="license">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/brand/heading-license-dark.svg">
    <source media="(prefers-color-scheme: light)" srcset="assets/brand/heading-license-light.svg">
    <img src="assets/brand/heading-license-light.svg" alt="License" width="100%">
  </picture>
</h3>

All project source code, protocols, and scripts are licensed under the [GNU General Public License v3.0](LICENSE).
