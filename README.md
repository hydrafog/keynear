<div align="center">
  <img src="assets/brand/icon.svg" width="100" height="100" alt="Keynear logo">
  <h1>Keynear</h1>
  <p><strong>Background-first, end-to-end encrypted continuity between Linux and Android</strong></p>

  <p>
    <a href="docs/development/timeline.md"><img src="https://img.shields.io/badge/Version-1.1.0--perseus-0656E8?style=flat-square" alt="Version 1.1.0-perseus"></a>
    <a href="https://github.com/hydrafog/keynear/actions/workflows/quality.yml"><img src="https://img.shields.io/github/actions/workflow/status/hydrafog/keynear/quality.yml?branch=main&style=flat-square&color=0656E8&logo=githubactions&logoColor=white&label=Quality%20Gate" alt="Quality Gate workflow status"></a>
    <a href="docs/deployment/linux-systemd.md"><img src="https://img.shields.io/badge/Linux-Wayland%20%26%20X11%20%C2%B7%20GTK4-0656E8?style=flat-square&logo=linux&logoColor=white" alt="Linux: Wayland, X11, and GTK4"></a>
    <a href="docs/deployment/android-build.md"><img src="https://img.shields.io/badge/Android-12--16%20(API%2031--36)-0656E8?style=flat-square&logo=android&logoColor=white" alt="Android 12 to 16 (API 31 to 36)"></a>
    <a href="docs/architecture/protocol-and-crypto.md"><img src="https://img.shields.io/badge/Crypto-Noise%20XX%20%2B%20ML--KEM--768-0656E8?style=flat-square" alt="Cryptography: Noise XX and ML-KEM-768"></a>
    <a href="LICENSE"><img src="https://img.shields.io/github/license/hydrafog/keynear?style=flat-square&color=0656E8&label=License" alt="License: GPL-3.0"></a>
  </p>
</div>

Keynear links a Linux desktop and an Android phone over Bluetooth Low Energy (BLE) and local Wi-Fi QUIC, with no cloud account or relay server. On Linux it is a `systemd --user` daemon with a native `StatusNotifierItem` tray menu and an on-demand GTK4/`relm4` preferences window. On Android it is a `connectedDevice` foreground service with a Quick Settings tile, the system Share Sheet, and native notifications.

<h3 id="documentation">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/brand/heading-documentation-dark.svg">
    <source media="(prefers-color-scheme: light)" srcset="assets/brand/heading-documentation-light.svg">
    <img src="assets/brand/heading-documentation-light.svg" alt="Documentation" width="100%">
  </picture>
</h3>

<details>
<summary><strong>Getting Started</strong></summary>

These guides cover installation and first pairing.

| Document | Description |
|----------|-------------|
| [Installation](docs/getting-started/installation.md) | Linux (`systemd --user` / Nix) and Android setup |
| [Quickstart](docs/getting-started/quickstart.md) | First pairing, visibility modes, and file drop |

</details>

<details>
<summary><strong>Architecture</strong></summary>

These guides cover the system design and the cryptographic protocol.

| Document | Description |
|----------|-------------|
| [System Overview](docs/architecture/overview.md) | Monorepo crates, `proto3`, and Android modules |
| [Protocol & Crypto](docs/architecture/protocol-and-crypto.md) | BLE tokens, Noise XX, QUIC, and `X25519MLKEM768` |

</details>

<details>
<summary><strong>Features</strong></summary>

These guides cover the continuity features Keynear provides.

| Document | Description |
|----------|-------------|
| [File Sharing](docs/features/file-sharing.md) | Streaming transfers, Share Sheet, and portals |
| [Clipboard & OTP](docs/features/clipboard-and-otp.md) | Wayland clipboard sync and SMS User Consent |
| [Audio Handoff](docs/features/audio-handoff.md) | PipeWire and MPRIS2 headset handoff |

</details>

<details>
<summary><strong>Development</strong></summary>

These guides cover contributing and the release plan.

| Document | Description |
|----------|-------------|
| [Contributing](docs/development/contributing.md) | DevShell, `scripts/`, and commit conventions |
| [Release Timeline](docs/development/timeline.md) | Dependency-ordered constellation milestones |

</details>

<details>
<summary><strong>Deployment &amp; Operations</strong></summary>

These guides cover deployment and runtime recovery.

| Document | Description |
|----------|-------------|
| [Linux systemd](docs/deployment/linux-systemd.md) | User unit setup and TOML configuration |
| [Android Build](docs/deployment/android-build.md) | `cargo-ndk` and Gradle APK/AAB builds |
| [Troubleshooting](docs/operations/troubleshooting.md) | BLE, local network, Wayland, and keyring fixes |

</details>

<details>
<summary><strong>Security &amp; Compliance</strong></summary>

These guides cover the trust model, permissions, and licensing.

| Document | Description |
|----------|-------------|
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
