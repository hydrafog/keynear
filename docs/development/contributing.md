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

Shared visual tokens (`Plus Jakarta Sans`, light/dark palettes, typography scale, radii, spacing, and motion timings) live in `design/tokens.json` and feed both `crates/keynear-settings/assets/style.css` and `android/designsystem/src/main/java/org/keynear/designsystem/Tokens.kt`.

## Quality Scripts and CI

- `scripts/project/check.sh`: Runs `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `cargo deny check`, `cargo audit`, `cargo llvm-cov --workspace --summary-only`, `scripts/validation/repo-check.sh`, and Android `testDebugUnitTest lintDebug detekt`.
- `scripts/validation/repo-check.sh`: Verifies `.editorconfig`, `.gitignore` hygiene, absence of committed secrets or `.env` files, `scripts/validation/strip-comments.sh --check`, and runs `scripts/validation/design-lint.sh`.
- `scripts/validation/design-lint.sh`: Thin wrapper around `tools/design-lint` checking design-token parity against `design/tokens.json` and scanning Android Kotlin/XML, GTK4 CSS, and Rust UI/tray/notification sources against `design/design-lint-baseline.json`.
- `scripts/validation/strip-comments.sh`: Thin wrapper around `tools/strip-comments` enforcing comment hygiene across `crates/` and `android/` (`--check`).
- `scripts/validation/commit-msg.sh`: Enforces `<prefix>: <imperative summary>` commit subjects within 80 characters.
- `scripts/export/export-docs.sh`: Exports documentation and `proto3` schemas deterministically to `out/`.
- `.github/workflows/quality.yml`: Executes Rust checks (`fmt`, `clippy`, `test`, `deny`, `audit`, `llvm-cov`), `repo-check.sh`, `nix flake check --no-build`, and Android `gradle testDebugUnitTest lintDebug detekt assembleDebug`.

## Commit Conventions

Every commit message uses `<prefix>: <imperative summary>` with a prefix from `feat`, `fix`, `docs`, `chore`, `refactor`, `test`, `build`, or `ci` and a maximum length of 80 characters. Local hooks in `lefthook.yml` run these checks automatically before commit and push.
