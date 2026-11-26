#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

bash scripts/validation/repo-check.sh
bash scripts/validation/size-check.sh
python3 scripts/validation/token-check.py
bash scripts/validation/design-lint.sh

run_rust_checks() {
  cargo fmt --check
  cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
  cargo test --locked --workspace --all-features
  cargo deny check
  cargo audit
  cargo llvm-cov --locked -p keynear-protocol -p keynear-core -p keynear-ffi -p keynear-ipc --all-features --fail-under-lines 75 --summary-only
}

if command -v cargo >/dev/null 2>&1 && command -v cargo-audit >/dev/null 2>&1 && command -v cargo-deny >/dev/null 2>&1 && command -v cargo-llvm-cov >/dev/null 2>&1; then
  run_rust_checks
elif command -v nix >/dev/null 2>&1; then
  nix develop -c bash -c 'set -euo pipefail; cargo fmt --check; cargo clippy --locked --workspace --all-targets --all-features -- -D warnings; cargo test --locked --workspace --all-features; cargo deny check; cargo audit; cargo llvm-cov --locked -p keynear-protocol -p keynear-core -p keynear-ffi -p keynear-ipc --all-features --fail-under-lines 75 --summary-only'
else
  echo "error: required Rust/security/coverage tools not found and nix missing from PATH" >&2
  exit 1
fi

if [[ "${KEYNEAR_SKIP_ANDROID:-0}" != "1" ]]; then
  if [[ -n "${ANDROID_HOME:-}" || -n "${ANDROID_SDK_ROOT:-}" || -f android/local.properties ]]; then
    (cd android && ./gradlew testDebugUnitTest lintDebug detekt)
  elif command -v nix >/dev/null 2>&1; then
    nix develop .#android --command bash -c 'cd android && gradle testDebugUnitTest lintDebug detekt'
  else
    echo "error: Android checks requested but neither Android SDK nor nix found in PATH" >&2
    exit 1
  fi
fi
