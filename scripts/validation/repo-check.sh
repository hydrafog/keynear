#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

for required in .editorconfig .gitignore LICENSE README.md; do
  if [ ! -f "$required" ]; then
    echo "repo-check: missing required file $required" >&2
    exit 1
  fi
done

if ! git check-ignore -q .prototype/ || ! git check-ignore -q .env; then
  echo "repo-check: .prototype/ and .env must be ignored by .gitignore" >&2
  exit 1
fi

if git ls-files --error-unmatch .env >/dev/null 2>&1; then
  echo "repo-check: .env must not be tracked in git" >&2
  exit 1
fi

if [ -n "$(git ls-files .prototype/)" ]; then
  echo "repo-check: .prototype/ must not be tracked in git" >&2
  exit 1
fi

if git grep -nE "BEGIN (RSA |EC |OPENSSH )?PRIVATE KEY" -- . ':!.prototype/' >/dev/null 2>&1; then
  echo "repo-check: private key material detected in tracked files" >&2
  exit 1
fi

echo "repo-check: OK"

