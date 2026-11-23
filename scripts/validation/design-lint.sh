#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

python3 - "$repo_root" "$@" <<'PY'
import os
import re
import sys

repo_root = sys.argv[1]
explicit = sys.argv[2:]

DESIGN_DIR = "android/designsystem/src/main/java/org/keynear/designsystem"
APP_UI_DIR = "android/app/src/main/java/org/keynear/app/ui"

COMPOSE_COLOR = re.compile(r"Color\s*\(\s*0x[0-9A-Fa-f]{6,8}\s*\)")
BARE_HEX = re.compile(r"(?<![0-9A-Fa-f])0x[0-9A-Fa-f]{8}(?![0-9A-Fa-f])")
HASH_HEX = re.compile(r"(?<![0-9A-Fa-f])#[0-9A-Fa-f]{6}(?:[0-9A-Fa-f]{2})?(?![0-9A-Fa-f])")
DIMENSION = re.compile(r"(?<![0-9A-Za-z_])([0-9]+(?:\.[0-9]+)?)\.(dp|sp)\b")


def excluded(rel):
    parts = rel.replace(os.sep, "/").split("/")
    return "build" in parts or rel.endswith("Test.kt") or os.path.basename(rel) == "Tokens.kt"


def display(full):
    absolute = os.path.abspath(full)
    rel = os.path.relpath(absolute, repo_root)
    return absolute, (rel if not rel.startswith("..") else absolute)


def discover(base):
    found = []
    root = os.path.join(repo_root, base)
    if not os.path.isdir(root):
        return found
    for dirpath, dirnames, filenames in os.walk(root):
        dirnames[:] = [d for d in dirnames if d != "build"]
        for name in filenames:
            if name.endswith(".kt"):
                found.append(os.path.join(dirpath, name))
    return found


def collect():
    if explicit:
        raw = explicit
        flags = [APP_UI_DIR not in os.path.abspath(item).replace(os.sep, "/") for item in explicit]
    else:
        design = discover(DESIGN_DIR)
        app_ui = discover(APP_UI_DIR)
        raw = design + app_ui
        flags = [True] * len(design) + [False] * len(app_ui)
    return [(display(item)[0], display(item)[1], flag) for item, flag in zip(raw, flags)]


def violations(text, dimensions):
    hits = []
    for lineno, line in enumerate(text.splitlines(), 1):
        for pattern in (COMPOSE_COLOR, BARE_HEX, HASH_HEX):
            match = pattern.search(line)
            if match:
                hits.append((lineno, match.group(0)))
                break
        else:
            if dimensions:
                for match in DIMENSION.finditer(line):
                    if float(match.group(1)) != 0:
                        hits.append((lineno, match.group(0)))
                        break
    return hits


def main():
    for item in explicit:
        if not os.path.isfile(os.path.abspath(item)):
            print(f"design-lint: missing path {item}", file=sys.stderr)
            return 1
    problems = []
    for full, shown, dimensions in collect():
        if excluded(shown):
            continue
        with open(full, encoding="utf-8") as handle:
            text = handle.read()
        for lineno, token in violations(text, dimensions):
            print(f"design-lint: {shown}:{lineno}: {token}", file=sys.stderr)
            problems.append((shown, lineno))
    if problems:
        return 1
    print("design-lint: OK")
    return 0


sys.exit(main())
PY
