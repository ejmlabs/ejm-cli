## ADDED Requirements

### Requirement: Python is strictly typed and linted
Python under `scripts/` SHALL pass `mypy --strict` (2.4.0) and ruff (0.17.0) with the shared `ruff.strict.toml`, run from a locked development group.

#### Scenario: An unannotated function
- **WHEN** a change adds a Python function without annotations
- **THEN** the check fails

### Requirement: Rust uses the shared lint tables
The workspace `Cargo.toml` SHALL carry the shared Rust lint tables (clippy `all` and `pedantic` denied, plus the shared restriction lints) at the same or a stricter level, and clippy SHALL run with `-D warnings` on all targets.

#### Scenario: An unwrap in library code
- **WHEN** a change adds `.unwrap()` outside a reasoned `#[expect]`
- **THEN** clippy fails

### Requirement: Shell scripts pass strict ShellCheck
`scripts/check`, `scripts/install-hooks`, `.githooks/pre-commit` and `scripts/sync-design-kit.sh` SHALL pass ShellCheck 0.11.0 with the shared `.shellcheckrc` (all optional checks except SC2250).

#### Scenario: A masked return value
- **WHEN** a script uses a command substitution whose failure would be ignored
- **THEN** the shell check fails

### Requirement: CI enforces the strict settings
`./scripts/check`, which the shared gate already runs, SHALL run every tool above.

#### Scenario: A violation reaches a pull request
- **WHEN** a pull request introduces any finding from these tools
- **THEN** the repository's required check fails

### Requirement: Escape hatches are line-scoped and reasoned
The repository SHALL contain no explicit `any`, no `@ts-ignore`, `@ts-expect-error` or `@ts-nocheck`, no unlimited or file-wide lint disable, and no strict compiler flag or lint rule turned off. Every remaining suppression SHALL name one rule and carry a reason.

#### Scenario: Undescribed suppression
- **WHEN** a change adds `// eslint-disable-next-line` without a rule name and a ` -- reason`, or a Rust `#[allow]` without a reason
- **THEN** the repository's check fails
