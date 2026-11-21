#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

max_lines=300

is_exempt() {
  case "$1" in
    */build/*|target/*|design/prototype/*|*/.git/*) return 0 ;;
    *Test.kt|*_tests.rs|*/tests/*) return 0 ;;
    android/designsystem/src/main/java/org/keynear/designsystem/KeynearIcons.kt) return 0 ;;
    android/designsystem/src/main/java/org/keynear/designsystem/Tokens.kt) return 0 ;;
    *) return 1 ;;
  esac
}

offenders=0
while IFS= read -r file; do
  [ -f "$file" ] || continue
  is_exempt "$file" && continue
  lines=$(wc -l < "$file")
  if [ "$lines" -gt "$max_lines" ]; then
    printf 'size-check: %s has %s lines (max %s)\n' "$file" "$lines" "$max_lines" >&2
    offenders=$((offenders + 1))
  fi
done < <(git ls-files '*.rs' '*.kt')

if [ "$offenders" -gt 0 ]; then
  printf 'size-check: FAILED (%s file(s) over %s lines)\n' "$offenders" "$max_lines" >&2
  exit 1
fi

printf 'size-check: OK\n'
