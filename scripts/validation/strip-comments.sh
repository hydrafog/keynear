#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

if ! command -v python3 >/dev/null 2>&1; then
  echo "strip-comments: python3 is required in PATH" >&2
  exit 1
fi

PYTHONPATH="$repo_root/tools/strip-comments${PYTHONPATH:+:$PYTHONPATH}" \
  exec python3 -m strip_comments --root "$repo_root" "$@"

