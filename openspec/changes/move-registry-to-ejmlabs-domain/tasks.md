# Tasks

Harness pull requests [#18](https://github.com/ejmlabs/ejm-harness/pull/18) and [#19](https://github.com/ejmlabs/ejm-harness/pull/19) are merged. `.github/harness-revision` moves to the harness main commit that contains them, in the same change as `discard-retired-session`.

`discard-retired-session` ships in the same ejm-cli release as the harness pin bump to the merged harness #18 and #19 commit. Neither ships alone. No release note tells users to discard until both are in.

## 1. Registry origin

- [x] 1.1 Pass `https://registry.ejmlabs.com` from the CLI. Add global `--registry` and `EJM_CLI_REGISTRY_URL` with flag, then env, then that default. Reject `meetgerry.ai` and its subdomains, non-plain origins, non-https hosts, `0.0.0.0`, and LAN addresses. Accept loopback `localhost`, `127.0.0.0/8`, and `::1` through `host_is_loopback` in `args.rs`. Do not change `api_base_url()`. Verify with the `args` unit tests.
- [x] 1.2 Package commands validate that choice before session load or HTTP. A rejection exits non-zero, names `--registry` or `EJM_CLI_REGISTRY_URL`, and does not fall back. `discard-retired-session` ignores the override. `run` ignores an invalid value. Verify with `cargo test -p ejm-cli --test cli`.
- [x] 1.3 Print a non-default origin before a package command makes a request. `doctor` prints the origin and `default`, `EJM_CLI_REGISTRY_URL`, or `--registry`, marks a rejected override, still prints the rest of the report, makes no registry connection, and exits non-zero. When a retired session file is present, name `ejm-cli package discard-retired-session` and do not print the token.

## 2. Sessions

- [x] 2.1 `package logout` revokes and removes only the session whose stored origin is the origin in effect. A missing file, or a file for another origin, does not fail and sends nothing. Remote revocation failure still removes the local file and says `local session removed; remote revocation failed`.
- [x] 2.2 `whoami`, `publisher`, and `publish` load only the session for the origin in effect. A different origin reports that there is no session and sends no token.
- [x] 2.3 `package discard-retired-session` `lstat`s `registry-session.json` and unlinks that path when the origin is retired or the file cannot be used (symlink, loose mode, malformed JSON, missing origin, or larger than 8192 bytes). It does not follow a symlink or read a symlink target. A directory is not deleted: print its path. A private readable session for another origin stays. Prints no token and opens no connection. Do not call `migrate_registry_token`. A retired override does not block the command. `login`, `logout`, and `whoami` print the path and name this command for those unusable files, and do not send a request.
- [x] 2.4 `package login` exits before the device flow when that retired file is present, prints `ejm-cli package discard-retired-session`, and sends nothing. After discard, login against a loopback fixture reaches `api/auth/device/code`.

## 3. Docs

- [x] 3.1 Update the registry section of `docs/ejm-cli.md`: default URL, override, loopback rule, session scope, logout, and `discard-retired-session`. Remove the sentence that says commands use `https://harness.meetgerry.ai`. Describe `registry-session.json` as this pin's session file, and say a newer vault record is not deleted here.
- [x] 3.2 Mention the default and the override in `README.md` and `CHANGELOG.md`. Leave the public site unchanged: it does not list package registry commands, and it must not claim the registry service is deployed.
- [x] 3.3 Search `meetgerry` and `gerry-registry`. Keep `gerry.package.json`, the contributing "Gerry platform" wording, the `gerry-*` worker warning, `.gitignore` `registry-session.json`, and the `harness.ejmlabs.com` product links.

## 4. Verification

- [x] 4.1 Run `cargo test -p ejm-cli`, `./scripts/check`, and `openspec validate move-registry-to-ejmlabs-domain --strict`. No test contacts `registry.ejmlabs.com`, `meetgerry.ai`, or a paid provider.
- [x] 4.2 Keep `.github/harness-revision` at `42e4f9cf33bd3525aec344cfdffabc251271f782` (vault credentials; still no discard API or new default). Record that `6c90a0d` exports `DISCARD_RETIRED_REGISTRY_COMMAND` and `discard_retired_registry_sessions`, that this build cannot import them, and that harness #18 and #19 are not on main.

## Workflow follow-up

- [x] Move `.github/harness-revision` to the harness main commit that contains the merged pull requests #18 and #19. Do that in the same ejm-cli release as `discard-retired-session`. Do not ship the command alone, and do not publish a release note that tells users to discard until both are in.
- Sync the delta into `openspec/specs/` and archive this change only after Everett asks. This implementation does not archive.

## 5. Proxy compatibility follow-up

- [x] 5.1 Add the narrow older-pin configuration adapter that rejects HTTP loopback registry access with any nonempty proxy variable, even with `NO_PROXY`, before client construction or credential lookup. Reuse provider endpoint code only if it exists on this branch. Preserve pure origin resolution, HTTPS, model commands, and local discard.
- [x] 5.2 Add isolated real-binary fixtures for each proxy variable form, flag and environment origin selection, doctor rejection, unchanged synthetic session, and no connection to passive registry/proxy listeners. Keep bearer and proxy values out of output and add no HTTP parser or server.
- [x] 5.3 Update help, README, registry reference and changelog with the temporary restriction and recovery. Record harness transport ownership, public API gap, fixture checks, integration order and removal condition. Preserve the full immutable pin and human CI gates.
- [x] 5.4 Run focused checks and the normal full CLI gate against the exact pinned harness, with Cargo jobs at most two, incremental compilation disabled, and the approved shared audit target. Keep session bytes out of assertion diagnostics. Make the existing pinned-origin fixture portable with `http://127.0.0.2:0`, preserving the precise constructor rejection without requiring a host alias or skipping the test. Record exact sources, commands, results, and every remaining platform or release gate in `proxy-verification.md`. Keep the immutable pin, locks, product behavior, and separate human gates unchanged. The Darwin arm64 gate originally passed against harness `4a751c6a3f24d7d143140f79ff53517fb93e3531`; after merging main the pin is `42e4f9cf33bd3525aec344cfdffabc251271f782`, which still lacks loopback `no_proxy()` so the CLI proxy guard remains required.

## 6. Adopt the harness registry change

- [x] 6.1 Pass `Some(registry)` to `install_source`, which now takes an optional client.
- [x] 6.2 Take `PUBLIC_REGISTRY_ORIGIN` and `DISCARD_RETIRED_REGISTRY_COMMAND` from the harness. Replace the local discard and session notice with `force_discard_legacy_registry_session`, `discard_retired_registry_sessions` and `retired_registry_session_notice`; keep only the CLI's message for a directory at the session path. The harness unlinks relative to the package root handle (ejmlabs/ejm-cli#19).
- [x] 6.3 Remove the proxy adapter and its fixtures; `doctor` accepts an HTTP loopback registry with proxies set and connects nowhere.
- [x] 6.4 Describe `ejm.package.json` as the manifest name, and update the README, CLI reference and changelog.
- [x] 6.5 Run `cargo test -p ejm-cli`, strict Clippy and `./scripts/check` against the new harness pin.
