# Tasks

Harness pull requests [#18](https://github.com/ejmlabs/ejm-harness/pull/18) and [#19](https://github.com/ejmlabs/ejm-harness/pull/19) are merged. `.github/harness-revision` moves to the harness main commit that contains them.

## 1. Registry origin

- [x] 1.1 Pass `https://registry.ejmlabs.com` from the CLI. Add global `--registry` and `EJM_CLI_REGISTRY_URL` with flag, then env, then that default. Reject `meetgerry.ai` and its subdomains, non-plain origins, non-https hosts, `0.0.0.0`, and LAN addresses. Accept loopback `localhost`, `127.0.0.0/8`, and `::1` through `host_is_loopback` in `args.rs`. Do not change `api_base_url()`. Verify with the `args` unit tests.
- [x] 1.2 Package commands validate that choice before session load or HTTP. A rejection exits non-zero, names `--registry` or `EJM_CLI_REGISTRY_URL`, and does not fall back. `run` ignores an invalid value. Verify with `cargo test -p ejm-cli --test cli`.
- [x] 1.3 Print a non-default origin before a package command makes a request. `doctor` prints the origin and `default`, `EJM_CLI_REGISTRY_URL`, or `--registry`, marks a rejected override, still prints the rest of the report, makes no registry connection, and exits non-zero.

## 2. Sessions

- [x] 2.1 `package logout` revokes and removes only the session whose stored origin is the origin in effect. A missing file, or a file for another origin, does not fail and sends nothing. Remote revocation failure still removes the local file and says `local session removed; remote revocation failed`.
- [x] 2.2 `whoami`, `publisher`, and `publish` load only the session for the origin in effect. A different origin reports that there is no session and sends no token.
- [x] 2.3 Add no discard command. Nothing shipped against `meetgerry.ai`, so no user holds a retired session, and the harness refuses that origin in save, load and migrate.

## 3. Docs

- [x] 3.1 Update the registry section of `docs/ejm-cli.md`: default URL, override, loopback rule, session scope and logout. Remove the sentence that says commands use `https://harness.meetgerry.ai`. Describe `registry-session.json` as this pin's session file, and say a newer vault record is not deleted here.
- [x] 3.2 Mention the default and the override in `README.md` and `CHANGELOG.md`. Leave the public site unchanged: it does not list package registry commands, and it must not claim the registry service is deployed.
- [x] 3.3 Search `meetgerry` and `gerry-registry`. Keep `gerry.package.json`, the contributing "Gerry platform" wording, the `gerry-*` worker warning, `.gitignore` `registry-session.json`, and the `harness.ejmlabs.com` product links.

## 4. Verification

- [x] 4.1 Run `cargo test -p ejm-cli`, `./scripts/check`, and `openspec validate move-registry-to-ejmlabs-domain --strict`. No test contacts `registry.ejmlabs.com`, `meetgerry.ai`, or a paid provider.
- [x] 4.2 Keep `.github/harness-revision` at `42e4f9cf33bd3525aec344cfdffabc251271f782` (vault credentials; still no discard API or new default). Record that `6c90a0d` exports `DISCARD_RETIRED_REGISTRY_COMMAND` and `discard_retired_registry_sessions`, that this build cannot import them, and that harness #18 and #19 are not on main.

## Workflow follow-up

- [x] Move `.github/harness-revision` to the harness main commit that contains the merged pull requests #18 and #19.
- Sync the delta into `openspec/specs/` and archive this change only after Everett asks. This implementation does not archive.

## 5. Proxy compatibility follow-up

- [x] 5.1 Add the narrow older-pin configuration adapter that rejects HTTP loopback registry access with any nonempty proxy variable, even with `NO_PROXY`, before client construction or credential lookup. Reuse provider endpoint code only if it exists on this branch. Preserve pure origin resolution, HTTPS and model commands.
- [x] 5.2 Add isolated real-binary fixtures for each proxy variable form, flag and environment origin selection, doctor rejection, unchanged synthetic session, and no connection to passive registry/proxy listeners. Keep bearer and proxy values out of output and add no HTTP parser or server.
- [x] 5.3 Update help, README, registry reference and changelog with the temporary restriction and recovery. Record harness transport ownership, public API gap, fixture checks, integration order and removal condition. Preserve the full immutable pin and human CI gates.
- [x] 5.4 Run focused checks and the normal full CLI gate against the exact pinned harness, with Cargo jobs at most two, incremental compilation disabled, and the approved shared audit target. Keep session bytes out of assertion diagnostics. Make the existing pinned-origin fixture portable with `http://127.0.0.2:0`, preserving the precise constructor rejection without requiring a host alias or skipping the test. Record exact sources, commands, results, and every remaining platform or release gate in `proxy-verification.md`. Keep the immutable pin, locks, product behavior, and separate human gates unchanged. The Darwin arm64 gate originally passed against harness `4a751c6a3f24d7d143140f79ff53517fb93e3531`; after merging main the pin is `42e4f9cf33bd3525aec344cfdffabc251271f782`, which still lacks loopback `no_proxy()` so the CLI proxy guard remains required.

## 6. Adopt the harness registry change

- [x] 6.1 Pass `Some(registry)` to `install_source`, which now takes an optional client.
- [x] 6.2 Take `PUBLIC_REGISTRY_ORIGIN` from the harness. Remove the local discard command and session notice; the harness refuses retired origins, and nothing shipped against them.
- [x] 6.3 Remove the proxy adapter and its fixtures; `doctor` accepts an HTTP loopback registry with proxies set and connects nowhere.
- [x] 6.4 Describe `ejm.package.json` as the manifest name, and update the README, CLI reference and changelog.
- [x] 6.5 Run `cargo test -p ejm-cli`, strict Clippy and `./scripts/check` against the new harness pin.
