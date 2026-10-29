# Tasks

## 1. Shared blocks (ejm-ui)

- [x] 1.1 One-line `SessionHeader`, `WorkspaceLayout` without the resource row, `ResourceLine` empty when idle; verify with `crates/ejm-tui` tests.
- [x] 1.2 `Status::line`; `CommandMenu` column rule in Rust and Ink; verify with Rust tests, Ink tests and the parity check.

## 2. Screen (ejm-cli)

- [x] 2.1 First run without a provider; verify with a unit test for the guarded submit and a PTY look.
- [x] 2.2 Header, counts in the working line, turn summary block; verify with `TestBackend` renders.
- [x] 2.3 Session titles from the first prompt; verify with a UI test.
- [x] 2.4 Picker footer keys, `/status` panel, panel labels; verify with render tests.

## 3. Integration

- [x] 3.1 Update the README and CLI guide.
- [x] 3.2 Run `./scripts/check` with at most two Cargo jobs, and the ejm-ui Rust and Ink checks.

  - `./scripts/check` passed: architecture, fmt, strict Clippy, 121 unit tests (one ignored), eight integration tests, release build, Rustdoc, machete, deny and the PTY check. The PTY check's help step now waits for `Shortcuts`. One earlier run failed a remote PTY case with exit -15; every case passed alone and the full gate then passed.
  - ejm-ui: `crates/ejm-tui` tests, Clippy and fmt, 144 Ink tests, typecheck, parity (40 states in each target), the em dash check and the theme sync check. The full `npm run check` was not run, because gallery and shots files there carry other work.

## Workflow follow-up

- Synced into `openspec/specs/terminal-screen` and archived on 2026-10-28. ejm-ui's matching recipes are on its `feat/terminal-polish` branch; bump `.github/ui-source.json` once that merges.
