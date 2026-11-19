#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

if ! command -v python3 >/dev/null 2>&1; then
  echo "design-lint: python3 is required in PATH" >&2
  exit 1
fi

PYTHONPATH="$repo_root/tools/design-lint${PYTHONPATH:+:$PYTHONPATH}" \
  exec python3 -m design_lint --root "$repo_root" "$@"

