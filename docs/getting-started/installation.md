# Installation

`Keynear` runs natively on Linux desktops and Android 12+ (`minSdk = 31`) devices.

## Linux

Build the workspace inside the Nix development shell or with a local Rust toolchain (`protobuf-compiler`, `libgtk-4-dev`, and `pkg-config` installed):

```bash
git clone git@github.com:hydrafog/keynear.git
cd keynear
nix develop -c cargo build --release -p keynear-linux -p keynear-settings
```

Install the `keynear-linux` daemon, `keynear-settings` binaries onto `~/.local/bin`, validate settings tokens headlessly with `keynear-settings --validate`, install `packaging/linux/systemd/keynear.service`, and optionally link the file-manager integrations under `packaging/linux/integrations/`:

```bash
install -Dm755 target/release/keynear-linux ~/.local/bin/keynear-linux
install -Dm755 target/release/keynear-settings ~/.local/bin/keynear-settings
~/.local/bin/keynear-settings --validate
mkdir -p ~/.config/systemd/user
cp packaging/linux/systemd/keynear.service ~/.config/systemd/user/keynear.service

install -Dm755 packaging/linux/integrations/nautilus/send-via-keynear ~/.local/share/nautilus/scripts/Send\ via\ Keynear
install -Dm644 packaging/linux/integrations/dolphin/keynear-send.desktop ~/.local/share/kio/servicemenus/keynear-send.desktop
install -Dm644 packaging/linux/integrations/yazi/keynear.yazi/main.lua ~/.config/yazi/plugins/keynear.yazi/main.lua
```

See [Linux systemd Deployment](../deployment/linux-systemd.md) for enabling and starting the user unit.

## Android

Build the native libraries and companion APK (`1.0.0-vega`, `versionCode = 6`) inside `nix develop .#android` using `cargo-ndk` and `gradle assembleDebug` as detailed in [Android Build](../deployment/android-build.md), then install it on the target device and follow the collapsible onboarding checklist in `MainActivity` for `NEARBY_DEVICES`, `NEARBY_WIFI_DEVICES`, `POST_NOTIFICATIONS`, and background battery-optimization exemption.
