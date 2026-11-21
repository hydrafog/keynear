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
nix develop .#android --command bash -c 'cd android && gradle testDebugUnitTest lintDebug detekt assembleDebug'
```

On non-Nix Ubuntu hosts, install `protobuf-compiler`, `libgtk-4-dev`, and `pkg-config` before building the workspace.

## Design System Tokens

Shared visual tokens (`Plus Jakarta Sans`, light/dark palettes, typography scale, radii, spacing, and motion timings) live in `design/tokens.json`. The GTK stylesheet in `crates/keynear-settings/assets/style.css` declares these as custom properties on `window.keynear-window` and its `.dark` state and references them with `var()`; GTK 4.16 or newer is required. The Android module in `android/designsystem/src/main/java/org/keynear/designsystem/Tokens.kt` holds the equivalent Kotlin token objects. `python3 scripts/validation/token-check.py` verifies that both derived sources match `design/tokens.json` and that no raw visual literals leak into component styles, and it runs in the project quality script and CI.

## Structure and Size Caps

Code is organized into feature modules with one root module re-exporting the public surface. Presentation depends on domain contracts only, domain code avoids UI and transport imports, and shared contracts live in `keynear-protocol` while shared infrastructure lives in `keynear-core`. Domain thresholds such as timeouts, poll intervals, retry counts, and buffer capacities live in typed configuration modules.

Handwritten logic files are capped at 300 lines, functions at 50 lines, cyclomatic complexity at 10, nesting depth at 3, and positional parameters at 4. Generated code, lookup tables, test fixtures, and declarative files are exempt. The limits are enforced by `clippy.toml` and crate-level lint attributes for Rust, `android/detekt.yml` for Kotlin, and a repository size check that runs in hooks and CI.

A targeted `#[allow(clippy::...)]` is permitted only where a cap conflicts with a fixed boundary: public or uniffi-exported signatures, generated protobuf output, and test fixtures. These categories are exempt by policy; any other suppression is treated as a defect.

## Quality Scripts and CI

- `scripts/project/check.sh`: Runs `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `cargo deny check`, `cargo audit`, `cargo llvm-cov --workspace --summary-only`, `scripts/validation/repo-check.sh`, `scripts/validation/size-check.sh`, and Android `testDebugUnitTest lintDebug detekt`.
- `scripts/validation/repo-check.sh`: Verifies `.editorconfig`, `.gitignore` hygiene, absence of committed secrets or `.env` files, required project files such as `design/tokens.json`, ignore rules for generated outputs, and the size cap via `size-check.sh`.
- `scripts/validation/size-check.sh`: Fails when a tracked handwritten Rust or Kotlin logic file exceeds 300 lines, skipping generated code, lookup tables, and test fixtures.
- `scripts/validation/commit-msg.sh`: Enforces `<prefix>: <imperative summary>` commit subjects within 80 characters.
- `scripts/export/export-docs.sh`: Exports documentation and `proto3` schemas deterministically to `out/`.
- `.github/workflows/quality.yml`: Executes Rust checks (`fmt`, `clippy`, `test`, `deny`, `audit`, `llvm-cov`), `repo-check.sh`, `size-check.sh`, `nix flake check --no-build`, and Android `gradle testDebugUnitTest lintDebug detekt assembleDebug`.

## Commit Conventions

Every commit message uses `<prefix>: <imperative summary>` with a prefix from `feat`, `fix`, `docs`, `chore`, `refactor`, `test`, `build`, or `ci` and a maximum length of 80 characters. Local hooks in `lefthook.yml` run these checks automatically before commit and push.
