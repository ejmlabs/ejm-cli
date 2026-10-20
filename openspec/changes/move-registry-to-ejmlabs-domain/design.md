# Design

## Context

See proposal.md for why the registry host is moving. This design covers how ejm-cli follows that move. It does not edit `ejm-harness`.

On 2026-10-20 the harness branch `openspec/move-registry-to-ejmlabs-domain` was not on GitHub yet. Coordination below is against public `ejmlabs/ejm-harness` main `8d3dc39b8bf38e0b54ef4315fd6c5c58a4f304e0`, which is nine commits ahead of the CLI pin `.github/harness-revision` (`4a751c6a3f24d7d143140f79ff53517fb93e3531`).

At the pin, `save_registry_token` and `load_registry_token` still read and write plaintext `registry-session.json`. Current main has moved those sessions into the encrypted vault (`ejm-harness-packages` / `registry-session`, keyed by the normalized origin). It exports `delete_registry_token` and `migrate_registry_token`. `save` and `load` refuse to continue when a legacy file exists and that origin is not marked migrated. The error tells the user to run `ejm-cli package migrate-credentials --old-clients-stopped`. The CLI has no such command. `migrate_registry_token` imports the file only when its `origin` matches the origin argument, then deletes the file. A file whose origin is `https://harness.meetgerry.ai/` does not import into the new default.

`package logout` in `crates/ejm-cli/src/packages.rs` loads a token, asks that origin to revoke it, then `remove_file`s `registry-session.json`. It never calls `delete_registry_token`. After the vault move, a signed-in user with no legacy file fails logout on the missing file, and the vault record remains. There is a separate top-level `logout` for model-provider accounts. This change does not alter that command.

`RegistryClient::new` is the only origin check. It allows HTTPS, and HTTP only for `localhost`, `127.0.0.1`, and `[::1]`. It requires a plain origin. It still accepts `https://harness.meetgerry.ai`. The auth Worker origin and the GitHub OAuth callback are set in the harness registry config and README, not in this repo.

Everett chooses the new host. The candidates are `harness.ejmlabs.com` and `registry.ejmlabs.com`. This repo's public page already links `https://harness.ejmlabs.com/` as the harness product page.

## Goals / Non-Goals

**Goals:**

- Package commands and `doctor` use one registry origin: `--registry`, then `EJM_CLI_REGISTRY_URL`, then the harness `DEFAULT_REGISTRY`.
- Reject unsafe and retired origins before any request or session load, using the harness rules.
- Keep each session bound to the origin that issued it. Logout removes that origin's vault record.
- Give the blocked user the command the harness already names, and make that command discard a retired-origin session locally.
- Document the new default, the override, the vault, and the upgrade. Leave every other Gerry string unchanged.

**Non-Goals:**

- Choosing the host, changing `DEFAULT_REGISTRY`, editing Worker config, or renaming `gerry-registry` / `gerry-registry-auth`. Those belong to the harness change.
- Renaming `gerry.package.json`, rewriting "Gerry platform" in `docs/contributing.md`, or retargeting the harness product links on the CLI site.
- A registry config file, a second host string in the CLI, or remote revocation of tokens issued by `harness.meetgerry.ai`.

## Decisions

1. **The host is the harness constant.** The CLI passes `DEFAULT_REGISTRY` into `RegistryClient::new` and does not declare its own host. Docs and `--help` quote that constant's value once Everett has picked it and the harness change has set it. `https://registry.ejmlabs.com` stays the Product Manager's recommendation in proposal.md. It is not a second constant. The auth Worker `REGISTRY_ORIGIN` and the OAuth callback `https://<host>/api/auth/callback/github` move with the same constant on the harness side.

2. **Implement `ejm-cli package migrate-credentials --old-clients-stopped`.** The harness error and `docs/packages.md` already tell the user to run that command, and `migrate_registry_token` is the public handoff for a legacy file whose origin matches the selected registry. Renaming the command, or deleting the sentence, would leave that handoff with no CLI entry. The flag is required. Without it the process exits non-zero and does not open either credential store. The command makes no HTTP request and never prints a token.

   The same command is the retired-origin discard. `package logout --retired` would sound like the remote revoke `package logout` already performs, and the user who is blocked never sees that name. Automatic deletion on startup would skip the "older clients have stopped" confirmation the vault uses, because an older client does not honor the vault lock.

   Current `migrate_registry_token(root, new_origin, true)` returns an origin mismatch for a meetgerry legacy file and leaves the user blocked. The harness change needs one public local operation the CLI can call after the flag is set:

   - It takes the selected registry origin and the confirmation flag.
   - It performs no HTTP and returns no token.
   - When the legacy file's origin is the selected origin, it keeps today's import behavior.
   - When the legacy file's origin is the retired host, it removes that file with the existing privacy checks and deletes the vault record for the retired origin. The migration marker stays, so a later leftover file cannot restore the bearer.
   - It does not copy the retired bearer into the new origin.
   - When the legacy file is already gone and a vault record for the retired origin remains, it deletes that record the same way.
   - Any other legacy origin is left in place and reported. The CLI prints that error.

   The CLI does not open `registry-session.json`. Mode, owner, link count, and size checks stay in the harness.

   Retirement must not be enforced inside the parser that builds storage identity. Today `identity()` calls `parse_origin`. If that function rejects `https://harness.meetgerry.ai`, local delete can no longer name the stored record. `RegistryClient::new` rejects the retired origin for every network client. Storage delete for that origin stays possible inside the harness operation above. The CLI never passes the retired host as `--registry`.

   A second public query reports whether a retired-origin vault record or legacy file exists, without returning a token or contacting a host. `doctor` uses it.

