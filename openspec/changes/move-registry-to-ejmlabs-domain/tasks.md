# Tasks

Harness pull requests [#18](https://github.com/ejmlabs/ejm-harness/pull/18) and [#19](https://github.com/ejmlabs/ejm-harness/pull/19) (`6c90a0d297861b78d83b958697c7e7aed9304409`) are open and are not on harness main. Do not move `.github/harness-revision` in this change. The discard command matches `DISCARD_RETIRED_REGISTRY_COMMAND`.

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
- [x] 4.2 Keep `.github/harness-revision` at `4a751c6a3f24d7d143140f79ff53517fb93e3531`. Record that `6c90a0d` exports `DISCARD_RETIRED_REGISTRY_COMMAND` and `discard_retired_registry_sessions`, that this build cannot import them, and that harness #18 and #19 are not on main.

## Workflow follow-up

- [ ] Move `.github/harness-revision` to the harness main commit that contains the merged pull requests #18 and #19. Do that in the same ejm-cli release as `discard-retired-session`. Do not ship the command alone, and do not publish a release note that tells users to discard until both are in.
- Sync the delta into `openspec/specs/` and archive this change only after Everett asks. This implementation does not archive.
