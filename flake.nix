{
  description = "Keynear continuity service development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, rust-overlay }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" ];
      forAllSystems = f:
        nixpkgs.lib.genAttrs systems (system: f system);
    in
    {
      devShells = forAllSystems (system:
        let
          pkgs = import nixpkgs { inherit system; };
          pkgsAndroid = import nixpkgs {
            inherit system;
            overlays = [ rust-overlay.overlays.default ];
            config = {
              allowUnfree = true;
              android_sdk.accept_license = true;
            };
          };
          androidComposition = pkgsAndroid.androidenv.composeAndroidPackages {
            platformVersions = [ "36" ];
            buildToolsVersions = [ "36.0.0" ];
            includeNDK = true;
            ndkVersion = "30.0.16248370";
          };
          androidSdk = androidComposition.androidsdk;
          androidNdk = "${androidSdk}/libexec/android-sdk/ndk/30.0.16248370";
          rustAndroidToolchain = pkgsAndroid.rust-bin.stable.latest.default.override {
            targets = [
              "aarch64-linux-android"
              "x86_64-linux-android"
            ];
            extensions = [ "rust-src" ];
          };
        in
        {
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
              jq
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

          android = pkgsAndroid.mkShell {
            nativeBuildInputs = with pkgsAndroid; [
              rustAndroidToolchain
              cargo-ndk
              cmake
              protobuf
              pkg-config
              jdk21
              gradle
              androidSdk
              llvmPackages.bintools
            ];

            ANDROID_HOME = "${androidSdk}/libexec/android-sdk";
            ANDROID_SDK_ROOT = "${androidSdk}/libexec/android-sdk";
            ANDROID_NDK_ROOT = androidNdk;
            NDK_HOME = androidNdk;
            JAVA_HOME = "${pkgsAndroid.jdk21}/lib/openjdk";
            GRADLE_OPTS = "-Dorg.gradle.project.android.aapt2FromMavenOverride=${androidSdk}/libexec/android-sdk/build-tools/36.0.0/aapt2";

            shellHook = ''
              for tool in cargo rustc cargo-ndk cmake protoc java gradle llvm-readelf; do
                if ! command -v "$tool" >/dev/null 2>&1; then
                  echo "keynear android devShell warning: missing tool '$tool'" >&2
                fi
              done
            '';
          };
        });
    };
}
