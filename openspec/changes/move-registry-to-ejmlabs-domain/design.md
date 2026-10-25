# Design

## Context

See proposal.md for why the registry host is moving. This design covers how ejm-cli follows that move. It does not edit `ejm-harness`.

Everett approved the host on 2026-10-20: the default registry is `https://registry.ejmlabs.com`. Retired sessions are discarded locally, not imported. Harness pull request [#19](https://github.com/ejmlabs/ejm-harness/pull/19) is implemented at `6c90a0d297861b78d83b958697c7e7aed9304409` and is not on harness main. That commit exports `DISCARD_RETIRED_REGISTRY_COMMAND` as `ejm-cli package discard-retired-session`. The CLI provides that command. Same-origin legacy handoff stays the harness function `migrate_registry_token`, exposed as `ejm-cli package migrate-credentials --old-clients-stopped` (already on this pin).

The CLI pin `.github/harness-revision` is `42e4f9cf33bd3525aec344cfdffabc251271f782`. At that commit `DEFAULT_REGISTRY` is still `https://harness.meetgerry.ai`, while `save_registry_token` / `load_registry_token` / `delete_registry_token` use the encrypted package credentials vault. `RegistryClient::new` allows HTTP only when the host string is `localhost`, `127.0.0.1`, or `[::1]`, does not reject `meetgerry.ai`, and still honors process proxy variables (no loopback `no_proxy()`). The discard API and the new default are not on harness main, so this implementation does not move the pin.

`https://harness.ejmlabs.com` stays the harness product page in this repo's site. It is not the registry.

## Goals / Non-Goals

**Goals:**

- Package commands and `doctor` use `--registry`, then `EJM_CLI_REGISTRY_URL`, then `https://registry.ejmlabs.com`.
- Reject `meetgerry.ai` and its subdomains, non-loopback HTTP, `0.0.0.0`, and LAN hosts before any request or session load. No fallback to another origin.
- Keep each saved session bound to its origin. Logout removes the local session and does not fail when the legacy file is already gone.
- Discard leftover meetgerry sessions locally, without importing them and without a network request.
- Document the default, the override, and the discard command.

**Non-Goals:**

- Editing `../ejm-harness`, deploying the registry, or moving `.github/harness-revision` before the needed harness commit is on main.
- Renaming `gerry.package.json`, the `gerry-*` Worker warning, or the harness product links.
- A registry config file, or remote revocation of tokens issued by `meetgerry.ai`.
- Importing an old session into `https://registry.ejmlabs.com`.

## Decisions

1. **The default origin is `https://registry.ejmlabs.com`.** Decided by Everett. The CLI passes that value to `RegistryClient::new` and does not read `DEFAULT_REGISTRY` while that constant is still `https://harness.meetgerry.ai`. Using the constant today would select the retired host and then fail the CLI's own rejection. When harness main publishes the same URL in `DEFAULT_REGISTRY`, a later pin bump can switch the CLI to the constant. Until then the CLI literal is the default users get. Docs and `--help` quote this URL.

2. **Discard retired sessions. Do not import them.** The command is `ejm-cli package discard-retired-session`, the value of `DISCARD_RETIRED_REGISTRY_COMMAND` at harness `6c90a0d`. The pinned crate does not export that constant, so `args.rs` owns the same string until the pin moves. The command `lstat`s `<package root>/registry-session.json` and unlinks that path. It does not follow a symlink and does not read a symlink target. A symlink, a non-regular file that is not a directory, a file whose mode is not private, a file larger than 8192 bytes, and a private file that is not a usable origin/token record are removed without a network request. A directory is not deleted: the command prints the full path and tells the user to remove it. A private readable session whose origin is not a meetgerry host is left in place. This build does not call `discard_retired_registry_sessions`, so a vault record stays until the pin includes that function or the user logs out of that origin.

   The pinned harness does not export `retired_registry_session_notice`. `package login` and `doctor` call the local function of that name. A readable retired session produces the harness text at `6c90a0d`: the meetgerry.ai registry is retired, the default is `https://registry.ejmlabs.com`, the old session is not sent or reused, and the user discards it with `ejm-cli package discard-retired-session` and signs in again. A symlink, loose mode, malformed JSON, a missing origin, or an oversize file makes `login`, `logout`, and `whoami` print the full path and that command when the vault has no session for the origin in effect, and send nothing. A directory makes them print the path and say the CLI does not delete directories. When the vault already has a session for the origin in effect, logout/whoami/publisher/publish use that record even if a leftover legacy path remains.

   Same-origin legacy handoff stays `migrate_registry_token` via `package migrate-credentials --old-clients-stopped`. On this pin, usable tokens come from the encrypted vault.

3. **Logout removes the vault session for the origin in effect.** Logout loads that origin's vault record, asks that origin to revoke the token, then calls `delete_registry_token`. A missing vault record reports that there is no session for the origin. A vault record for another origin is not sent and not deleted. A leftover retired legacy file does not block logout of a different origin's vault session. `6c90a0d` adds `discard_retired_registry_sessions`, and it is not on main. The pin stays.

4. **`--registry` is global, and only package commands and `doctor` read it.** `doctor` has to say whether the origin came from the flag. The flag is not given to clap's env injector, so the CLI can tell a flag from `EJM_CLI_REGISTRY_URL`. Precedence is flag, then env, then the default. An empty env var is not an override. `run` and the other model commands ignore the value, including when it is invalid. `discard-retired-session` ignores it too, so a retired value in the environment cannot block the cleanup and cannot cause a request.

5. **The CLI validates the registry URL before `RegistryClient::new`.** The predicate `host_is_loopback` matches the rule in ejm-cli pull request [#11](https://github.com/ejmlabs/ejm-cli/pull/11) and ejm-ide: ASCII case-insensitive `localhost`, IPv4 `is_loopback()` (`127.0.0.0/8`), IPv6 `is_loopback()` (`::1`). `0.0.0.0`, LAN addresses, `::`, and `::ffff:127.0.0.1` are not loopback. That helper lives in `args.rs` so the base-URL change can call it instead of copying it. Pull request 11 has not landed, so this change does not edit `api_base_url()`.

   `meetgerry.ai` and any subdomain are rejected with a message that the meetgerry registry is retired and that the default is `https://registry.ejmlabs.com`. `notmeetgerry.ai` is a normal HTTPS origin. The process does not retry the default. Plain-origin failures (path, userinfo, query, fragment, other schemes) are rejected in the CLI as well.

   The pinned `RegistryClient::new` still rejects `http://127.0.0.2` even though this predicate accepts it. `doctor` uses the CLI predicate, so it can show that origin. A package command that must open a client still calls `RegistryClient::new` and surfaces the harness error. `127.0.0.1`, `localhost`, and `[::1]` work on the pin. Widening the harness loopback set is a harness change and is not on main.

6. **`doctor` prints the choice and keeps going.** A valid origin is `Registry: <origin> (default|EJM_CLI_REGISTRY_URL|--registry)`. A rejected override is marked rejected, names the reason, and the rest of the report still prints. `doctor` then exits non-zero. It does not open a registry connection. When a retired session file is present, `doctor` names `ejm-cli package discard-retired-session` and does not print the token. Both registry lines use `display_registry_url`. That helper matches the host, port, and path shape of `display_without_userinfo` on ejm-cli pull request 11. It drops userinfo, query, and fragment, and strips control characters. A value that does not parse is `(invalid registry URL)`, never the raw input.

7. **Tests stay local.** No test contacts `registry.ejmlabs.com`, `meetgerry.ai`, or a paid provider. The default origin is asserted as a string. Live registry calls use a loopback fixture. The fixture token must not appear on stdout or stderr.

## Risks / Trade-offs

- The CLI default and `DEFAULT_REGISTRY` differ until harness PR 19 is on main. Package commands in this CLI use the CLI literal, so they do not follow the constant back to meetgerry.
- `http://127.0.0.2` passes the CLI rule and fails inside `RegistryClient::new` on the pin. The harness loopback set has to grow before that host works as a registry.
- Logout on this pin calls `delete_registry_token` for the origin in effect. A leftover plaintext legacy file is left for `discard-retired-session` or `migrate-credentials`.
- A hostile `EJM_CLI_REGISTRY_URL` can point package commands at another HTTPS registry. The process prints that origin, and `doctor` shows the source. Install stays disabled until `--trust`.

## Migration Plan

`discard-retired-session` ships in the same ejm-cli release as the harness pin bump to the merged harness pull requests #18 and #19. Neither ships alone. No release note tells users to discard until both are in.

After that combined release, registry commands use `https://registry.ejmlabs.com` unless `--registry` or `EJM_CLI_REGISTRY_URL` is set. A leftover meetgerry session can then be discarded locally. There is no copy of the old bearer onto the new host. Rollback does not restore a discarded file. This change does not add `ejm-cli package migrate-credentials --old-clients-stopped`.

Harness `6c90a0d` sets `DEFAULT_REGISTRY` to `https://registry.ejmlabs.com`, rejects `meetgerry.ai` and its subdomains, and names `ejm-cli package discard-retired-session`. Its `RegistryClient::new` still allows HTTP only for `localhost`, `127.0.0.1`, and `[::1]`, so `http://127.0.0.2` still fails inside the client. Harness pull request [#18](https://github.com/ejmlabs/ejm-harness/pull/18) (`f8ffd3a3075e52dd40b7d639aec956f40e640953`, HTTPS provider endpoints) is also open. Neither commit is on the current pin `42e4f9cf33bd3525aec344cfdffabc251271f782`. `.github/harness-revision` stays at that pin until both are on main.

## Resolved questions

- The host is `https://registry.ejmlabs.com`.
- Retired sessions are discarded, not imported. The command is `ejm-cli package discard-retired-session`.
- Same-origin legacy handoff stays `migrate_registry_token`. This CLI does not call it.
- `--registry` is global. Model commands ignore it.

## Temporary proxy compatibility adapter

**Gap:** the pinned public `RegistryClient::new` has no option to force a direct HTTP loopback route and still uses process proxy variables. The harness owns transport, so the CLI must not build a second registry client or import private sibling code.

**Adapter:** `Args::registry_choice` rejects an otherwise valid HTTP loopback choice when a nonempty `HTTP_PROXY`, `HTTPS_PROXY`, or `ALL_PROXY` variable, uppercase or lowercase, is set. It stores only the variable name, not its value. `NO_PROXY` does not override this temporary refusal. HTTPS, model commands, replay, and local retired-session discard retain their current behavior. Doctor uses the same choice and marks the origin rejected. Package commands stop before opening the client or loading credentials. Existing pure origin selection remains independent of process proxy state.

This branch has no provider endpoint guard to reuse. Use the existing validated origin and standard-library environment lookup for this narrow refusal. Do not import the provider branch's private module or duplicate its transport. This adds at most six environment lookups only when resolving an HTTP registry choice, with no new dependency or background work.

**Compatibility checks:** isolated real-binary fixtures supply only synthetic registry credentials and a secret-bearing local proxy URL. Passive loopback listeners prove that neither the registry nor proxy receives a connection on rejection. Test flag and environment selection, each proxy variable form, doctor output, preservation of the saved session, and absence of token/proxy values in output. No new HTTP parser, server, dependency, persistent state, process service, or background work is added.

**Upstream owner and removal:** `ejm-harness-packages::RegistryClient::new` must bypass proxies for accepted HTTP loopback origins using reqwest `no_proxy()`. Its Wiremock route fixture exercises a synthetic bearer. Verify and merge that harness change through the human gates first. The CLI then adopts a compatible reviewed full commit from protected main and reconciles credential storage through its public APIs. Remove this adapter only after the committed pin's direct-route and credential checks pass. Keep dependency direction CLI to public harness packages, preserve `.github/harness-revision` here, and do not claim a release or deployment.
