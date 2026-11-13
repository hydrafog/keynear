{
  description = "Keynear continuity service development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" ];
      forAllSystems = f:
        nixpkgs.lib.genAttrs systems (system:
          f (import nixpkgs { inherit system; })
        );
    in
    {
      devShells = forAllSystems (pkgs: {
        default = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [
            rustc
            cargo
            rustfmt
            clippy
            rust-analyzer
            cargo-ndk
            protobuf
            pkg-config
            jdk21
            gradle
            lefthook
          ];

          buildInputs = with pkgs; [
            gtk4
            libadwaita
            dbus
            pipewire
            bluez
            wayland
          ];

          shellHook = ''
            for tool in cargo rustc protoc java gradle; do
              if ! command -v "$tool" >/dev/null 2>&1; then
                echo "keynear devShell warning: missing tool '$tool'" >&2
              fi
            done
          '';
        };
      });
    };
}

