#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

for required in .editorconfig .gitignore LICENSE README.md deny.toml android/detekt.yml design/tokens.json design/design-lint-baseline.json; do
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

if ! git check-ignore -q design/prototype/ || ! git check-ignore -q out/ || ! git check-ignore -q .env; then
  echo "repo-check: design/prototype/, out/, and .env must be ignored by .gitignore" >&2
  exit 1
fi

for forbidden_tracked in .env out/ design/prototype/ exports/ .prototype/; do
  if [ -n "$(git ls-files "$forbidden_tracked")" ]; then
    echo "repo-check: '$forbidden_tracked' must not be tracked in git" >&2
    exit 1
  fi
done

if git grep -nE "BEGIN (RSA |EC |OPENSSH )?PRIVATE KEY" -- . ':!design/prototype/' ':!out/' >/dev/null 2>&1; then
  echo "repo-check: private key material detected in tracked files" >&2
  exit 1
fi

bash "$repo_root/scripts/validation/design-lint.sh" --baseline "$repo_root/design/design-lint-baseline.json"
bash "$repo_root/scripts/validation/strip-comments.sh" --check

echo "repo-check: OK"
