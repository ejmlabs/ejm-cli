---
name: ejm-cli-terminal
description: Change or diagnose ejm-cli keyboard input, wrapping, scrolling, terminal rendering, paste, or terminal restoration.
---

# ejm-cli terminal work

Read `src/ui.rs`, `src/render.rs` and `src/app.rs` under `crates/ejm-cli`, plus [the CLI reference](../../../docs/ejm-cli.md).

Preserve the terminal guard across fallible setup, normal return, errors and unwinding. Raw mode, alternate screen, cursor, bracketed paste and mouse capture must all be restored. Terminal control sequences from providers and tools remain text, including OSC clipboard sequences.

Verify Unicode byte boundaries separately from screen-column width. Test narrow screens, long lines, multiline drafts, paste, wrapped cursors, scrolling while streaming, help/session overlays and failed submissions. Input must remain independent of model progress.

Use `cargo test -p ejm-cli` for deterministic state/render checks. After a build, run `python3 scripts/check_terminal.py target/release/ejm-cli` for real PTY interaction and normal/SIGTERM restoration. Honor `CARGO_TARGET_DIR` when set. This script proves Linux/macOS behavior only. Do not infer Windows terminal behavior from it.

Keep redraws coalesced, and retain bounded visible history. Add only tests that prove an actual changed interaction or lifecycle invariant.
