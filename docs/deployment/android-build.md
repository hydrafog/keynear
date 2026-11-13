# Android Build

The Android companion app (`org.keynear.app`, `versionCode = 2`, `versionName = "0.2.0-cygnus"`) compiles `keynear-core` into `jniLibs` with `cargo-ndk` using 16 KB ELF page alignment before assembling the Gradle APK or AAB.

## Native Library Compilation

The Android devShell (`nix develop .#android`) provides the Rust Android targets (`aarch64-linux-android`, `x86_64-linux-android`), `cargo-ndk`, Android SDK platform `36`, build-tools `36.0.0`, NDK `30.0.16248370`, and `llvm-readelf`. `.cargo/config.toml` passes `-C link-arg=-Wl,-z,max-page-size=16384` for both Android targets. Invoke commands inside the devShell using `--command bash -c` (avoid `-l`, which resets `PATH`):

```bash
nix develop .#android --command bash -c 'cargo ndk -t arm64-v8a -t x86_64 -o android/app/src/main/jniLibs build --release -p keynear-core'
nix develop .#android --command bash -c 'llvm-readelf -l android/app/src/main/jniLibs/arm64-v8a/libkeynear_core.so'
nix develop .#android --command bash -c 'llvm-readelf -l android/app/src/main/jniLibs/x86_64/libkeynear_core.so'
```

Verify that every `LOAD` program header reports `0x4000` alignment.

## UniFFI Kotlin Bindings

`crates/keynear-core` exposes `uniffi-bindgen` behind the `bindgen-cli` feature to generate `android/app/src/main/java/uniffi/keynear_core/keynear_core.kt`:

```bash
cargo build --release -p keynear-core
cargo run -p keynear-core --features bindgen-cli --bin uniffi-bindgen -- generate \
  --library target/release/libkeynear_core.so \
  --language kotlin \
  --out-dir android/app/src/main/java
```

## Gradle Assembly and Unit Tests

`android/app/build.gradle.kts` sets `useLegacyPackaging = false` for 16 KB page alignment on Android 15+ and targets `compileSdk = 36`, `buildToolsVersion = "36.0.0"`, `minSdk = 31`, and `targetSdk = 36`:

```bash
nix develop .#android --command bash -c 'cd android && gradle testDebugUnitTest assembleDebug'
```
