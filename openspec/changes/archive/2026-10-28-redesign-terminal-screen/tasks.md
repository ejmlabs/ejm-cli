# Tasks

## 1. Screen

- [x] 1.1 Add the transcript module with gutters, light Markdown, tool previews and the welcome card; verify with unit tests for wrapping, Markdown, collapse and ASCII output.
- [x] 1.2 Replace the header and status line with the working line, quiet composer, footer and inline picker; verify with `TestBackend` renders at small and normal sizes.
- [x] 1.3 Add Escape stop, double Ctrl+C exit, Ctrl+O and `?`; verify with render and UI tests.

## 2. Integration

- [x] 2.1 Update the README and CLI guide.
- [x] 2.2 Run `./scripts/check` with at most two Cargo jobs, including the PTY check.

  - Passed after the move to `ejm-tui`: architecture, fmt, strict Clippy, 116 unit tests (one ignored; transcript tests moved to ejm-ui), eight integration tests, release build, Rustdoc, machete, deny and the PTY check.

## Workflow follow-up

- The design kit's guide moved to ejm-ui's `packages/tokens/rust/TUI.md`, updated with ejm-ui's workspace recipes. Synced into `openspec/specs/terminal-screen` and archived on 2026-10-28.
