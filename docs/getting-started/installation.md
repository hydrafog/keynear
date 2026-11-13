# Installation

`Keynear` runs natively on Linux desktops and Android 12+ (`minSdk = 31`) devices.

## Linux

Build the workspace inside the Nix development shell or with a local Rust toolchain:

```bash
git clone git@github.com:hydrafog/keynear.git
cd keynear
nix develop -c cargo build --release -p keynear-linux -p keynear-settings
```

Install the `keynear-linux` daemon and `keynear-settings` binaries onto your `PATH`, then enable the `systemd --user` service described in [Linux systemd Deployment](../deployment/linux-systemd.md).

## Android

Build the companion APK using `cargo-ndk` and Gradle as detailed in [Android Build](../deployment/android-build.md), then install it on the target device and grant the one-time `NEARBY_DEVICES`, `POST_NOTIFICATIONS`, and battery-exemption permissions.

