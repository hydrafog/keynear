{
  description = "Keynear continuity service packages, NixOS/Home Manager modules, and development environments";

  nixConfig = {
    extra-substituters = [ "https://hydrafog.cachix.org" ];
    extra-trusted-public-keys = [ "hydrafog.cachix.org-1:UKlU4vpY+vHoft1i+OAVBaVnFaq6udjRM8FPUSAPKLw=" ];
  };

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
      mkPackages = pkgs: {
        keynear = pkgs.callPackage ./packaging/nixos/package.nix { };
        keynear-fuse = pkgs.callPackage ./packaging/nixos/package.nix { enableFuse = true; };
        keynear-gnome-extension = pkgs.callPackage ./packaging/nixos/gnome-extension.nix { };
      };
    in
    {
      overlays.default = final: _prev: mkPackages final;

      nixosModules = {
        default = self.nixosModules.keynear;
        keynear = import ./packaging/nixos/nixos-module.nix;
      };

      homeManagerModules = {
        default = self.homeManagerModules.keynear;
        keynear = import ./packaging/nixos/home-manager-module.nix;
      };

      packages = forAllSystems (system:
        let
          pkgs = import nixpkgs { inherit system; };
          built = mkPackages pkgs;
        in
        built // { default = built.keynear; });

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
              cargo-audit
              cargo-deny
              cargo-llvm-cov
              cargo-sweep
              rustc.llvmPackages.llvm
              llvmPackages.libclang
              protobuf
              pkg-config
              jdk21
              gradle
              lefthook
              jq
            ];

            buildInputs = with pkgs; [
              dbus
              pipewire
              bluez
              wayland
            ];

            LLVM_COV = "${pkgs.rustc.llvmPackages.llvm}/bin/llvm-cov";
            LLVM_PROFDATA = "${pkgs.rustc.llvmPackages.llvm}/bin/llvm-profdata";
            LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
            BINDGEN_EXTRA_CLANG_ARGS = "-I${pkgs.glibc.dev}/include";

            shellHook = ''
              for tool in cargo cargo-audit cargo-deny cargo-llvm-cov cargo-sweep llvm-profdata llvm-cov rustc protoc java gradle lefthook jq; do
                if ! command -v "$tool" >/dev/null 2>&1; then
                  echo "keynear devShell error: missing required tool '$tool'" >&2
                  exit 1
                fi
              done
              echo "keynear devShell: $(rustc --version) | $(cargo --version) | $(cargo-sweep --version) | $(protoc --version) | $(java -version 2>&1 | head -n 1)"
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
              gradle_9
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
                  echo "keynear android devShell error: missing required tool '$tool'" >&2
                  exit 1
                fi
              done
              echo "keynear android devShell: $(rustc --version) | $(cargo --version) | $(cargo-ndk --version) | $(protoc --version) | $(java -version 2>&1 | head -n 1)"
            '';
          };
        });
    };
}
