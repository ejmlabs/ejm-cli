# Proposal

## Why

Security audit finding **L2** (`security-baseline-2026-10-19.md`, ejm-cli at `c8e2239`) blocks the coordinated 0.1.0 release. `--base-url` / `EJM_CLI_BASE_URL` (`crates/ejm-cli/src/args.rs:78-84`) is passed unchanged through `Args::api_base_url()` (`args.rs:280-289`) into `OpenAiConfig` / `AnthropicConfig` (`crates/ejm-cli/src/main.rs:211-220`). The harness currently accepts plain `http://` for any host there and then sends the API key. A typo, or a hostile `EJM_CLI_BASE_URL` in the environment, can leak a long-lived provider key in cleartext.

## What Changes

- **BREAKING:** `--base-url` and `EJM_CLI_BASE_URL` must be `https`. The only exception is plain `http` to a loopback host (`localhost`, `127.0.0.0/8`, `::1`). The loopback set is the same one defined by the harness change of the same name, `require-https-provider-endpoints`.
- The CLI checks the URL itself, before it looks for an API key or contacts the network. The error names `--base-url` / `EJM_CLI_BASE_URL` and the rule, and exits non-zero. Users get a clear CLI message instead of a lower-level harness error, and the URL problem is reported even when no key is set.
- `doctor` shows when the configured endpoint is not permitted, instead of printing it as if it were usable (`main.rs:415-427`).
- No change to defaults: `https://api.openai.com/v1` and `https://api.anthropic.com/v1` (`args.rs:280-289`). The `responses` and `chatgpt` providers still reject any endpoint override (`main.rs:243-244`).
- There is no opt-out flag for insecure HTTP.

### Out of scope

- The harness-side enforcement itself (ejm-harness change `require-https-provider-endpoints`).
- Relay and remote transport URLs in the in-flight `deploy-private-remote-relay` change.
- The package registry URL policy.
- Proxy and CA configuration.

## Capabilities

### New Capabilities

- `provider-endpoint-security`: how the CLI validates user-supplied provider base URLs before it uses any credential.

### Modified Capabilities

None. `openspec/specs/` in ejm-cli has no main specs (only `.gitkeep`).

## Impact

- `crates/ejm-cli/src/args.rs` and `crates/ejm-cli/src/main.rs` (provider setup and `doctor`), plus CLI integration tests in `crates/ejm-cli/tests/cli.rs`.
- The CLI check is self-contained and does not require a harness pin bump. `.github/harness-revision` stays at the current commit until harness main contains `require-https-provider-endpoints` ([ejm-harness PR 18](https://github.com/ejmlabs/ejm-harness/pull/18)). That pin bump is a follow-up. The coordinated release still needs both changes.
- `crates/ejm-cli/src/endpoint.rs` holds the shared https-or-loopback check so a later `--registry` flag (`move-registry-to-ejmlabs-domain`) can reuse it. Messages that name `--base-url` stay with the provider flag.
- Docs: `README.md:50` and `docs/ejm-cli.md:20-25` should state the HTTPS rule. Their existing examples already use HTTPS.
- Users with an `http://` remote base URL must switch to HTTPS or a loopback tunnel.

## Decisions

Everett approved this change with these decisions:

1. **Loopback set.** Plain `http` is allowed only for `localhost` (exact, ASCII case-insensitive), IPv4 `127.0.0.0/8`, and `::1`. `0.0.0.0` and LAN or private addresses are rejected.
2. **No opt-out.** There is no flag or environment variable that re-enables remote cleartext HTTP.
3. **CLI check, self-contained.** The CLI validates before it reads an API key. The check does not depend on the harness pin. The harness change remains the sender-side control for other callers. The pin bump waits until that fix is on harness main.
