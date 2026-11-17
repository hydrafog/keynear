#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if ! command -v python3 >/dev/null 2>&1; then
  echo "ui-ux-check: python3 is required in PATH" >&2
  exit 1
fi

exec python3 "$script_dir/ui-ux-check.py" "$@"

