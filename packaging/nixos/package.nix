{
  lib,
  rustPlatform,
  pkg-config,
  protobuf,
  wrapGAppsHook4,
  gtk4,
  libadwaita,
  dbus,
  pipewire,
  bluez,
  wayland,
  fuse3,
  enableFuse ? false,
}:

rustPlatform.buildRustPackage {
  pname = "keynear";
  version = "1.1.0";

  src = lib.fileset.toSource {
    root = ../..;
    fileset = lib.fileset.unions [
      ../../Cargo.toml
      ../../Cargo.lock
      ../../crates
      ../../proto
      ../../packaging
      ../../assets
      ../../design/tokens.json
    ];
  };

  cargoLock = {
    lockFile = ../../Cargo.lock;
  };

  nativeBuildInputs = [
    pkg-config
    protobuf
    wrapGAppsHook4
  ];

  buildInputs = [
    gtk4
    libadwaita
    dbus
    pipewire
    bluez
    wayland
  ] ++ lib.optionals enableFuse [ fuse3 ];

  cargoBuildFlags = [
    "-p"
    "keynear-linux"
    "-p"
    "keynear-settings"
  ] ++ lib.optionals enableFuse [
    "--features"
    "keynear-linux/fuse"
  ];

  cargoTestFlags = [
    "-p"
    "keynear-linux"
    "-p"
    "keynear-settings"
  ];

  postInstall = ''
    install -Dm644 packaging/flathub/org.keynear.Keynear.desktop \
      "$out/share/applications/org.keynear.Keynear.desktop"
    install -Dm644 packaging/flathub/org.keynear.Keynear.metainfo.xml \
      "$out/share/metainfo/org.keynear.Keynear.metainfo.xml"

    install -Dm644 packaging/linux/icons/hicolor/scalable/apps/org.keynear.Keynear.svg \
      "$out/share/icons/hicolor/scalable/apps/org.keynear.Keynear.svg"
    install -Dm644 packaging/linux/icons/hicolor/symbolic/apps/org.keynear.Keynear-symbolic.svg \
      "$out/share/icons/hicolor/symbolic/apps/org.keynear.Keynear-symbolic.svg"
    for size in 48x48 128x128 256x256 512x512; do
      install -Dm644 "packaging/linux/icons/hicolor/$size/apps/org.keynear.Keynear.png" \
        "$out/share/icons/hicolor/$size/apps/org.keynear.Keynear.png"
    done

    install -Dm644 'assets/fonts/PlusJakartaSans[wght].ttf' \
      "$out/share/fonts/truetype/keynear/PlusJakartaSans[wght].ttf"
    install -Dm644 assets/fonts/OFL.txt \
      "$out/share/fonts/truetype/keynear/OFL.txt"

    install -Dm644 packaging/linux/systemd/keynear.service \
      "$out/share/systemd/user/keynear.service"
    substituteInPlace "$out/share/systemd/user/keynear.service" \
      --replace-fail '%h/.local/bin/keynear-linux' "$out/bin/keynear-linux"

    install -Dm644 packaging/linux/integrations/dolphin/keynear-send.desktop \
      "$out/share/kio/servicemenus/keynear-send.desktop"
    substituteInPlace "$out/share/kio/servicemenus/keynear-send.desktop" \
      --replace-fail 'Exec=keynear-settings' "Exec=$out/bin/keynear-settings"

    install -Dm755 packaging/linux/integrations/nautilus/send-via-keynear \
      "$out/share/keynear/integrations/nautilus/send-via-keynear"
    install -Dm644 packaging/linux/integrations/yazi/keynear.yazi/main.lua \
      "$out/share/keynear/integrations/yazi/keynear.yazi/main.lua"
  '';

  meta = {
    description = "Direct, end-to-end encrypted continuity between Linux and Android";
    homepage = "https://github.com/hydrafog/keynear";
    license = lib.licenses.gpl3Only;
    platforms = lib.platforms.linux;
    mainProgram = "keynear-settings";
  };
}

