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
- Depends on the harness change of the same name. The CLI builds against the sibling `../ejm-harness` path (`Cargo.toml:18-23`), so the two need to land together for 0.1.0.
- Docs: `README.md:50` and `docs/ejm-cli.md:20-25` should state the HTTPS rule. Their existing examples already use HTTPS.
- Users with an `http://` remote base URL must switch to HTTPS or a loopback tunnel.

## Open Questions

1. **Loopback set** (shared with ejm-harness). Recommendation: `localhost`, `127.0.0.0/8` and `::1`, matching ejm-ide. The audit text names only `127.0.0.1`, `::1` and `localhost`.
2. **Insecure opt-out flag.** Recommendation: **no**. A flag or env override would bring back the hostile-environment risk.
3. **Should the CLI also validate, or just rely on the harness?** Recommendation: validate in both. The harness is the security boundary. The CLI check gives a clear flag/env-specific error and fails before any credential lookup.
