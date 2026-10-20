# Design

## Context

Security finding L2 (baseline audit, 2026-10-19) says a provider API key can be sent in cleartext because a base URL of `http://` is accepted for any host.

Observed on this CLI at `c8e2239`:

- `crates/ejm-cli/src/args.rs` defines global `--base-url` / `EJM_CLI_BASE_URL` (`Args::base_url`, lines 78-84). `api_base_url()` (lines 280-288) returns that string or a fixed default: `https://api.openai.com/v1`, or `https://api.anthropic.com/v1` when `--provider anthropic`.
- `crates/ejm-cli/src/main.rs` `provider()` (lines 203-220) reads the API key, then passes `api_base_url()` straight into `AnthropicConfig.base_url` or `OpenAiConfig.base_url`.
- `responses` and `chatgpt` never use that URL. `responses_credentials()` (lines 242-245) already rejects any set `--base-url` or `EJM_CLI_BASE_URL` before credentials are read. The integration test `native_model_discovery_rejects_endpoint_overrides_before_accessing_credentials` locks that message.
- `doctor()` (lines 420-427) prints the OpenAI-compatible or Anthropic URL and does not construct a provider. `sessions`, package commands, login, accounts, and logout do not send it. `--replay` and `--replay-file` return a `ReplayProvider` before the URL is read. `--package-provider` selects an extension provider and does not pass `--base-url`.
- Process errors use exit code 1 and the prefix `ejm-cli: ` (`main`, lines 56-62). Clap parse failures exit 2 through clap. There is no separate status for configuration errors.
- The `url` crate is already a direct dependency and is the parser `auth.rs` uses for authorization URLs. No new crate is required.

Observed on ejm-harness:

