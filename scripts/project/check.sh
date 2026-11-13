#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

bash scripts/validation/repo-check.sh

if command -v cargo >/dev/null 2>&1; then
  cargo fmt --check
  cargo check --workspace
  cargo test --workspace
elif command -v nix >/dev/null 2>&1; then
  nix develop -c cargo fmt --check
  nix develop -c cargo check --workspace
  nix develop -c cargo test --workspace
else
  echo "error: neither cargo nor nix found in PATH" >&2
  exit 1
fi

