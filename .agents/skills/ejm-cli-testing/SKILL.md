---
name: ejm-cli-testing
description: Add or fix ejm-cli tests, replay fixtures, render checks, the PTY check, or test coverage in CI.
---

# ejm-cli testing

Read `crates/ejm-cli/tests/cli.rs`, the `#[cfg(test)]` modules beside the code, `scripts/check_terminal.py` and [the contributor guide](../../../docs/contributing.md).

- No network and no paid provider calls. Use `--replay`, `--replay-file` with public `ModelResponse` JSON, or a local fixture. Remove provider environment variables such as `EJM_CLI_API_KEY` from child processes.
- Integration tests run the real binary that Cargo builds, with a fresh temporary state directory, and read plain or JSON output through the helpers in `tests/cli.rs`. Assert the user visible result, for example that a denied shell tool fails and executes nothing.
- Render tests use ratatui's `TestBackend` at small and normal sizes. Unit tests cover Unicode byte boundaries versus screen columns, bounded transcripts, argument parsing and callback validation.
- Prove a new test fails without the change when you can. Do not add a test that restates the implementation.
- Test code may use `unwrap` inside test-only modules through the documented expectation. Production code may not.
- The PTY check, `python3 scripts/check_terminal.py target/release/ejm-cli`, proves real input, offline output and terminal restoration on Linux and macOS only.

Run the narrow test first: `cargo test -p ejm-cli NAME` or `cargo test -p ejm-cli --test cli`. Keep Cargo jobs at two or fewer. Then run `./scripts/check` and report real results.
