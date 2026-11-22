#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: export-prototype.sh [OUTPUT_PATH]

Export tracked source files under design/prototype/ into a deterministic
single-file text bundle.

Options:
  -h, --help    Show this help message and exit
EOF
}

if [ "${1:-}" = "-h" ] || [ "${1:-}" = "--help" ]; then
  usage
  exit 0
fi

if [ "$#" -gt 1 ]; then
  printf 'Error: expected at most one output path argument\n' >&2
  usage >&2
  exit 1
fi

if [ "${1:-}" != "" ] && [[ "$1" == -* ]]; then
  printf 'Error: unknown option: %s\n' "$1" >&2
  usage >&2
  exit 1
fi

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
if repo_root="$(git -C "$script_dir" rev-parse --show-toplevel 2>/dev/null)"; then
  :
else
  repo_root="$(cd -- "$script_dir/../.." && pwd)"
fi

proto_dir="$repo_root/design/prototype"
if [ ! -d "$proto_dir" ]; then
  printf 'Error: prototype directory not found: %s\n' "$proto_dir" >&2
  exit 1
fi

out_path="${1:-$repo_root/out/keynear-prototype.txt}"
out_dir="$(dirname -- "$out_path")"
mkdir -p -- "$out_dir"

tmp_file="$(mktemp "$out_dir/.export-prototype.XXXXXX")"
cleanup() {
  rm -f -- "$tmp_file"
}
trap cleanup EXIT

declare -a raw_files=()
for root_file in index.html package.json tsconfig.json vite.config.ts; do
  if [ -f "$proto_dir/$root_file" ]; then
    raw_files+=("design/prototype/$root_file")
  fi
done

for sub_dir in public src; do
  if [ -d "$proto_dir/$sub_dir" ]; then
    while IFS= read -r -d '' abs_file; do
      rel_file="${abs_file#"$repo_root/"}"
      case "$rel_file" in
        */node_modules/*|*/dist/*|*/package-lock.json)
          continue
          ;;
      esac
      raw_files+=("$rel_file")
    done < <(find "$proto_dir/$sub_dir" -type f -print0)
  fi
done

if [ "${#raw_files[@]}" -eq 0 ]; then
  printf 'Error: no source files found under %s\n' "$proto_dir" >&2
  exit 1
fi

mapfile -t sorted_files < <(printf '%s\n' "${raw_files[@]}" | LC_ALL=C sort)

file_count=0
for rel_file in "${sorted_files[@]}"; do
  printf '===== %s =====\n' "$rel_file" >> "$tmp_file"
  cat -- "$repo_root/$rel_file" >> "$tmp_file"
  printf '\n' >> "$tmp_file"
  file_count=$((file_count + 1))
done

mv -- "$tmp_file" "$out_path"
printf 'Exported %d prototype files to %s\n' "$file_count" "$out_path"
