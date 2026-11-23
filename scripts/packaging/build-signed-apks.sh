#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

has_env_signing=0
if [ -n "${KEYNEAR_KEYSTORE_PATH:-}" ] && [ -n "${KEYNEAR_KEYSTORE_PASSWORD:-}" ] && [ -n "${KEYNEAR_KEY_ALIAS:-}" ] && [ -n "${KEYNEAR_KEY_PASSWORD:-}" ]; then
  has_env_signing=1
fi

if [ "$has_env_signing" -eq 0 ] && [ ! -f "$repo_root/android/keystore.properties" ]; then
  cat >&2 <<'EOF'
error: release signing credentials not configured.
Provide either android/keystore.properties (storeFile, storePassword, keyAlias, keyPassword)
or set KEYNEAR_KEYSTORE_PATH, KEYNEAR_KEYSTORE_PASSWORD, KEYNEAR_KEY_ALIAS, and KEYNEAR_KEY_PASSWORD.
EOF
  exit 1
fi

if [ -n "${KEYNEAR_KEYSTORE_PATH:-}" ] && [ ! -f "$KEYNEAR_KEYSTORE_PATH" ]; then
  echo "error: keystore file not found at $KEYNEAR_KEYSTORE_PATH" >&2
  exit 1
fi

run_in_android_env() {
  if command -v cargo-ndk >/dev/null 2>&1 && { [ -n "${ANDROID_HOME:-}" ] || [ -n "${ANDROID_SDK_ROOT:-}" ]; }; then
    bash -c "$1"
  elif command -v nix >/dev/null 2>&1; then
    nix develop .#android --command bash -c "$1"
  else
    echo "error: Android SDK / cargo-ndk not found and nix is unavailable" >&2
    exit 1
  fi
}

gradle_cmd="./gradlew"
if ! [ -x "$repo_root/android/gradlew" ]; then
  gradle_cmd="gradle"
fi

run_in_android_env "
  set -euo pipefail
  cargo ndk -t arm64-v8a -t x86_64 -o android/core-ffi/src/main/jniLibs build --release -p keynear-ffi
  cd android
  if command -v gradle >/dev/null 2>&1; then
    gradle assembleGmsRelease assembleFossRelease
  else
    $gradle_cmd assembleGmsRelease assembleFossRelease
  fi
"

mkdir -p "$repo_root/out/apk"
for flavor in gms foss; do
  apk="$repo_root/android/app/build/outputs/apk/$flavor/release/app-$flavor-release.apk"
  if [ ! -f "$apk" ]; then
    echo "error: expected signed APK at $apk (check keystore configuration)" >&2
    exit 1
  fi
  cp "$apk" "$repo_root/out/apk/keynear-$flavor-release.apk"
  echo "Signed APK: out/apk/keynear-$flavor-release.apk"
done

if command -v apksigner >/dev/null 2>&1; then
  for flavor in gms foss; do
    apksigner verify --verbose "$repo_root/out/apk/keynear-$flavor-release.apk"
  done
elif [ -n "${ANDROID_HOME:-}" ] && [ -x "$ANDROID_HOME/build-tools/36.0.0/apksigner" ]; then
  for flavor in gms foss; do
    "$ANDROID_HOME/build-tools/36.0.0/apksigner" verify --verbose "$repo_root/out/apk/keynear-$flavor-release.apk"
  done
fi

