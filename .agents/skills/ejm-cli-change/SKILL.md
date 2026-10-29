---
name: ejm-cli-change
description: Implement ejm-cli commands, session interaction, or TUI changes through the public ejm-harness API, and place new code by the crate's layout and shared helpers.
---

# ejm-cli changes

Read `AGENTS.md` and [the contributor guide](../../../docs/contributing.md). Map the request to argument parsing, app state, input handling, rendering, and documentation before editing. Load `ejmlabs-rust` too; this skill adds the crate's own layout and helpers.

- Model state belongs in `App`. Terminal drawing stays in `render`. Keep run policy, providers, tools, storage and resource admission in the sibling ejm-harness project.
- Use the public harness methods. A new UI command must preserve cancellation, grants, session identity and durable event ordering.
- Keep user input after failed submissions. Never make paste submit a prompt. Bound added history, buffers and background tasks.
- Wire flags to their real owning configuration and show effective grants. API keys come from environment variables; saved sign-ins use the shared `ejm-harness-auth` store. Credentials never appear in flags or logs. Keep browser launch and callback listening in the CLI host.

Add an observable check for changed behavior. Use `$ejm-cli-terminal` for terminal lifecycle work and `$ejm-cli-performance` for latency or memory work. Finish with `./scripts/check` and report actual results and limits.

## Code layout

The crate is one binary, `crates/ejm-cli`. `main.rs` declares every top-level module and composes the app. [The ownership table](../../../docs/contributing.md#ownership-and-rust-design-rules) says what each module owns.

- One file per module in `src/`. A module with submodules is `name.rs` plus a `name/` folder, as `remote/controller.rs` and `remote/controller/network.rs` are. Never add a `mod.rs`. `remote/mod.rs` predates this rule; move it to `remote.rs` only in a change of its own.
- Top-level modules are private, so their items are crate-visible. Write new items as `pub(crate)` across the crate (as `endpoint.rs` and `terminal.rs` do) or `pub(super)` between `remote/` siblings (as `remote/vault.rs` does).
- Errors are `anyhow::Result` with `.context`, `bail!` and `ensure!`. When a caller branches on a failure, add a small error enum like `endpoint::HttpsLoopbackError`. A message used in more than one place is named once in its owning module, as `args.rs` names `CLEARTEXT_BASE_URL` and `proxy_block_message`.
- Limits and timeouts are named constants at the top of the module that owns them, for example `MAX_DISPLAY_BYTES` in `app.rs`, `MAX_PROMPT_BYTES` in `remote/protocol.rs` and the timeouts in `remote/transport.rs`.
- Cleanup that must run on every path is a guard with `Drop`: `ui::TerminalGuard`, `auth::LoginGuard`, `NetworkGuard` in `remote/controller.rs`.
- Unit tests end the module in `#[cfg(test)] mod tests` with one `#[expect(clippy::unwrap_used, reason = "...")]`. A large suite moves to a sibling `name_tests.rs`, attached with `#[cfg(test)] #[path = "name_tests.rs"] mod tests;`. Fixtures live in `name_fixture.rs`, `remote/test_support.rs` or `remote/testdata/`. Integration tests in `tests/` run the real binary through the `cli` and `provider_cli` helpers in `tests/cli.rs`.

## Shared helpers

Reuse these instead of writing another. Extend one when it falls short, keeping each caller's error text.

| Need | Use |
| --- | --- |
| Print or draw provider, tool, package, registry or error text | `render::safe_text` |
| Keep the newest part of bounded text | `app::truncate_start` |
| Check a provider URL, its proxy variables, or show a URL | `endpoint::require_https_or_loopback`, `endpoint::blocking_proxy_variable`, `endpoint::display_without_userinfo` |
| Open a browser | `browser::launch`, the only OS URL launcher |
| The sign-in engine and the selected account's credential source | `auth::engine`, `auth::credentials` |
| The package manager | `packages::manager` |
| Slash commands | `commands::catalog`, `commands::parse` |
| Raw mode, alternate screen and restoration | `ui::TerminalGuard`, `terminal::Modes` |
| Ctrl+C and SIGTERM | `ui::with_shutdown`, `ui::termination_signal` |
| Pairing and invitation text on a private screen | `remote::protected` |
| Cancellable blocking credential work in `remote/` | `remote::vault::blocking` |
| Wall-clock milliseconds in `remote/` | `remote::now_ms` |
| A gated offline tool in contract tests | `remote::test_support::Gate` |

Known drift: `remote::transport::now_ms` repeats `remote::now_ms` with a saturating result, and the std mutexes in `ui.rs` and `terminal_input.rs` handle poisoning inline. A new std mutex adds the one crate-private `lock` helper that `ejmlabs-rust` describes; do not copy the inline forms.
