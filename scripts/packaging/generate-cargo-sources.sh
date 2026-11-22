#!/bin/sh
set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)

python3 - "$repo_root/Cargo.lock" "$repo_root/cargo-sources.json" <<'PY'
import json
import sys

REGISTRY = "registry+https://github.com/rust-lang/crates.io-index"
CONFIG = (
    "[source.crates-io]\n"
    "replace-with = \"vendored-sources\"\n"
    "\n"
    "[source.vendored-sources]\n"
    "directory = \"cargo/vendor\"\n"
)

try:
    import tomllib
except ImportError:
    tomllib = None


def parse_toml(text):
    return tomllib.loads(text).get("package", [])


def parse_regex(text):
    packages = []
    current = None
    for raw_line in text.splitlines():
        line = raw_line.strip()
        if line.startswith("["):
            if current is not None:
                packages.append(current)
                current = None
            if line == "[[package]]":
                current = {}
            continue
        if current is None or "=" not in line or line.startswith("#"):
            continue
        key, _, value = line.partition("=")
        key = key.strip()
        value = value.strip()
        if key in ("name", "version", "source", "checksum") and value.startswith('"'):
            end = value.find('"', 1)
            if end > 0:
                current[key] = value[1:end]
    if current is not None:
        packages.append(current)
    return packages


def main():
    lock_path, output_path = sys.argv[1], sys.argv[2]
    with open(lock_path, "r", encoding="utf-8") as handle:
        text = handle.read()

    packages = parse_toml(text) if tomllib is not None else parse_regex(text)
    sources = []
    for package in packages:
        name = package["name"]
        version = package["version"]
        source = package.get("source")
        if source is None:
            continue
        if source.startswith("git+"):
            raise SystemExit(
                "error: git source for {} {} cannot be vendored offline".format(name, version)
            )
        if source != REGISTRY:
            raise SystemExit(
                "error: unsupported source '{}' for {} {}".format(source, name, version)
            )
        checksum = package.get("checksum")
        if not checksum:
            raise SystemExit(
                "error: registry package {} {} has no checksum".format(name, version)
            )
        dest = "cargo/vendor/{}-{}".format(name, version)
        sources.append({
            "type": "archive",
            "archive-type": "tar-gzip",
            "url": "https://static.crates.io/crates/{0}/{0}-{1}.crate".format(name, version),
            "sha256": checksum,
            "dest": dest,
        })
        sources.append({
            "type": "inline",
            "contents": json.dumps(
                {"package": checksum, "files": {}}, separators=(",", ":")
            ),
            "dest": dest,
            "dest-filename": ".cargo-checksum.json",
        })

    sources.append({
        "type": "inline",
        "contents": CONFIG,
        "dest": "cargo",
        "dest-filename": "config",
    })

    with open(output_path, "w", encoding="utf-8") as handle:
        json.dump(sources, handle, indent=4)
        handle.write("\n")

    print("wrote {} ({} entries)".format(output_path, len(sources)))


if __name__ == "__main__":
    main()
PY
