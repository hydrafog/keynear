# Contributing

Contributions to [Keynear](https://github.com/hydrafog/keynear) follow a reproducible Nix workflow and automated quality checks.

## Environment Setup

Activate the FOSS development shell with `direnv allow`, which invokes `flake.nix` through `.envrc`. You can also run commands inside `nix develop -c`:

```bash
nix develop -c cargo check --workspace
nix develop -c cargo test --workspace
```

For Android native and Gradle builds, use the isolated Android devShell:

```bash
nix develop .#android --command bash -c 'cd android && gradle testDebugUnitTest lintDebug detekt assembleDebug'
```

On non-Nix Ubuntu hosts, install `protobuf-compiler`, `libdbus-1-dev`, and `pkg-config` before building the workspace.

## Design System Tokens

Shared visual tokens (`Plus Jakarta Sans`, light and dark palettes, typography scale, radii, spacing, and motion timings) live in `design/tokens.json`. The Android module at `android/designsystem/src/main/java/org/keynear/designsystem/Tokens.kt` defines the equivalent Kotlin token objects.

The script `python3 scripts/validation/token-check.py` checks that `Tokens.kt`, `Theme.kt`, and the bundled `plus_jakarta_sans.ttf` font asset match `design/tokens.json` and that no raw visual literals leak into `Controls*.kt` or `KeynearIcons.kt`. It runs in the project quality script and CI.

## Structure and Size Caps

Code is organized into feature modules, with one root module re-exporting the public surface. Presentation depends on domain contracts only, and domain code avoids UI and transport imports. Shared contracts live in `keynear-protocol`, while shared infrastructure lives in `keynear-core`. Domain thresholds such as timeouts, poll intervals, retry counts, and buffer capacities live in typed configuration modules.

Handwritten logic files are capped at 300 lines, functions at 50 lines, cyclomatic complexity at 10, nesting depth at 3, and positional parameters at 4. Generated code, lookup tables, test fixtures, and declarative files are exempt. For Rust, `clippy.toml` and crate-level lint attributes enforce the limits. For Kotlin, `android/detekt.yml` does. A repository size check runs in hooks and CI.

A targeted `#[allow(clippy::...)]` is permitted only where a cap conflicts with a fixed boundary. The allowed cases are public or uniffi-exported signatures, generated protobuf output, and test fixtures. These categories are exempt by policy, and any other suppression is treated as a defect.

## Quality Scripts and CI

The repository provides the following quality scripts and CI checks.

- The script `scripts/project/check.sh` runs `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `cargo deny check`, `cargo audit`, `cargo llvm-cov --workspace --summary-only`, `scripts/validation/repo-check.sh`, `scripts/validation/size-check.sh`, and Android `testDebugUnitTest lintDebug detekt`.
- The script `scripts/validation/repo-check.sh` verifies `.editorconfig` and `.gitignore` hygiene, the absence of committed secrets or `.env` files, required project files such as `design/tokens.json`, ignore rules for generated outputs, and the size cap through `size-check.sh`.
- The script `scripts/validation/size-check.sh` fails when a tracked handwritten Rust or Kotlin logic file exceeds 300 lines. It skips generated code, lookup tables, and test fixtures.
- The script `scripts/validation/commit-msg.sh` enforces `<prefix>: <imperative summary>` commit subjects within 80 characters.
- The script `scripts/export/export-docs.sh` exports documentation and `proto3` schemas deterministically to `out/`.
- The script `scripts/export/export-prototype.sh` exports tracked `design/prototype/` source files deterministically to `out/keynear-prototype.txt`.
- The workflow `.github/workflows/quality.yml` executes the Rust checks (`fmt`, `clippy`, `test`, `deny`, `audit`, `llvm-cov`), `repo-check.sh`, `size-check.sh`, `nix flake check --no-build`, and Android `gradle testDebugUnitTest lintDebug detekt assembleDebug`.

## Commit Conventions

Every commit message uses `<prefix>: <imperative summary>` with a prefix from `feat`, `fix`, `docs`, `chore`, `refactor`, `test`, `build`, or `ci`. Messages are at most 80 characters long. Local hooks in `lefthook.yml` run these checks automatically before commit and push.
