## Context

The gate runs `./scripts/check` with uv 0.9.26 installed. Measured: ruff 45 (S101 16 asserts in non-test scripts, ANN 20), mypy 20, ShellCheck 1 (SC2292), clippy 0. This change lands after the release pull request https://github.com/ejmlabs/ejm-cli/pull/38, which also edits `scripts/check`.

## Decisions

- **asserts in scripts become explicit checks** that raise with a message; `assert` is skipped under `-O`.
- **`print_stdout` and `unreachable_pub` stay off**: the binary prints by design, and `pub` in a binary crate exports nothing.
- **No loosening.** Escape hatches are line-scoped and reasoned. There is no explicit `any`, no `@ts-ignore`, `@ts-expect-error` or `@ts-nocheck`, no file-wide or unlimited disable, and no strict flag or rule turned off. A remaining `eslint-disable-next-line <rule> -- <reason>`, `# noqa: <code>` with a reason, `# type: ignore[<code>]` with a reason, `#[expect(<lint>, reason = "...")]` or `# shellcheck disable=<code>` with a reason names one rule and says why.

## Risks

Small.
