# Android Build

The Android companion app (`org.keynear.app`, `versionCode = 6`, `versionName = "1.0.0-vega"`) compiles `keynear-ffi` (`libkeynear_core.so`) into `android/core-ffi/src/main/jniLibs` with `cargo-ndk` using 16 KB ELF page alignment before assembling the multi-module Gradle build (`:core-ffi`, `:designsystem`, `:app`).

## Native Library Compilation

The Android devShell (`nix develop .#android`) provides the Rust Android targets (`aarch64-linux-android`, `x86_64-linux-android`), `cargo-ndk`, `cmake` (for `aws-lc-sys` / `rustls` `X25519MLKEM768` cross-compilation), Android SDK platform `36`, build-tools `36.0.0`, NDK `30.0.16248370`, and `llvm-readelf`. `.cargo/config.toml` passes `-C link-arg=-Wl,-z,max-page-size=16384` for both Android targets. Invoke commands inside the devShell using `--command bash -c` (avoid `-l`, which resets `PATH`):

```bash
nix develop .#android --command bash -c 'cargo ndk -t arm64-v8a -t x86_64 -o android/core-ffi/src/main/jniLibs build --release -p keynear-ffi'
nix develop .#android --command bash -c 'llvm-readelf -l android/core-ffi/src/main/jniLibs/arm64-v8a/libkeynear_core.so'
nix develop .#android --command bash -c 'llvm-readelf -l android/core-ffi/src/main/jniLibs/x86_64/libkeynear_core.so'
```

Verify that every `LOAD` program header reports `0x4000` alignment.

## UniFFI Kotlin Bindings

`crates/keynear-ffi` exposes `uniffi-bindgen` (`src/bin/uniffi_bindgen.rs`) behind the `bindgen-cli` feature. `:core-ffi:generateUniffiBindings` generates `uniffi/keynear_core/keynear_core.kt` into `android/core-ffi/build/generated/source/uniffi/` during Gradle builds:

```bash
nix develop .#android --command bash -c 'cd android && gradle :core-ffi:generateUniffiBindings'
```

## Gradle Assembly, Lint, and Unit Tests

`android/app/build.gradle.kts` and `android/core-ffi/build.gradle.kts` set `useLegacyPackaging = false` for 16 KB page alignment on Android 15+, target `compileSdk = 36`, `buildToolsVersion = "36.0.0"`, `minSdk = 31`, and `targetSdk = 36`, and apply `dev.detekt` (`android/detekt.yml`) across `:designsystem` and `:app`:

```bash
nix develop .#android --command bash -c 'cd android && gradle testDebugUnitTest lintDebug detekt assembleDebug'
```
