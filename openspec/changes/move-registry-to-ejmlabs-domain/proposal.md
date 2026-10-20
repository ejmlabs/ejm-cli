# Proposal

## Why

ejm-cli's registry commands always use the harness constant `DEFAULT_REGISTRY` (`crates/ejm-cli/src/packages.rs:85`, `RegistryClient::new(DEFAULT_REGISTRY)`), which today is `https://harness.meetgerry.ai`, a domain Everett does not operate. The ejm-harness change `move-registry-to-ejmlabs-domain` moves that default to `ejmlabs.com`. The CLI picks up the new default automatically. A follow-on is still needed because the CLI has its own registry-facing behavior that has to change:

- **Docs name the old host.** `docs/ejm-cli.md:188` says "Registry commands use `https://harness.meetgerry.ai`", and `docs/ejm-cli.md:198` says tokens live in `registry-session.json`, which is no longer true since the encrypted vault.
- **No override exists.** There is no flag, env var or config for the registry, so the CLI cannot point at a local development registry or a staging host without a code change.
- **Logout is broken.** `package logout` loads the vault token, then deletes the legacy `registry-session.json` file (`packages.rs:151-156`). It never deletes the vault record for the origin, and it fails if the legacy file is absent.
- **A required command is missing.** The harness tells users to run `ejm-cli package migrate-credentials --old-clients-stopped` (`ejm-harness-packages/src/registry/credentials.rs:52`), but the CLI has no such command. A leftover legacy session for the retired origin would block `package login` with no way out.

## What Changes

- **BREAKING:** registry commands use the new harness default, recommended `https://registry.ejmlabs.com`. Sessions saved for `harness.meetgerry.ai` stop working.
- New minimal override: `--registry URL` on `ejm-cli package ...`, plus the `EJM_CLI_REGISTRY_URL` env var. Precedence is flag, then env, then default. Values must pass the harness origin rules: HTTPS plain origin, loopback HTTP only, and the retired `meetgerry.ai` origin rejected. Validation runs before any network request or token load.
- Registry sessions are scoped to the origin in effect: `login`, `whoami`, `publisher`, `publish` and `logout` use only that origin's session. `logout` removes that origin's vault record.
- A way to discard a session saved for the retired origin, locally and without contacting any host. A blocked `login` explains how.
- `doctor` shows the registry origin in effect and where it came from (default, env or flag).
- Docs and help text name the new default and the override, and drop the meetgerry host.

### Out of scope

- Changing the harness default or server config (ejm-harness change).
- Renaming `gerry.package.json`, and the wider meetgerry/Gerry sweep.
- Registry config files: the CLI has no config file today, and none is added.
- New registry features.

## Capabilities

### New Capabilities

- `package-registry-selection`: how the CLI chooses and validates the registry origin, scopes registry sessions to it, and handles sessions saved for the retired origin.

### Modified Capabilities

None. `openspec/specs/` has no main specs.

## Impact

- **Code:** `crates/ejm-cli/src/packages.rs` (registry client construction, login, logout, a retired-session discard command) and `crates/ejm-cli/src/main.rs` (`doctor`, `main.rs:411`). The `package` subcommands are defined in `PackageCommand` (`packages.rs:15`). Integration tests go in `crates/ejm-cli/tests/cli.rs`.
- **Docs:** `docs/ejm-cli.md:188-198` and the `package` help text.
- **Dependency:** this needs the harness change of the same name, which provides the new default, the retired-origin rejection and the discard API. The CLI builds against the sibling `../ejm-harness` path (`Cargo.toml:18-23`), so the two must land together.
- **Coordination:** the Engineer is sweeping every `meetgerry` reference across the five repos. This change covers only the registry default and the CLI behavior that directly depends on it. Other meetgerry/Gerry references in ejm-cli, left to the sweep:
  - `docs/ejm-cli.md:136`: the `gerry.package.json` manifest name
  - `docs/contributing.md:3`: "Gerry platform" wording
  - `docs/ejm-cli.md:188` is the only `meetgerry` host reference, and this change rewrites it

## Open Questions

1. **Override names.** Proposed: `--registry` and `EJM_CLI_REGISTRY_URL`, matching `--base-url` / `EJM_CLI_BASE_URL`. Should the flag be global, or only on `package`? Recommendation: `package` only.
2. **Is an override wanted at all?** It mainly serves local development (`http://127.0.0.1:8787` with `wrangler dev`) and staging. An env override is also something a hostile environment can set. The risk is limited, because registry digests come from the same origin and installs stay disabled until trusted, but a user could still be pointed at a hostile registry. Recommendation: keep it, show it in `doctor`, and print the non-default origin on every registry command.
3. **Retired-session handling.** Recommendation: an explicit discard command (for example `package logout --retired`) rather than deleting automatically.
4. **New default origin and the loopback set:** see the ejm-harness change and `require-https-provider-endpoints`.

## Proxy transport compatibility follow-up

The current harness pin `4a751c6a3f24d7d143140f79ff53517fb93e3531` permits environment proxies for an HTTP loopback registry. That can carry its registry bearer away from the loopback endpoint. `ejm-harness-packages::RegistryClient::new` owns the transport fix. The CLI must reject HTTP loopback registry use when a nonempty `HTTP_PROXY`, `HTTPS_PROXY`, or `ALL_PROXY` variable, in either case, is set until it adopts the reviewed fix. This temporary guard fails closed even when `NO_PROXY` is set. It names the variable without its value, blocks before a session lookup or request, and appears as rejected in doctor.

No provider endpoint helper from PR #11 is present on this branch. Add only a configuration adapter using the existing `url` parser and environment API. Keep the harness pin and CI gates intact. Verify both owners with local synthetic-token fixtures, adopt a compatible reviewed harness commit only after protected merge, then remove the adapter when the pinned direct-route and credential contract checks pass. Public docs must state the temporary restriction and recovery.
