# Installation

`Keynear` runs natively on Linux desktops and Android 12+ (`minSdk = 31`) devices.

## NixOS and Home Manager

Add Keynear as a flake input and enable either the NixOS system module (`nixosModules.default`) or the Home Manager module (`homeManagerModules.default`). The flake also exports `packages.${system}.{default,keynear,keynear-fuse,keynear-gnome-extension}` and `overlays.default`:

```nix
{
  inputs.keynear.url = "github:hydrafog/keynear";

  # NixOS system configuration
  outputs = { nixpkgs, keynear, ... }: {
    nixosConfigurations.desktop = nixpkgs.lib.nixosSystem {
      modules = [
        keynear.nixosModules.default
        {
          services.keynear = {
            enable = true;
            openFirewall = true;
            fuse.enable = true;
            integrations.gnomeExtension.enable = false;
          };
        }
      ];
    };
  };
}
```

For user-scoped declarative `config.toml` settings and file-manager integrations, import `keynear.homeManagerModules.default` (also exposed at `packaging/nixos/default.nix`) and configure non-secret `services.keynear.settings` plus `services.keynear.integrations.{nautilus,dolphin,yazi,gnomeExtension}.enable`.

Pre-built binaries are published to `https://hydrafog.cachix.org`. `flake.nix` declares `nixConfig` for interactive flake commands, and NixOS configurations can opt in permanently with:

```nix
nix.settings = {
  extra-substituters = [ "https://hydrafog.cachix.org" ];
  extra-trusted-public-keys = [ "hydrafog.cachix.org-1:UKlU4vpY+vHoft1i+OAVBaVnFaq6udjRM8FPUSAPKLw=" ];
};
```

## Flathub (Flatpak)

`packaging/flathub/org.keynear.Keynear.yml` targets `org.gnome.Platform` `49` with `org.freedesktop.Sdk.Extension.rust-stable`, installs `org.keynear.Keynear.desktop`, `org.keynear.Keynear.metainfo.xml`, `hicolor` icons, and the bundled `Plus Jakarta Sans` font, and aligns the GTK4 application ID to `org.keynear.Keynear`.

## Linux From Source

Build the workspace inside the Nix development shell or with a local Rust toolchain (`protobuf-compiler`, `libgtk-4-dev`, `libdbus-1-dev`, and `pkg-config` installed):

```bash
git clone git@github.com:hydrafog/keynear.git
cd keynear
nix develop -c cargo build --release -p keynear-linux -p keynear-settings
```

Install the `keynear-linux` daemon, `keynear-settings`, desktop entry, `hicolor` icons, systemd user unit, and optional file-manager integrations under `packaging/linux/`:

```bash
install -Dm755 target/release/keynear-linux ~/.local/bin/keynear-linux
install -Dm755 target/release/keynear-settings ~/.local/bin/keynear-settings
~/.local/bin/keynear-settings --validate
install -Dm644 packaging/flathub/org.keynear.Keynear.desktop ~/.local/share/applications/org.keynear.Keynear.desktop
install -Dm644 packaging/linux/icons/hicolor/scalable/apps/org.keynear.Keynear.svg ~/.local/share/icons/hicolor/scalable/apps/org.keynear.Keynear.svg
install -Dm644 packaging/linux/icons/hicolor/symbolic/apps/org.keynear.Keynear-symbolic.svg ~/.local/share/icons/hicolor/symbolic/apps/org.keynear.Keynear-symbolic.svg
install -Dm644 packaging/linux/systemd/keynear.service ~/.config/systemd/user/keynear.service

install -Dm755 packaging/linux/integrations/nautilus/send-via-keynear ~/.local/share/nautilus/scripts/Send\ via\ Keynear
install -Dm644 packaging/linux/integrations/dolphin/keynear-send.desktop ~/.local/share/kio/servicemenus/keynear-send.desktop
install -Dm644 packaging/linux/integrations/yazi/keynear.yazi/main.lua ~/.config/yazi/plugins/keynear.yazi/main.lua
```

See [Linux systemd Deployment](../deployment/linux-systemd.md) for enabling and starting the user unit.

## Android and F-Droid

The Android app (`1.1.0-perseus`, `versionCode = 7`) ships two Gradle product flavors: `gms` (default, includes Play Services SMS User Consent OTP forwarding) and `foss` (zero Google Play Services dependencies, built by `packaging/fdroid/org.keynear.app.yml`). Build inside `nix develop .#android` using `cargo-ndk` and `gradle assembleGmsDebug` or `gradle assembleFossRelease` as detailed in [Android Build](../deployment/android-build.md).