- CI pin `.github/harness-revision` is `4a751c6a3f24d7d143140f79ff53517fb93e3531`. Local builds use the path dependency `../ejm-harness/crates/ejm-harness`.
- At that pin, and on audited harness main `8d3dc39`, private `endpoint()` in `crates/ejm-harness/src/provider.rs` accepts scheme `http` or `https` for any host. It already rejects embedded userinfo, query, and fragment. `OpenAiProvider::stream` sends `bearer_auth(api_key)`. `AnthropicProvider::stream` sends `x-api-key`. The HTTP client uses `redirect::Policy::none()`.
- `endpoint()` is private. The public constructors take `base_url: String` and call `endpoint()` only after the key is already inside the config value. A harness error still drops that config before a request is sent, but only when `endpoint()` rejects the URL. Today a remote `http://` URL is not rejected.
- Harness pull request [#18](https://github.com/ejmlabs/ejm-harness/pull/18), branch `openspec/require-https-provider-endpoints`, is the sibling design. It keeps validation inside private `endpoint()`, uses this same loopback predicate, and adds no public type. Its stable rejection text is `provider URL must use HTTPS, or HTTP on a loopback host`. Parse failure stays `invalid provider base URL`. That design also asks the CLI not to re-parse the URL. The CLI spec in this change still requires a local check. See Cross-repo dependency.

Observed on ejm-ide at `0309cb7`:

- `src-tauri/src/providers/driver.rs` `validate_endpoint` (lines 433-455) is the behavior to match. It parses with `url::Url`, requires a host, and allows `https`, or `http` when the host is loopback. Loopback is an ASCII case-insensitive domain `localhost`, an IPv4 address for which `is_loopback()` is true (`127.0.0.0/8`), or an IPv6 address for which `is_loopback()` is true (`::1`). It does not resolve DNS. The profile test (profiles.rs lines 520-539) allows `http://localhost`, `http://127.0.0.1`, and `http://[::1]`, and rejects `http://api.example.test`.
- The audit names the hosts `127.0.0.1`, `::1`, and `localhost`. The IDE predicate is wider for IPv4: the whole `127.0.0.0/8` block. This design follows the IDE predicate.
- ejm-ide has no flag or environment variable that re-enables remote `http://`.

`docs/ejm-cli.md` and the README show only `https://api.deepseek.com/v1`. The public page row for OpenAI-compatible access says "Any Chat Completions endpoint" (`site/dist/index.html`). Nothing in the docs promises remote `http://`.

`proposal.md` and `specs/provider-endpoint-security/spec.md` are the product manager's text. This design follows them. `.openspec.yaml` keeps `created: 2026-10-20`, the day this change was opened on the branch. The product manager draft of that file had the same schema and `created: 2026-10-19`.

## Goals / Non-Goals

**Goals:**

- Stop ejm-cli from accepting a non-loopback `http://` provider base URL on the OpenAI-compatible and Anthropic paths, before the API key is read and before `OpenAiProvider` or `AnthropicProvider` is constructed.
- Use the loopback predicate in the spec, which is the same predicate as ejm-ide `validate_endpoint` and harness PR 18.
- On `run` and the TUI, report a stable configuration error on stderr with exit code 1. The error names `--base-url` / `EJM_CLI_BASE_URL`, states the HTTPS rule, and does not include the API key or the raw URL.
- On `doctor`, print an endpoint line that includes the disallowed URL, marks it rejected, and states that HTTPS is required except for loopback hosts, then exit 1. Do not print that URL as a usable endpoint, and do not print the API key.
- Keep the existing `responses` / `chatgpt` fixed-endpoint error ahead of this rule.
- Document the rule in `--help`, `docs/ejm-cli.md`, the README, and the site row that describes `EJM_CLI_BASE_URL`.
- Leave the harness pin and lockfile unchanged, and state how the sibling harness change is adopted later.

**Non-Goals:**

- Editing `../ejm-harness` or `../ejm-ide`, exporting a new harness API, or moving `.github/harness-revision`.
- An escape hatch for remote cleartext HTTP.
- Changing ChatGPT sign-in. Its loopback redirect (`http://127.0.0.1`) is not a provider base URL.
- Changing the hard-coded `https` package registry, package-provider endpoints, or SDK clients.
- Replacing the harness checks for embedded userinfo, query, and fragment. Those still run inside `endpoint()` when a credentialed provider is constructed.
- Adding a new URL length limit. ejm-ide rejects values longer than 2048 bytes. This CLI change does not add that cap, so a long `https://` URL that the current harness accepts keeps working.
- Rewriting, canonicalizing, or adding a trailing slash to the stored URL. Validation accepts or rejects the effective string. The harness still appends `/chat/completions` or `/messages`.
- Making `doctor` reject a set base URL for `responses` and `chatgpt`. Those modes already ignore the URL and print the official endpoint label. `models` still rejects it.

## Decisions

### 1. Validate in the CLI before the key is read, and keep the harness as the sender-side control

**Choice:** `Args::api_base_url()` becomes fallible and enforces the rule. `provider()` calls it before `required_api_key()` and before either provider constructor. A failure there uses the existing `Err` path: stderr `ejm-cli: ...`, exit code 1, and the message does not contain the URL or the API key.

`doctor()` calls the same check for OpenAI-compatible and Anthropic providers. An accepted URL is printed as the usable endpoint. A URL rejected by the HTTPS rule still produces an `Endpoint:` line. That line contains the configured URL, marks it rejected, and carries the same HTTPS-required explanation. `doctor` then exits 1. The API key is not printed. An invalid URL exits 1 with the invalid-URL error. The spec's doctor scenario is the HTTPS-rule case, and the endpoint line is what satisfies it.

The harness change remains necessary. It is the only check that runs for every caller that can put a key on a request. This CLI check is the early, user-facing gate for `--base-url` and `EJM_CLI_BASE_URL`. It does not replace `endpoint()`.

**Why this shape:**

- The dangerous send is `bearer_auth` / `x-api-key` inside the harness. A CLI-only check would leave every other harness caller exposed. A harness-only check, on the current pin, does not exist: `endpoint()` still allows remote `http://`.
- On today's code the key is loaded before `OpenAiProvider::new` / `AnthropicProvider::new`. Even after the harness rejects remote `http://`, that rejection happens after `required_api_key()`, and `doctor` would still print the cleartext URL as the configured endpoint.
- Making `api_base_url()` itself return `Result<&str>` means the compiler sees both production call sites. A second unchecked accessor would be easy to misuse. The method returns the same string the caller supplied or the existing default. It does not allocate a rewritten URL.
- The check runs only when that method is called. Replay, replay files, session listing, package commands, login, accounts, logout, and `--package-provider` do not call it, so a hostile or stale `EJM_CLI_BASE_URL` does not break commands that never send the URL.

**Alternatives:**

- Clap `value_parser` on `base_url`. This fails while arguments are parsed, including for `sessions`, package commands, login, and `--replay`, whenever the environment variable is set. Clap then exits 2 via its own process exit, with clap's usage format, before the `ejm-cli:` error path. A cleartext URL on `responses` or `chatgpt` would also surface this parser error before the existing "fixed official endpoint" message.
- Rely on the harness and do nothing in the CLI until the pin moves. `doctor` would keep displaying a remote `http://` endpoint, the missing-key error could win over the URL error depending on order, and the CLI would stay vulnerable for as long as the pin stays on `4a751c6a`.
- A new public harness helper that the CLI calls. Harness PR 18 does not add one. It keeps `endpoint()` private and asks the CLI to pass the string through. This change still parses locally, because the CLI spec requires a flag-named error before the key is read. The predicate is copied from that PR so the two checks allow and reject the same hosts.

### 2. Match ejm-ide's loopback predicate, using the `url` crate already in the CLI

**Choice:** Parse with `url::Url::parse`. Do not resolve DNS and do not open a socket. The parser lowercases the scheme, so `HTTPS://` and `HTTP://` follow the same rules as lowercase.

- If the value does not parse, the parsed URL has no host, the value has no scheme, or the scheme is anything other than `http` or `https`, fail with: `invalid provider base URL. Change --base-url or EJM_CLI_BASE_URL.`
- If the scheme is `https` and a host is present, accept it.
- If the scheme is `http` and the host is loopback, accept it.
- If the scheme is `http` and the host is not loopback, fail with: `provider base URL must use https, or http on a loopback host (localhost, 127.0.0.0/8, or ::1). Change --base-url or EJM_CLI_BASE_URL.`

`ftp`, `file`, `ws`, and a value with no scheme use the invalid-URL sentence. The spec groups those four as one scenario. A remote `http://` URL uses the HTTPS sentence, which names `--base-url` / `EJM_CLI_BASE_URL` and states the loopback exception. The stderr text for that failure does not include the raw URL. The `doctor` endpoint line does include it, marked rejected.

Loopback, taken from `url::Host` after parsing:

- `Host::Domain`: ASCII case-insensitive equality with `localhost` only.
- `Host::Ipv4`: `Ipv4Addr::is_loopback()` (`127.0.0.0/8`).
- `Host::Ipv6`: `Ipv6Addr::is_loopback()` (`::1` only).

The stderr error does not contain the supplied URL, its userinfo, the query, the fragment, or the API key. `render::safe_text` still wraps that line and strips terminal control sequences. The `doctor` endpoint line is the exception the spec requires: it shows the configured URL and marks it rejected. If that URL has userinfo, the printed line omits the userinfo and still shows the host. The spec scenario `http://api.example.com/v1` has no userinfo, so the line shows that whole URL.

`responses` and `chatgpt` keep today's check first, in `responses_credentials()`. An `http://` override there still reports `native Responses uses its fixed official endpoint; remove --base-url and EJM_CLI_BASE_URL` and does not report the new sentence.

**Alternatives:**

- Allow only the three literal hosts named in the audit (`127.0.0.1`, `::1`, `localhost`). That would reject `http://127.0.0.2`, which ejm-ide and harness PR 18 allow.
- Treat any hostname that resolves to loopback as loopback. That needs DNS at configuration time, and a resolver can answer differently later. It is the wrong place to decide whether a key may leave the machine.
- Treat IPv4-mapped `::ffff:127.0.0.1` as loopback. `Ipv6Addr::is_loopback()` is false for that address, and ejm-ide rejects it. Allowing it would widen cleartext HTTP beyond the IDE.

### 3. No escape hatch

**Choice:** There is no `--insecure-base-url`, no `EJM_CLI_ALLOW_INSECURE_HTTP`, and no config file switch that turns remote `http://` back on.

Remote cleartext is the finding: a typo or a hostile environment variable can point the key at an `http://` host. An environment-variable bypass uses that same channel. A hidden flag would remain a supported way to send a long-lived key in cleartext, which is the behavior L2 removes. ejm-ide has no bypass.

Local OpenAI-compatible servers (Ollama, llama.cpp, LM Studio, and similar) are the case the exception exists for. They are reached with `http://127.0.0.1`, `http://localhost`, or `http://[::1]`, including a non-default port. A private remote HTTP service can terminate TLS, or sit behind a TLS proxy on one of those loopback hosts. Those are the supported setups.

### 4. Keep the check in a shared helper and leave the harness pin for a follow-up

**Choice:** `crates/ejm-cli/src/endpoint.rs` exports `require_https_or_loopback`. It returns a small error enum and does not name `--base-url`. `Args::api_base_url()` maps that enum to the provider error text. A later `--registry` flag in `move-registry-to-ejmlabs-domain` can call the same function and supply its own message. The helper does not open a socket or resolve DNS.

### 5. Keep the current harness pin in this change

**Choice:** Implementation of this design does not edit `../ejm-harness` and does not point `.github/harness-revision` at a branch. Tests must pass against the current pin `4a751c6a3f24d7d143140f79ff53517fb93e3531`. On that pin, `endpoint()` still accepts remote `http://`, so a test that reaches `OpenAiProvider::new` with `http://api.example.com/v1` does not prove the CLI check. The CLI test has to fail before that constructor and before `required_api_key()`.

The proposal says the CLI and the harness change land together for the coordinated 0.1.0 release. That release needs a harness revision whose `endpoint()` enforces this predicate. When harness PR 18 is on protected main, the pin update uses that full commit and refreshes `Cargo.lock` with the locked, at-most-two-job workflow. If that commit does not exist yet during implementation, the pin stays on `4a751c6a` and the pin update is the follow-up that completes the pair. It is not a second product rule.

After that adoption, keep the CLI's early error. The harness sentence does not name `--base-url` or `EJM_CLI_BASE_URL`, so a CLI that only printed the harness error would miss the spec. The harness check still covers other callers and any future CLI path that skips `api_base_url()`.

Harness PR 18 does not export a public validator. The CLI copies the predicate and does not reach into private harness items. A later public helper can replace the copy.

## Edge cases

Accepted:

- Defaults with no `--base-url` and no `EJM_CLI_BASE_URL`: `https://api.openai.com/v1` and `https://api.anthropic.com/v1`.
- Any `https` URL with a host, any port, and any path, including `https://localhost` and `https://api.deepseek.com/v1`.
- `http://localhost`, `http://LOCALHOST`, `http://127.0.0.1`, `http://127.0.0.2`, and `http://[::1]`, with or without a port and path. `http://127.0.0.1:11434/v1` is the local-model case.
- IPv4 and IPv6 spellings that `url` 2 already classifies as those loopback hosts, including `http://[0:0:0:0:0:0:0:1]/`. The test records the classification this crate version produces for shortened IPv4 forms such as `http://127.1/v1`. Do not add a second parser.

Rejected with the https sentence:

- `http://api.example.com/v1`, `http://example.com:443`, `http://0.0.0.0`, `http://10.1.2.3`, `http://172.16.0.1`, `http://[::]`, `http://[::ffff:127.0.0.1]`, `http://localhost.evil.com/v1`, `http://127.0.0.1.nip.io/v1`, `http://192.168.1.10:11434/v1`, `http://localhost.localdomain`, `http://localhost.`, and any other domain that is not exactly `localhost`.

Rejected with the invalid-URL sentence:

- `ftp://`, `file://`, `ws://`, and a value with no scheme, including `api.example.com/v1`.
- The empty string, a value `url::Url::parse` rejects, and a parsed URL with no host.
- The CLI does not trim. A trailing space that fails parsing uses the invalid-URL sentence.

Left to the harness, after this check has accepted the URL:

- `https://user:secret@host/v1`, and any accepted URL that also has a query or fragment. `endpoint()` already rejects those with `provider URL must not contain credentials, query, or fragment` and does not include the URL in the error. The API key is read before that constructor runs. This design does not move that existing check. The CLI https error for `http://user:secret@example.test/v1` must still omit `secret`, because that URL is rejected here as non-loopback HTTP.
- Redirects. The harness client already uses `redirect::Policy::none()`, so an accepted `https` URL is not followed onto `http`. This change does not weaken that policy.

Commands that must keep working when `EJM_CLI_BASE_URL` is a remote `http://` URL, because they do not call `api_base_url()`:

- `--replay` and `--replay-file`, including `run`.
- `sessions`, package commands, `login`, `accounts`, and `logout`.
- `--package-provider` (the extension provider is selected and the base URL is unused).

`doctor` and a non-replay `run` for OpenAI-compatible or Anthropic providers fail closed on a bad URL even when no API key is set. The user sees the base URL error, not `set EJM_CLI_API_KEY` and not `provider connection failed`. `doctor` also prints the rejected endpoint line for a remote `http://` URL. A planted `EJM_CLI_API_KEY` must not appear in stdout or stderr. No provider connection is opened.

The interactive TUI calls `provider()` before `ui::run`, so a bad URL exits before raw mode and the alternate screen. No terminal-restoration change is required unless a later edit moves the check.

## Compatibility

This is a behavior change for source checkouts that currently point OpenAI-compatible or Anthropic access at a non-loopback `http://` base URL. The next `run` or TUI launch exits 1 with the https sentence. `doctor` exits 1 after an endpoint line that marks that URL rejected. No state directory migration is required. Rollback is reverting the CLI change. The harness pin does not move inside this implementation, so rollback does not involve a dependency downgrade.

Unchanged:

- Every `https://` base URL the current harness accepts, including the documented DeepSeek example.
- Loopback `http://` as defined above.
- Omitted base URL (the https defaults).
- `responses` and `chatgpt`, including their fixed-endpoint error and official `doctor` labels.
- Replay, session listing, package commands, and account commands.
- Exit code 1 for configuration errors, and exit code 2 for clap usage errors. This rule is a configuration error, not a parse error.

The public page, README, reference, and `--help` currently allow a reader to think any Chat Completions URL works. Those four surfaces gain the https rule in the same implementation. The site file is edited in the repo and is not deployed from this change.

Changelog: an Unreleased Fixed entry names the CLI behavior and states that the harness pin is unchanged. It must not say that other ejm-harness callers are protected or that a harness release contains the check.

## Cross-repo dependency

| Item | What this change does |
| --- | --- |
| CLI `Args::api_base_url()` | Enforces the shared predicate before the key is read, with an error that names `--base-url` / `EJM_CLI_BASE_URL` |
| Harness PR 18 `endpoint()` | Private check, same predicate, message `provider URL must use HTTPS, or HTTP on a loopback host`. No new public type. Not on the CLI pin yet |
| `.github/harness-revision` and `Cargo.lock` | Stay on `4a751c6a3f24d7d143140f79ff53517fb93e3531` until that harness commit is on protected main |
| Paired release | The proposal says both changes land together for the coordinated 0.1.0 release. The CLI early error stays after the pin moves |
| ejm-ide | Already enforces the predicate. No IDE edit |

Open question 1 is resolved. The assumption was a private `endpoint()` check, the IDE loopback predicate, and no new type the CLI must call. Harness PR 18 matches that assumption. It also rejects `http://[::ffff:127.0.0.1]`, exact-only `localhost`, and `foo.localhost`.

The same PR rejects a second parser in the CLI and says the harness error should be authoritative. This change does not follow that part. The CLI spec requires validation before the API key is read, and the error must name `--base-url` / `EJM_CLI_BASE_URL`. The harness sentence does not name them. The CLI therefore keeps its own check and uses the harness predicate so the two cannot disagree on which host is loopback.

Harness PR 18 records the break under `## [Unreleased]` and prefers a later minor `0.2.0`, leaving its dated `0.1.0` notes unchanged. The CLI proposal says L2 blocks the coordinated 0.1.0 release. This change does not edit `VERSION` in either repo. Which release number carries both fixes is unresolved.

## Risks / Trade-offs

- Two copies of the predicate can drift. Harness PR 18 keeps its copy private and asked the CLI not to re-parse. The CLI spec requires the local check anyway. The drift bound is the shared predicate, the spec scenarios, and the unit tests for the host classes. A later public helper can collapse the copies.
- Users of a private non-loopback `http://` server lose a path that the binary accepted and the docs did not describe. Loopback HTTP and real TLS remain. No bypass flag is added to soften that.
- `doctor` for `responses` and `chatgpt` still succeeds when a base URL is set, while `models` and `run` fail. That mismatch is existing behavior and stays. Widening `doctor` would change a path that does not send a key.
- Accepting every `https` host still sends the key to whatever host the user or the environment named. TLS does not make a hostile host trustworthy. The finding is cleartext, not endpoint identity.
- `https` URLs with userinfo still reach `required_api_key()` before the harness rejects them. The key is not sent. Closing that order is a separate cleanup.
- A pin bump before harness PR 18 is on protected main would name a commit that does not contain the check, or point CI at a branch. The pin stays on `4a751c6a` until that full SHA exists. The proposal's paired 0.1.0 release is what later moves it.
- Claiming "the key never leaves the process" needs the call order and the tests below. It does not need a packet capture, and this change must not claim one.

## Test strategy

No paid provider calls and no network. Cargo jobs stay at most two. The integration helper in `crates/ejm-cli/tests/cli.rs` always inserts `--replay`, which skips `api_base_url()`. New process tests need their own builder, still with a temporary `--state-dir`, and they clear `EJM_CLI_API_KEY`, `OPENAI_API_KEY`, and `ANTHROPIC_API_KEY`.

Unit tests on `Args` cover the predicate, including `127.0.0.2`, `LOCALHOST`, `http://[::ffff:127.0.0.1]`, and `http://127.1/v1` following `is_loopback()` on the host `url` 2 parses. They also cover every spec URL that does not need a process: the accepted DeepSeek URL, the three loopback URLs, the three lookalike URLs, `ftp`, `file`, `ws`, and a value with no scheme.

Integration tests use a builder that does not force `--replay`, a temporary `--state-dir`, and a cleared key environment unless the scenario plants a key. Exit status for a rejected URL is exactly 1. A clap exit of 2 fails the test. The scenario list in `tasks.md` is the checklist. In particular:

- `doctor` with `EJM_CLI_BASE_URL=http://api.example.com/v1` prints an `Endpoint:` line that contains that URL, marks it rejected, and says HTTPS is required except for loopback hosts. The process exits 1. Stdout does not present the URL as a usable endpoint.
- Accepted `https://api.deepseek.com/v1` from the flag and from the environment is the endpoint `doctor` prints, with no rejection mark. `--provider anthropic` with no base URL prints `https://api.anthropic.com/v1`.
- The three loopback URLs are accepted. `doctor` prints `http://127.0.0.1:1234/v1` as a usable endpoint and does not connect to that port.
- A planted API key is absent from stdout and stderr, and the output does not contain `provider connection failed` or `set EJM_CLI_API_KEY`.
- `--provider chatgpt --base-url https://other.test/v1` still reports the fixed official endpoint. The existing `models` test for `responses` and `chatgpt` stays.

A PTY test is not required while the check stays before `ui::run`.

Implementation verification is `cargo test -p ejm-cli` for the new unit test and `cargo test -p ejm-cli --test cli`, then `./scripts/check`.

## Migration Plan

1. Everett approves this design. `proposal.md` and the delta spec are already on the branch.
2. Implement the CLI tasks. Leave the harness pin on `4a751c6a` until harness PR 18 is on protected main.
3. Tell users, in the changelog and the four doc surfaces, that a non-loopback `http://` base URL now exits 1, and that `doctor` marks that endpoint rejected.
4. When the harness commit is on protected main, adopt that full SHA so the coordinated release in the proposal contains both checks. Keep the CLI error, because the harness sentence does not name the flag or the environment variable.

There is no data migration. Reverting the CLI commit restores the previous acceptance of remote `http://` for as long as the harness pin still allows it.

## Implementation notes

The CLI check is `require_https_or_loopback` in `crates/ejm-cli/src/endpoint.rs`. Provider error text stays in `args.rs`, so a later `--registry` flag can call the helper and choose its own sentence.

Sibling design: [ejm-harness PR 18](https://github.com/ejmlabs/ejm-harness/pull/18). Same loopback predicate, private `endpoint()`, message `provider URL must use HTTPS, or HTTP on a loopback host`. The coordinated release adopts that commit after it is on protected harness main. This change leaves `.github/harness-revision` on `4a751c6a3f24d7d143140f79ff53517fb93e3531`. The CLI early error stays after that pin update. The harness `0.2.0` versus CLI `0.1.0` release-number disagreement stays unresolved.

## Resolved questions

1. **Harness shape.** Resolved against [ejm-harness PR 18](https://github.com/ejmlabs/ejm-harness/pull/18). Private `endpoint()`, the IDE loopback predicate (`localhost`, `127.0.0.0/8`, `::1`), and no new public type. The assumption holds. The CLI still validates locally because the spec requires a flag-named error before the key is read. See Cross-repo dependency.
2. **Spec versus the earlier open points.** The spec confirms the loopback set, forbids an opt-out, and requires the CLI check before credentials. It does not require a clap exit 2. It does not require this implementation to invent a harness SHA. The proposal's paired 0.1.0 release is the pin update in migration step 4.
3. **IPv4-mapped loopback.** `http://[::ffff:127.0.0.1]` is outside the spec's loopback set. Harness PR 18 rejects it. This change rejects it.

## Approved follow-up

Everett approved the CLI check as self-contained. This change does not bump `.github/harness-revision`. After [ejm-harness PR 18](https://github.com/ejmlabs/ejm-harness/pull/18) is on protected harness main, a follow-up adopts that full commit and refreshes `Cargo.lock`. The CLI error stays, because the harness sentence does not name `--base-url` or `EJM_CLI_BASE_URL`.

Commands that never read the provider base URL (replay, session listing, package commands, and account commands) keep working when `EJM_CLI_BASE_URL` is a remote `http` URL. The spec scenarios all use or display a provider endpoint. No scenario fails those commands.