3. **Logout removes the vault record and does not touch the legacy path.** For the origin in effect, `package logout` validates the origin, loads that origin's token, asks that origin to revoke it, and calls `delete_registry_token` for that origin. Local deletion still happens when remote revocation fails, and the error says the local session was removed and remote revocation failed. A missing `registry-session.json` is not an error. There is no `remove_file` of that path. No session means a non-zero exit that tells the user to sign in, with no file-not-found text. Logout never runs against the retired host, so the old bearer is not posted to `harness.meetgerry.ai`.

4. **`--registry` is global, and only package commands and `doctor` read it.** The spec requires `doctor` to say when the origin came from `--registry`. `doctor` is a top-level command, so a flag that exists only under `package` cannot reach it. The flag follows `--base-url`: global, environment `EJM_CLI_REGISTRY_URL`, flag wins, then env, then `DEFAULT_REGISTRY`. An empty env var is not an override. Model commands ignore it. A bad value does not fail `ejm-cli run`. Help text says the flag selects the package registry for `package` and `doctor`, and that model endpoints are unchanged. This resolves the proposal's package-only recommendation.

5. **Validation is the harness check, and it runs first.** Every `package` subcommand and `doctor` resolve the origin before session load, registry HTTP, or other package work. `RegistryClient::new` is the check. The CLI does not copy the loopback set or the retired-host comparison. Failure exits non-zero, names `--registry` or `EJM_CLI_REGISTRY_URL` according to which one supplied the value, and includes the harness error. For the retired host that error says the meetgerry registry is retired and names the current default. No flag disables the rules. When the origin is not the default, registry commands print it before any request. `doctor` prints `Registry: <origin> (default|EJM_CLI_REGISTRY_URL|--registry)`. A rejected override is printed as rejected, the rest of the doctor report still prints, and the process exits non-zero. `doctor` makes no registry request.

6. **Sessions stay origin-scoped.** `login` saves with `save_registry_token` for the origin in effect. `whoami`, `publisher`, `publish`, and `logout` load only that origin. A missing session exits non-zero, says there is no session for that origin, and sends no token. The CLI never accepts a token as an argument and never prints one.

7. **The pin moves only with protected harness main.** This planning change does not edit `.github/harness-revision` or `Cargo.lock`. Implementation sets the pin to the full lowercase SHA of the harness change after that commit is on protected `ejmlabs/ejm-harness` main. The selector accepts a commit only when it equals or is an ancestor of that main. A feature-branch SHA will fail CI. `Cargo.toml` keeps the `../ejm-harness` path dependencies. Refresh `Cargo.lock` from a sibling checkout of the pinned commit, then `cargo check --locked --all-targets -p ejm-cli`. The current pin cannot build this design: it does not export `delete_registry_token` or `migrate_registry_token`.

8. **Tests stay local.** No test contacts `harness.meetgerry.ai`, the new host, or a paid provider. Argument and doctor cases run the real binary. Cases that need a registry use a loopback HTTP fixture in `crates/ejm-cli/tests/cli.rs` and record method, path, and whether `Authorization` was present. Session cases use the OS key service. Linux CI already prepares Secret Service for persistent-account fixtures. Tests do not skip when it is missing, and the CLI does not grow a test-only key store. A legacy fixture is a private owner-only regular file in the temp package root, the same shape the harness reader accepts. Assert the fixture token never appears on stdout or stderr. `package publish` is not called. The retired URL may appear in tests as a rejected value. It must not appear as the live default in docs, `--help`, or default `doctor` output.

## Risks / Trade-offs

- The harness PR can choose a different function name. The CLI calls whatever public function implements decision 2. If that PR rejects the retired origin inside `parse_origin`, or leaves the origin-mismatch error in place, stop and update this change before writing CLI code.
- `EJM_CLI_REGISTRY_URL` can point package commands at another registry. The process prints a non-default origin and `doctor` shows the source. Digests still come from that origin, and install stays disabled until `--trust`.
- Discard deletes the only local copy of a meetgerry session. An older CLI cannot reconstruct `registry-session.json` afterward. That is the point of retiring the host. Document it in the upgrade note.
- A vault-only retired session does not block `package login` on the new origin, because the storage key changed. `doctor` is what tells that user to run the discard command.
- `harness.ejmlabs.com` is already the harness product page in this repo's site. Selecting it as the registry host collides with that page. The CLI plan does not retarget those links.

## Migration Plan

After the new CLI is installed, registry commands use the new default. Sessions saved for `https://harness.meetgerry.ai/` are not sent anywhere.

- A user with a legacy `registry-session.json`, or with a vault record for the retired origin, stops older clients and runs `ejm-cli package migrate-credentials --old-clients-stopped`. That removes the retired material locally.
- The user then runs `ejm-cli package login` and approves a new session for the new origin.
- `package logout` on the new origin revokes and deletes only that new session.

There is no remote revoke of the old bearer and no copy of it onto the new host. Rolling the CLI back to the pinned plaintext build does not restore a discarded legacy file. Users who never signed in to the registry have nothing to migrate.

The harness deploys the new host, the auth Worker origin, and the OAuth callback together. This CLI release depends on that commit being on protected main so the pin can move. Shipping the CLI first would still point CI at the old plaintext pin.

## Open Questions

- Which host Everett picks. The candidates are `harness.ejmlabs.com` and `registry.ejmlabs.com`. Implementation quotes `DEFAULT_REGISTRY` after that pick. The Product Manager recommends `registry.ejmlabs.com`. The CLI site links to `harness.ejmlabs.com` are the harness product page and stay as they are in this change. If Everett picks that same host for the registry, the product page and the registry Worker need a separate decision before either side assumes the host.
