# Android Build

The Android companion app (`org.keynear.app`) compiles `keynear-core` into `jniLibs` using `cargo-ndk` with 16 KB ELF page alignment before assembling the Gradle APK or AAB.

## Native Library Compilation

```bash
nix develop -c cargo ndk -t arm64-v8a -t x86_64 -o android/app/src/main/jniLibs build --release -p keynear-core
```

## Gradle Assembly

`android/app/build.gradle.kts` sets `useLegacyPackaging = false` for 16 KB page alignment on Android 15+ and targets `compileSdk = 36`, `minSdk = 31`, and `targetSdk = 36`:

```bash
cd android
gradle assembleRelease
```

