#!/usr/bin/env bash
set -euo pipefail

msg_file="${1:?usage: commit-msg.sh <commit-msg-file>}"
subject="$(head -n 1 "$msg_file" | tr -d '\r')"

if [ "${#subject}" -gt 80 ]; then
  echo "commit-msg: subject exceeds 80 characters (${#subject})" >&2
  exit 1
fi

if ! printf '%s\n' "$subject" | grep -qE '^(feat|fix|docs|chore|refactor|test|build|ci): [a-z0-9].*'; then
  echo "commit-msg: subject must match '<prefix>: <imperative summary>'" >&2
  exit 1
fi

# Reject any non-empty body lines beyond optional Co-authored-by trailers and git comments.
body_bad="$(tail -n +2 "$msg_file" | tr -d '\r' | grep -vE '^[[:space:]]*$|^[[:space:]]*#|^Co-authored-by: ' || true)"
if [ -n "$body_bad" ]; then
  echo "commit-msg: commit body is disallowed beyond optional Co-authored-by: trailers" >&2
  exit 1
fi
