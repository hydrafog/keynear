#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: export-website.sh [OUTPUT_PATH]

Export tracked source files under design/website/ into a deterministic
single-file text bundle.

Options:
  -h, --help    Show this help message and exit
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

if [[ $# -gt 1 ]]; then
  usage >&2
  exit 1
fi

repo_root="$(git rev-parse --show-toplevel)"
output_path="${1:-$repo_root/website-export.txt}"

mapfile -t files < <(
  git -C "$repo_root" ls-files -- 'design/website' \
    | grep -vE '(\.png|\.jpe?g|\.webp|\.ico|\.woff2?|package-lock\.json)$' \
    | LC_ALL=C sort
)

if [[ ${#files[@]} -eq 0 ]]; then
  echo "error: no tracked source files found under design/website/" >&2
  exit 1
fi

tmp_file="$(mktemp)"
trap 'rm -f "$tmp_file"' EXIT

{
  printf '# Keynear Website Source Export\n'
  printf '# Files: %d\n\n' "${#files[@]}"
  for rel in "${files[@]}"; do
    printf '================================================================================\n'
    printf 'FILE: %s\n' "$rel"
    printf '================================================================================\n'
    cat "$repo_root/$rel"
    printf '\n'
  done
} > "$tmp_file"

mkdir -p "$(dirname "$output_path")"
mv "$tmp_file" "$output_path"
trap - EXIT

printf 'Exported %d files to %s\n' "${#files[@]}" "$output_path"

