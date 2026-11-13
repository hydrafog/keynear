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

