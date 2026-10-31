## 1. Tooling
- [x] 1.1 `pyproject.toml`, `uv.lock` (uv 0.9.26), `ruff.toml`, `.shellcheckrc`, Cargo tables.
- [x] 1.2 `scripts/check` requires `uv` and runs `uv run --locked --group dev ruff check .`, `... mypy`, `... shellcheck <files>`.
## 2. Fixes
- [x] 2.1 ruff 45, mypy 20, ShellCheck 1.
## 3. Verification
- [x] 3.1 `./scripts/check` passes with the pinned harness revision beside the checkout.
