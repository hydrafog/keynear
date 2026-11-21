{ lib, stdenvNoCC }:

let
  extensionUuid = "keynear-clipboard@hydrafog.github.com";
in
stdenvNoCC.mkDerivation {
  pname = "gnome-shell-extension-keynear-clipboard";
  version = "1.1.0";

  src = builtins.path {
    name = "keynear-gnome-extension-src";
    path = ../gnome + "/${extensionUuid}";
  };

  dontBuild = true;

  installPhase = ''
    runHook preInstall
    mkdir -p "$out/share/gnome-shell/extensions/${extensionUuid}"
    cp -r ./* "$out/share/gnome-shell/extensions/${extensionUuid}/"
    runHook postInstall
  '';

  passthru = {
    inherit extensionUuid;
    extensionPortalSlug = "keynear-clipboard";
  };

  meta = {
    description = "GNOME Shell D-Bus clipboard bridge extension for Keynear";
    homepage = "https://github.com/hydrafog/keynear";
    license = lib.licenses.gpl3Only;
    platforms = lib.platforms.linux;
  };
}
