#!/bin/sh
set -eu

cd "$(dirname "$0")/../.."

flatpak-cargo-generator Cargo.lock -o cargo-sources.json
