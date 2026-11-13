# Installation

`Keynear` runs natively on Linux desktops and Android 12+ (`minSdk = 31`) devices.

## Linux

Build the workspace inside the Nix development shell or with a local Rust toolchain (`protobuf-compiler`, `libgtk-4-dev`, and `pkg-config` installed):

```bash
git clone git@github.com:hydrafog/keynear.git
cd keynear
nix develop -c cargo build --release -p keynear-linux -p keynear-settings
```

Install the `keynear-linux` daemon and `keynear-settings` binaries onto `~/.local/bin`, validate settings tokens headlessly with `keynear-settings --validate`, and install `systemd/keynear.service`:

```bash
install -Dm755 target/release/keynear-linux ~/.local/bin/keynear-linux
install -Dm755 target/release/keynear-settings ~/.local/bin/keynear-settings
~/.local/bin/keynear-settings --validate
mkdir -p ~/.config/systemd/user
cp systemd/keynear.service ~/.config/systemd/user/keynear.service
```

See [Linux systemd Deployment](../deployment/linux-systemd.md) for enabling and starting the user unit.

## Android

Build the native libraries and companion APK inside `nix develop .#android` using `cargo-ndk` and `./gradlew assembleDebug` as detailed in [Android Build](../deployment/android-build.md), then install it on the target device and grant the one-time `NEARBY_DEVICES`, `POST_NOTIFICATIONS`, and battery-exemption permissions.
