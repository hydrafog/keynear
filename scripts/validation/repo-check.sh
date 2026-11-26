#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

for required in .editorconfig .gitignore .env.example LICENSE README.md deny.toml android/detekt.yml design/tokens.json; do
  if [ ! -f "$required" ]; then
    echo "repo-check: missing required file $required" >&2
    exit 1
  fi
done

for legacy_dir in exports .prototype systemd integrations; do
  if [ -e "$legacy_dir" ]; then
    echo "repo-check: legacy root directory '$legacy_dir' must be removed" >&2
    exit 1
  fi
done

if ! git check-ignore -q design/prototype/node_modules/ || ! git check-ignore -q design/prototype/dist/ || ! git check-ignore -q out/ || ! git check-ignore -q .env || ! git check-ignore -q exports/; then
  echo "repo-check: design/prototype/node_modules/, design/prototype/dist/, out/, exports/, and .env must be ignored by .gitignore" >&2
  exit 1
fi

for forbidden_tracked in .env out/ design/prototype/node_modules/ design/prototype/dist/ exports/ .prototype/; do
  if [ -n "$(git ls-files "$forbidden_tracked")" ]; then
    echo "repo-check: '$forbidden_tracked' must not be tracked in git" >&2
    exit 1
  fi
done

if git grep -nE "BEGIN (RSA |EC |OPENSSH )?PRIVATE KEY" -- . ':!design/prototype/node_modules/' ':!design/prototype/dist/' ':!out/' >/dev/null 2>&1; then
  echo "repo-check: private key material detected in tracked files" >&2
  exit 1
fi

# Verify KEYNEAR_* environment variables read in crates/ are documented in .env.example.
env_missing=0
while IFS= read -r var; do
  [ -n "$var" ] || continue
  if ! grep -qE "^(# *)?${var}=" .env.example; then
    echo "repo-check: env var '$var' read in crates/ is missing from .env.example" >&2
    env_missing=1
  fi
done < <(grep -rhoE 'env::var(_os)?\("(KEYNEAR_[A-Z0-9_]+)"\)' crates/ 2>/dev/null | sed -E 's/.*"([^"]+)".*/\1/' | sort -u)
if [ "$env_missing" -ne 0 ]; then
  exit 1
fi

# Verify relative links in README.md and docs/ resolve to existing files.
python3 - <<'PY'
import pathlib, re, sys

root = pathlib.Path(".")
md_files = [root / "README.md"] + sorted((root / "docs").rglob("*.md"))
link_re = re.compile(r'\]\(([^)\s]+)\)')
broken = []
for md in md_files:
    if not md.is_file():
        continue
    text = md.read_text(encoding="utf-8")
    # Strip fenced code blocks before scanning links.
    text = re.sub(r'```.*?```', '', text, flags=re.S)
    for target in link_re.findall(text):
        if target.startswith(("http://", "https://", "mailto:", "#")):
            continue
        path_part = target.split("#", 1)[0].split("?", 1)[0]
        if not path_part:
            continue
        resolved = (md.parent / path_part).resolve()
        if not resolved.exists():
            broken.append(f"{md}: broken link -> {target}")

if broken:
    for b in broken:
        print(f"repo-check: {b}", file=sys.stderr)
    sys.exit(1)
PY

bash "$repo_root/scripts/validation/size-check.sh"

echo "repo-check: OK"
