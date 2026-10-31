## Why

The owner asked for strict type checking and strict linting in every ejmlabs repository where they apply, from one shared layer, enforced by CI. The Rust code already denies clippy `pedantic` and the restriction lints. The Python scripts behind `scripts/check` have no linter or type checker, and the shell scripts are unchecked.

## What Changes

- Root `pyproject.toml` (no package; development group `ruff==0.17.0`, `mypy==2.4.0`, `shellcheck-py==0.11.0.1`) with `uv.lock`; `ruff.toml` extends `ruff.strict.toml`; `[tool.mypy]` strict over `scripts/`.
- Copy `.shellcheckrc`; ShellCheck covers `scripts/check`, `scripts/install-hooks`, `.githooks/pre-commit` and `scripts/sync-design-kit.sh`.
- Add `undocumented_unsafe_blocks` and `unused_must_use` to the workspace tables (already covered by `unsafe_code = "forbid"` and `warnings = "deny"`; added so the sync check matches).
- `scripts/check` requires `uv` and runs the three tools.

## Capabilities

### New Capabilities

- `strict-lint`: the strict type-checking and lint settings this repository holds, how CI enforces them, and how suppressions are allowed.

## Impact

`scripts/*.py`, `scripts/check`, `Cargo.toml`, new `pyproject.toml`, `uv.lock`, `ruff.toml`, `ruff.strict.toml`, `.shellcheckrc`. `site/dist/design-kit` is a generated copy and is not linted.
