---
name: ejm-cli-auth
description: Change or review ejm-cli sign-in, saved accounts, credential sources, browser launch, or the loopback callback listener.
---

# ejm-cli sign-in and credentials

Read `crates/ejm-cli/src/auth.rs`, `crates/ejm-cli/src/browser.rs`, [the CLI reference](../../../docs/ejm-cli.md) and the public `ejm-harness-auth` API they call. The harness owns credential storage, refresh and registrations. The CLI host owns only the browser launch, the loopback listener and what the user sees.

- Bind the loopback listener first, then open the browser. Only `browser.rs` may launch an OS URL handler, and `scripts/check_architecture.py` enforces it. `--no-browser` prints a checked sign-in URL instead.
- Validate the authorization URL's origin and reject token hints before showing or opening it. Accept a callback only from the exact loopback host and path. Answer anything else without echoing the code, and keep the listener alive for the real browser.
- Hold codes, state and tokens in `SecretString`. Never accept them as arguments or print them in logs, errors, JSON events or the TUI. Provider keys come from environment variables only.
- Sign-in must time out, honor Ctrl+C and release its port on every path. A second authorization step retries once with fresh state and PKCE on the same listener.
- With several saved accounts, require an explicit `--account`. `accounts` reads cached metadata and never contacts the provider. `logout` says when revocation is unconfirmed.
- Anthropic sign-in stays in the official client. Never collect or reuse Claude subscription tokens.

Test with the loopback fixtures in `auth.rs` and the offline cases in `crates/ejm-cli/tests/cli.rs`. Never sign in against a real provider from a test. Use `$ejm-cli-testing` for fixtures and finish with `./scripts/check`.
