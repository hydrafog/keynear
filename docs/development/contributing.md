# Contributing

Contributions to [Keynear](https://github.com/hydrafog/keynear) follow a reproducible Nix workflow and automated quality checks.

## Environment Setup

Activate the FOSS development shell with `direnv allow` (which invokes `flake.nix` via `.envrc`) or run commands inside `nix develop -c`:

```bash
nix develop -c cargo check --workspace
nix develop -c cargo test --workspace
```

For Android native and Gradle builds, use the isolated Android devShell:

```bash
nix develop .#android -c bash -c 'cd android && ./gradlew testDebugUnitTest assembleDebug'
```

On non-Nix Ubuntu hosts, install `protobuf-compiler`, `libgtk-4-dev`, and `pkg-config` before building the workspace.

## Design System Tokens

Shared visual tokens (`Plus Jakarta Sans`, light/dark palettes, typography scale, radii, spacing, and motion timings) live in `design/tokens.json` and feed both `crates/keynear-settings/assets/style.css` and `android/app/src/main/java/org/keynear/app/ui/theme/Tokens.kt`.

## Quality Scripts and CI

- `scripts/project/check.sh`: Runs `cargo fmt --check`, `cargo clippy`, `cargo test --workspace`, and `scripts/validation/repo-check.sh`.
- `scripts/validation/repo-check.sh`: Verifies `.editorconfig`, `.gitignore` hygiene, and absence of committed secrets or `.env` files.
- `scripts/validation/commit-msg.sh`: Enforces `<prefix>: <imperative summary>` commit subjects within 80 characters.
- `scripts/export/export-docs.sh`: Exports documentation and `proto3` schemas deterministically to `exports/`.
- `.github/workflows/quality.yml`: Executes Rust checks, `nix flake check --no-build`, and Android `./gradlew testDebugUnitTest assembleDebug`.

## Commit Conventions

Every commit message uses `<prefix>: <imperative summary>` with a prefix from `feat`, `fix`, `docs`, `chore`, `refactor`, `test`, `build`, or `ci` and a maximum length of 80 characters. Local hooks in `lefthook.yml` run these checks automatically before commit and push.
