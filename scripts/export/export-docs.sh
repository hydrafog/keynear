#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

out_dir="${1:-exports}"
mkdir -p "$out_dir"
out_file="$out_dir/export.md"

: > "$out_file"

printf '# Project Export\n\n' >> "$out_file"
printf '_Generated %s from `%s`_\n\n' \
  "$(date -u +'%Y-%m-%dT%H:%M:%SZ')" "$repo_root" >> "$out_file"

if [[ -f README.md ]]; then
  printf '\n---\n\n## Source: `README.md`\n\n' >> "$out_file"
  cat README.md >> "$out_file"
  printf '\n' >> "$out_file"
fi

if [[ -d docs ]]; then
  while IFS= read -r -d '' f; do
    rel="${f#./}"
    printf '\n---\n\n## Source: `%s`\n\n' "$rel" >> "$out_file"
    cat "$f" >> "$out_file"
    printf '\n' >> "$out_file"
  done < <(find docs -type f -name '*.md' -print0 | sort -z)
fi

project_slug="$(basename "$repo_root" | tr '[:upper:]' '[:lower:]')"
artifacts_dir="${AGENTS_ARTIFACT_ROOT:-.agents/artifacts}/$project_slug"
if [[ -d "$artifacts_dir" ]]; then
  while IFS= read -r -d '' f; do
    rel="${f#./}"
    printf '\n---\n\n## Source: `%s`\n\n' "$rel" >> "$out_file"
    cat "$f" >> "$out_file"
    printf '\n' >> "$out_file"
  done < <(find "$artifacts_dir" -type f \( -name '*.md' -o -name '*.log' \) -print0 | sort -z)
fi

if [[ -d proto ]]; then
  while IFS= read -r -d '' f; do
    rel="${f#./}"
    case "$f" in
      *.proto) lang=proto ;;
      *.yaml|*.yml) lang=yaml ;;
      *.json) lang=json ;;
      *) lang=text ;;
    esac
    printf '\n---\n\n## Source: `%s`\n\n```%s\n' "$rel" "$lang" >> "$out_file"
    cat "$f" >> "$out_file"
    printf '\n```\n' >> "$out_file"
  done < <(find proto -type f -print0 | sort -z)
fi

echo "export-docs: exported docs, README.md, artifacts, and proto into $out_file"