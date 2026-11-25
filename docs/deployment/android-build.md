# Android Build

The Android companion app (`org.keynear.app`, `versionCode = 9`, `versionName = "1.3.0-phoenix"`) compiles `keynear-ffi` into `libkeynear_core.so`. The build uses `cargo-ndk` with 16 KB ELF page alignment and places the library in `android/core-ffi/src/main/jniLibs` before assembling the multi-module Gradle build (`:core-ffi`, `:designsystem`, `:app`). It ships these launcher assets.

- An Adaptive Icon (`mipmap-anydpi-v26/ic_launcher.xml`) with background, foreground, and Android 13+ monochrome layers.
- Fallback `mipmap-*/ic_launcher*.png` rasters.

## Product Flavors (`gms` and `foss`)

The file `android/app/build.gradle.kts` defines the `distribution` flavor dimension with two flavors.

- The default `gms` flavor links `com.google.android.gms:play-services-auth-api-phone` through `gmsImplementation`. It compiles `src/gms/java/org/keynear/app/otp/SmsUserConsentManager.kt` for Google Play Services SMS User Consent OTP forwarding.
- The `foss` flavor excludes all Google Play Services dependencies. It compiles `src/foss/java/org/keynear/app/otp/SmsUserConsentManager.kt` with a clean availability notification fallback and matches the F-Droid recipe in `packaging/fdroid/org.keynear.app.yml` and the metadata in `fastlane/metadata/android/en-US/`.

## Native Library Compilation

The Android devShell (`nix develop .#android`) provides these tools.

- The Rust Android targets `aarch64-linux-android` and `x86_64-linux-android`, plus `cargo-ndk`.
- `cmake` for `aws-lc-sys` / `rustls` `X25519MLKEM768` cross-compilation.
- Android SDK platform `36`, build-tools `36.0.0`, NDK `30.0.16248370`, and `llvm-readelf`.

The file `.cargo/config.toml` passes `-C link-arg=-Wl,-z,max-page-size=16384` for both Android targets. Run commands inside the devShell with `--command bash -c`, and avoid `-l` because it resets `PATH`:

```bash
nix develop .#android --command bash -c 'cargo ndk -t arm64-v8a -t x86_64 -o android/core-ffi/src/main/jniLibs build --release -p keynear-ffi'
nix develop .#android --command bash -c 'llvm-readelf -l android/core-ffi/src/main/jniLibs/arm64-v8a/libkeynear_core.so'
nix develop .#android --command bash -c 'llvm-readelf -l android/core-ffi/src/main/jniLibs/x86_64/libkeynear_core.so'
```

Verify that every `LOAD` program header reports `0x4000` alignment. The quality workflow runs the same check for both ABIs after `cargo ndk` builds the libraries. It reads each `LOAD` alignment with `llvm-readelf -l`, rejects any alignment below `16384` bytes, and fails on a missing library or a library with no `LOAD` headers. A native load smoke then confirms that each file is a shared object through `llvm-readelf -h` and exports UniFFI bridge symbols through `llvm-nm -D`. The workflow adds the NDK LLVM tool directory to `PATH` so both checks use the same tools as the local devShell.

## UniFFI Kotlin Bindings

The `crates/keynear-ffi` crate exposes `uniffi-bindgen` (`src/bin/uniffi_bindgen.rs`) behind the `bindgen-cli` feature. The `:core-ffi:generateUniffiBindings` task generates `uniffi/keynear_core/keynear_core.kt` into `android/core-ffi/build/generated/source/uniffi/` during Gradle builds. The `vfs.rs` and `continuity.rs` modules export these records and functions:

- `FfiChunkCheckpoint`, `FfiReadPlan`, `FfiVfsNode`, and `FfiResumeDecision`.
- `resumable_transfer_capability_flags`, `transfer_resume_decision`, `vfs_read_plan`, `vfs_root_node`, and `provide_vfs_read_fd`.
- `FfiCapabilityPair`, `FfiNotificationDescriptor`, `FfiMirroredNotification`, `FfiMediaState`, `FfiCallAction`, `FfiCallPhase`, `FfiNotificationAction`, `FfiPlayback`, and `FfiMediaCommand`.
- `continuity_capability_flags`, `mirror_notification`, `validate_notification_reply`, `call_action_allowed`, and `media_command_allowed`.

The `provide_vfs_read_fd` function consumes the write end of a `ParcelFileDescriptor` pipe created by `RemoteDocumentStream`:

```bash
nix develop .#android --command bash -c 'cd android && gradle :core-ffi:generateUniffiBindings'
```

## Gradle Assembly, Lint, and Unit Tests

The Gradle files `android/app/build.gradle.kts` and `android/core-ffi/build.gradle.kts` share these settings.

- `useLegacyPackaging = false` for 16 KB page alignment on Android 15 and later.
- `compileSdk = 36`, `buildToolsVersion = "36.0.0"`, `minSdk = 31`, and `targetSdk = 36`.
- `dev.detekt` (`android/detekt.yml`) across `:designsystem` and `:app`.

The compatibility aggregate tasks `testDebugUnitTest` and `lintDebug` exercise both `gms` and `foss`:

```bash
nix develop .#android --command bash -c 'cd android && gradle testDebugUnitTest lintDebug detekt assembleDebug'
nix develop .#android --command bash -c 'cd android && gradle assembleFossRelease'
```

## Signed Release APKs

The `android/app/build.gradle.kts` release build type signs `gms` and `foss` APKs when signing credentials are provided via either environment variables or `android/keystore.properties` (both ignored by git). When no credentials are present, release tasks still succeed and produce unsigned APKs for downstream signers such as F-Droid.

1. Generate a release keystore (once, stored outside the repository):

```bash
keytool -genkeypair -v -keystore ~/keynear-release.jks -alias keynear \
  -keyalg RSA -keysize 4096 -validity 10000
```

2. Configure credentials via environment variables or `android/keystore.properties`:

```properties
# android/keystore.properties
storeFile=/path/to/keynear-release.jks
storePassword=...
keyAlias=keynear
keyPassword=...
```

Or using environment variables: `KEYNEAR_KEYSTORE_PATH`, `KEYNEAR_KEYSTORE_PASSWORD`, `KEYNEAR_KEY_ALIAS`, and `KEYNEAR_KEY_PASSWORD`.

3. Build and verify signed APKs into `out/apk/`:

```bash
bash scripts/packaging/build-signed-apks.sh
```

In GitHub Actions, `.github/workflows/release.yml` builds signed `gms` and `foss` release APKs on `v*` tags or manual dispatch using repository secrets `ANDROID_KEYSTORE_BASE64`, `ANDROID_KEYSTORE_PASSWORD`, `ANDROID_KEY_ALIAS`, and `ANDROID_KEY_PASSWORD`.
