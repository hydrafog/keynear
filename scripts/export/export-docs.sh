#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

out_dir="${1:-exports}"
mkdir -p "$out_dir"
cp -r docs "$out_dir/docs"
cp README.md "$out_dir/README.md"
cp -r proto "$out_dir/proto"
echo "export-docs: exported docs, README.md, and proto to $out_dir"

