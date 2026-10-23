# ejm-cli

The `ejm-cli` binary is the reference client for the public Rust ejm-harness API. The CLI links ejm-harness directly and uses the same tool registration and session APIs as community applications. Model responses require a configured provider. Setup and saved-session inspection need no provider credentials.

## Start

Build from sibling source checkouts with Rust 1.99.1, as shown in the [quick start](../README.md#quick-start). The examples below use `ejm-cli` from your `PATH`, except for the source-tree `cargo run` examples.

```sh
cargo run -p ejm-cli --
cargo run -p ejm-cli -- --help
```

Use `/login` in the TUI to choose a provider, or configure API access below. Without a connection, submitted prompts remain in the composer with setup guidance. Plain `run` fails before inference when configuration is missing. The shipped CLI has no fake-response mode. Deterministic response fixtures belong to the test executable and local provider tests.

For a hosted OpenAI compatible API, set the key through your shell's secret handling:

```sh
export EJM_CLI_MODEL=deepseek-chat
export EJM_CLI_BASE_URL=https://api.deepseek.com/v1
# Set EJM_CLI_API_KEY without putting its value in shell history.
ejm-cli
```

The provider key comes from `EJM_CLI_API_KEY`, falling back to `OPENAI_API_KEY`. `--api-key-env NAME` selects another variable. There is no API key argument. `--model` and `--base-url` override environment settings. The default `--provider openai` uses the OpenAI compatible Chat Completions interface and defaults to `gpt-4.1-mini`.

Read tools are enabled by default. `--allow-write` registers file write tools. `--allow-shell` registers shell execution as the current user. Grants stay visible in the TUI header and remain fixed for the process. Native execution is not a security sandbox. `ejm-cli doctor` prints the executor's actual capability report.

## Provider accounts and models

ChatGPT account access uses the shared Rust `ejm-harness-auth` module and native Responses provider:

```sh
ejm-cli login chatgpt
ejm-cli accounts
ejm-cli --provider chatgpt models
ejm-cli --provider chatgpt
ejm-cli --provider chatgpt --model MODEL --reasoning-effort high
```

Sign-in binds an ephemeral loopback listener before opening the browser. The command stays open until sign-in completes, times out or is cancelled with Ctrl+C. If the provider requires a second authorization step, ejm-cli retries once with fresh state and PKCE on the same listener. `--no-browser` prints a checked sign-in URL for manual use. Do not share that URL. Authorization codes and bearer tokens are never command arguments, log messages or TUI content.

```sh
ejm-cli login chatgpt --no-browser
ejm-cli accounts --json
ejm-cli login chatgpt --account REGISTRATION
ejm-cli --provider chatgpt --account REGISTRATION models --json
ejm-cli --provider chatgpt --account REGISTRATION
ejm-cli logout REGISTRATION
```

An account selector accepts a full registration ID or a unique prefix. If several ChatGPT accounts are saved, choose one explicitly. `accounts` reads cached ChatGPT metadata and asks the official Claude client for its safe authentication status. An unavailable provider does not prevent listing the other provider. A locked, missing-key or invalid ChatGPT vault shows recovery guidance and still permits Claude status and selection. Reading accounts before any provider state exists does not create a vault or request a native key. `models` retrieves the selected ChatGPT account's current model catalog. When `--model` and `EJM_CLI_MODEL` are absent, ChatGPT selects the first model from that catalog. ChatGPT logout clears the saved credentials and attempts provider revocation, reporting when revocation is unconfirmed.

`accounts --json` returns an array. ChatGPT registration entries keep their existing fields. If its vault cannot be read, a ChatGPT entry has `provider: "chatgpt"`, `state: "unavailable"` and a safe recovery `message`. This status does not mean signed out. The appended Claude entry has `provider: "claude"` and `state: "available"` or `"unavailable"`. Available entries include safe `loggedIn`, `authMethod`, `email` and `subscriptionType` metadata from the official client. Raw vault errors and credential bytes are omitted.

ChatGPT credentials live in the harness encrypted vault under the configured state directory's `auth` subdirectory. macOS Keychain, Windows Credential Manager or an unlocked Linux Secret Service protects the vault key. Files retain native access controls, atomic replacement and cross-process locking. The account metadata, relay tokens and relay endpoint keys share this physical vault through separate product, purpose and origin/account identifiers. Package-registry sessions use the same vault contract under the package state root and remain a separate authority. None of these credentials can authorize another service.

The provider store imports earlier private-file state while holding its existing lock, verifies the encrypted destination and replaces the old credential payload with a version barrier. Stop older clients before upgrading and do not downgrade a migrated state directory. If the native service is locked, denied or unavailable, unlock it and retry. If an encrypted vault remains but its original key is missing, restore that key and vault together; the CLI must not generate a replacement key. Corrupt state fails closed. Keep the entire state directory private and back up its encryption key through the OS's supported recovery process. The official Claude client keeps its own credential store.

Claude subscription access uses the installed official Claude Code client:

```sh
ejm-cli login claude
ejm-cli --provider claude
ejm-cli --provider claude --model opus
ejm-cli logout claude
```

Install Claude Code through its [official setup guide](https://code.claude.com/docs/en/setup). `login claude` gives that client the terminal for its own subscription sign-in, then checks its authentication status. In the TUI, successful sign-in selects that connection. An API-key or gateway identity is rejected for this provider. Claude Code owns its saved identity and credentials; ejm-cli does not copy or accept subscription tokens. `accounts` reports whether the client is unavailable, signed out, or signed in through a subscription. Claude logout delegates to the official client and clears an active Claude connection.

`--provider claude` defaults to the `sonnet` model alias. Use `--model` or `EJM_CLI_MODEL` for another model supported by Claude Code. `--claude-path PATH` or `EJM_CLI_CLAUDE_PATH` selects the executable. Without an override, the shared harness resolver probes an executable `~/.local/bin/claude` on Unix or the native home directory's `.local/bin/claude.exe` on Windows, then native executables on `PATH`. Native Windows automatic discovery accepts `.exe`, not `.cmd` or `.bat` shims. Relative explicit paths with directory components resolve against the launch directory. An invalid explicit selection fails without silently running another client. This provider does not accept `--account`, API endpoint overrides, or API credentials. Remove incompatible provider settings before selecting it. `--no-browser` applies only to ChatGPT sign-in.

[//]: # "FIXME: live Claude subscription sign-in and inference are covered only by offline fixtures on macOS arm64, never against the real service."

Claude replies arrive after the full structured response is validated; this provider does not stream tokens. The client proposes tool calls, and ejm-harness validates and executes allowed tools through its shared executor. Built-in client tools, MCP and persistent client sessions are disabled. Managed Claude Code policies and hooks may still apply. Native execution is not a sandbox. Offline fixtures test this integration on macOS arm64. Live subscription sign-in and inference are not yet covered by those tests. See the [harness Claude integration contract](https://github.com/ejmlabs/ejm-harness/blob/main/docs/claude-code.md) for process bounds and client limits.

Native OpenAI Responses access with an API key uses:

```sh
# Set OPENAI_API_KEY or EJM_CLI_API_KEY through your shell's secret handling.
ejm-cli --provider responses models --json
ejm-cli --provider responses --model MODEL
```

The `responses` and `chatgpt` providers use fixed official endpoints. Remove `--base-url` and `EJM_CLI_BASE_URL` when selecting either. `--reasoning-effort` accepts `none`, `minimal`, `low`, `medium`, `high`, `xhigh`, `max` and `ultra`; the chosen model must support the requested level. `--account` applies only to ChatGPT access.

Anthropic API access uses the native Anthropic provider and requires an explicit model:

```sh
# Set ANTHROPIC_API_KEY or EJM_CLI_API_KEY through your shell's secret handling.
ejm-cli --provider anthropic --model MODEL
ejm-cli login anthropic
```

`login anthropic` shows Anthropic API-key setup guidance. API-key access falls back to `ANTHROPIC_API_KEY` for this provider. Use `login claude` for official-client subscription sign-in.

## Interaction

Type `/` at the start of the composer to browse built-in commands and granted
package commands. Continue typing to filter names, use Up or Down to choose, and
press Tab to complete without running the command. Escape closes the list and
keeps the draft. Enter completes a partial command name; Enter on an exact
command runs it. Arguments and multiline drafts keep normal editing behavior.
Clap parses and validates slash commands from the catalog used by help and the
picker. Message text keeps its punctuation, spacing and line breaks.

| Key                            | Action                                                                           |
| ------------------------------ | -------------------------------------------------------------------------------- |
| Enter                          | Send a turn, or queue it while a run is active                                   |
| Tab                            | Complete the selected slash command without running it                           |
| Alt+Enter, Shift+Enter, Ctrl+J | Insert a newline                                                                 |
| Ctrl+K                         | Steer the active run with the draft                                              |
| Ctrl+C                         | Cancel a pending account/model command; otherwise stop the run or exit when idle |
| Ctrl+D                         | Exit when the composer is empty                                                  |
| Ctrl+S                         | Choose a saved session                                                           |
| Up, Down                       | Choose a command or panel row; otherwise recall sent messages                    |
| Left, Right, Home, End         | Move the text cursor                                                             |
| PageUp, PageDown, mouse wheel  | Scroll the transcript                                                            |
| End, Escape                    | Follow current output                                                            |
| Escape                         | Close a panel or command list without deleting the draft                         |

Terminals vary in how they encode modifier keys. Ctrl+J is the newline fallback when Alt+Enter or Shift+Enter is unavailable. Bracketed paste inserts text without submitting it.

| Command                                       | Action                                                                               |
| --------------------------------------------- | ------------------------------------------------------------------------------------ |
| `/help`                                       | Show command and key reference                                                       |
| `/model [PROVIDER] MODEL`                     | Choose a model for this CLI process                                                  |
| `/model`, `/models`                           | Show the current model and available native models                                   |
| `/login`                                      | Choose a provider for subscription sign-in or API-key setup guidance                 |
| `/accounts`                                   | List safe ChatGPT registrations and official Claude authentication status            |
| `/account REGISTRATION`, `/account claude`    | Select a saved ChatGPT registration or the official client's current Claude identity |
| `/logout REGISTRATION`, `/logout claude`      | Sign out of a ChatGPT registration or the official Claude client                     |
| `/status`                                     | Show the effective connection, account, session, grants and concurrency              |
| `/new [title]`                                | Create a session in the configured workspace                                         |
| `/sessions`                                   | Choose a saved session                                                               |
| `/agents`                                     | Choose among managed sessions                                                        |
| `/resume ID`                                  | Resume by full ID or unique prefix                                                   |
| `/queue MESSAGE`                              | Add a turn, or start when idle                                                       |
| `/steer MESSAGE`                              | Send direction to the active run                                                     |
| `/stop`                                       | Cancel current work                                                                  |
| `/quit`                                       | Cancel runs and exit                                                                 |
| `/extensions`                                 | List enabled packages, tool/command grants and hook counts                           |
| `/widgets pause`, `/widgets resume`           | Pause selected widgets, or resume after a failure                                    |
| `/theme NAME`                                 | Select a theme asset from an enabled package                                         |
| `/prompt NAME [INPUT]`, `/skill NAME [INPUT]` | Add a package asset and optional input to a run                                      |
| `/reload @publisher/package`                  | Reload unchanged exports while all runs and callbacks are idle                       |

Switching sessions leaves other runs managed by the same ejm-harness. `/stop` targets the selected session. Exiting cancels and drains all runs owned by this CLI process.

The TUI opens while the initial connection loads, even when provider credentials
are missing. Ctrl+C cancels pending setup. `/login` opens a provider picker with
ChatGPT, Claude, OpenAI-compatible API, native Responses API and Anthropic API
choices. Up and Down select a row; Enter starts the chosen sign-in or setup
guidance. Escape restores the command draft and keeps the active connection.
Opening or browsing the picker starts no sign-in. Shell commands still require
an explicit provider, such as `ejm-cli login chatgpt` or `ejm-cli login claude`.

API choices show endpoint, model and environment-key setup guidance. They do not
accept or save credentials or change the active connection. Set API keys through
your shell's secret handling before starting the CLI. Restart after setting a
new key outside the app. Use `/model PROVIDER MODEL` to connect with a key already
available to the process; endpoint and key-variable options remain launch
settings. Use `/account REGISTRATION` or `/account claude` to select an existing
account. Model turns before connection report the problem and keep the draft.
Plain `run` mode still requires a usable provider at startup.

Providers are `openai`, `responses`, `chatgpt`, `claude` and `anthropic`. `/model MODEL`
keeps the current provider; include the provider to leave a
package provider. Native ChatGPT and OpenAI Responses access can fetch a model
catalog. Claude accepts model aliases, for example `/model claude sonnet`.
Other providers accept explicit model IDs. Catalog retrieval uses the
selected provider's credentials and may need network access.

Model and account changes apply to every session managed by this CLI process
and require all managed runs to be idle. Failed changes retain the prior
connection and draft. Selection lasts for the current process; use launch flags
for later starts. `/status` reports agent, model and tool concurrency separately.

Choosing ChatGPT from `/login` opens the browser after binding its callback listener, then
selects the saved registration when sign-in succeeds. Authorization links,
state, codes and tokens stay out of the transcript. Ctrl+C cancels pending
sign-in. For manual browser links, use the shell command
`ejm-cli login chatgpt --no-browser`. Choosing Claude from `/login` temporarily
leaves the TUI and gives terminal input to the official client. The TUI returns after success,
failure or Ctrl+C. Cancelled or failed sign-in keeps the command draft and prior
connection. The official client's sign-in output stays outside the conversation
transcript. SIGTERM still exits the app. API keys remain environment variables;
`anthropic` uses the API, while `claude` uses the official client's subscription.

## Packages and trust

`package`, `packages` and `pkg` name the same command. Package state lives in the state directory's `packages` subdirectory. Code packages need Node 24 or later, or Python 3.11 or later. Asset-only packages need neither runtime. Installation does not install dependencies or run package code, package-manager lifecycle scripts or build backends.

**Enabled native packages run arbitrary code as your user.** Named grants control registration with the harness. They do not restrict file access, network access, secrets, subprocesses or runtime memory. Package hashes verify selected content; they do not establish that code is safe. Inspect the source and bundled dependencies before enabling a package.

### Install and enable

For a local package directory containing a valid `ejm.package.json` (the only supported manifest filename):

```sh
ejm-cli package install ./my-package
ejm-cli package list
```

Installation records an exact content snapshot and leaves it disabled with no grants. Local `.tgz` archives work too. Use `local:` or `./` for a local path that resembles a remote reference.

Remote source forms require exact versions or a full Git commit. Replace the names and versions in these examples with a package you have reviewed:

| Source                       | Form                                                                    |
| ---------------------------- | ----------------------------------------------------------------------- |
| Package registry             | `@publisher/package@1.2.3`                                              |
| Public npm                   | `npm:@publisher/package@1.2.3`                                          |
| Public PyPI portable wheel   | `pypi:distribution==1.2.3`                                              |
| Public GitHub commit archive | `git:https://github.com/publisher/package.git#FULL_40_CHARACTER_COMMIT` |

Pass the source as one argument to `ejm-cli package install SOURCE`. Registry sources require `--registry ORIGIN` or `EJM_REGISTRY_URL` as described below. Git support does not fetch submodules or Git LFS. npm and PyPI support do not install dependencies. Bundle unsupported layouts locally. See the [harness package contracts](https://github.com/ejmlabs/ejm-harness/blob/main/docs/packages.md) for archive, runtime and source requirements.

After inspection, explicitly trust the installed version and name only the exports you need. This example uses the names from the harness guide:

```sh
ejm-cli package enable @example/workflow --trust \
  --tool word_count --hook run_start --grant-provider offline --command hello --grant-widget status
```

`--tool`, `--hook`, `--grant-provider`, `--command` and `--grant-widget` may repeat. The grant options are separate from global `--provider` and `--widget` host selectors. Hook names are `run_start`, `before_model`, `before_tool`, `after_tool` and `run_end`. Undeclared exports are rejected. Themes, prompts and skills are available from enabled packages without a separate export grant. Trust applies to the entire native package, including its imports.

### Use and change packages

Select a granted provider or widget, or an enabled theme, by its qualified name:

```sh
ejm-cli --package-provider @example/workflow:offline \
  --theme @example/workflow:midnight --widget @example/workflow:status
```

`--widget` may repeat up to four times. Widgets refresh at most once a second. A widget failure pauses refresh until `/widgets resume`. Unambiguous short export names also work. A slash command such as `/hello INPUT` invokes a granted package command; `/@publisher/package:hello INPUT` resolves a collision. Built-in commands keep precedence.

`/prompt NAME INPUT` and `/skill NAME INPUT` submit the asset text and optional input as a turn, or queue it during an active run. `/theme NAME` selects an enabled theme; `NO_COLOR` remains authoritative. `/reload @publisher/package` requires all runs and extension callbacks to be idle. Changed manifests or grants require a restart.

```sh
ejm-cli package update @publisher/package 1.2.4
ejm-cli package disable @publisher/package
ejm-cli package remove @publisher/package
```

Update installs a new exact registry version and clears trust and grants. Inspect it and enable it again explicitly. Disable and remove affect future CLI processes. Active processes retain their loaded snapshot until exit; restart to unload them. Remove deletes the registration, and stored snapshots may remain for active hosts.

### Registry and publishing

Registry operations require an explicit origin. Set `EJM_REGISTRY_URL` to a registry you operate or trust, or pass `ejm-cli package --registry ORIGIN COMMAND`. The flag also works after the package subcommand and overrides the environment. There is no default registry and no hosted registry is claimed by this project.

The origin must use HTTPS, with no path, query, fragment or URL credentials. Plain HTTP is allowed only for loopback development. Invalid origins fail before registry credentials or network use. Search, info, update, registry-source install, login, logout, whoami, publisher, publish and credential import require this setting. Without it they fail with setup guidance. Local list, pack, enable, disable, remove and local installs need no registry. Exact npm, PyPI and Git sources also work without one.

With `EJM_REGISTRY_URL` set, these commands make network requests:

```sh
ejm-cli package search QUERY
ejm-cli package info @publisher/package 1.2.3
ejm-cli package login --no-browser
ejm-cli package whoami
ejm-cli package logout
```

Registry sign-in uses a revocable account session, separate from model-provider accounts. Approve only the request you started. Tokens stay in a separate origin-scoped registry namespace in the harness encrypted vault under the package state root; changing the selected registry does not transfer tokens between origins; the CLI never accepts or displays them as arguments. Logout removes the local token, retains its migration marker and reports remote revocation failures. For an earlier `registry-session.json`, stop every older CLI and IDE that could write it, then run `ejm-cli package migrate-credentials --old-clients-stopped`. Import verifies the saved destination before deleting the legacy file. Retry cleanup with those clients still stopped if removal fails. Old clients do not honor the new vault lock. Direct downgrade and stale-token reimport after logout are unsupported.

`ejm-cli package pack DIRECTORY --output package.tgz` creates a new local archive and refuses to overwrite an existing file. `package publisher HANDLE --display-name NAME` claims a registry namespace, and `package publish DIRECTORY` uploads a package using the saved registry session. Those last two commands change the remote registry. They are deliberate author actions, separate from installing or running the CLI.

## Scripts and saved sessions

```sh
ejm-cli run "Find the parser"
ejm-cli run "Find the parser" --json
ejm-cli sessions
ejm-cli sessions --json
ejm-cli --session SESSION_ID run "Now explain its tests"
ejm-cli --session SESSION_ID
ejm-cli doctor
ejm-cli --help
```

Plain mode streams assistant text on stdout and tool status on stderr. JSON mode emits one durable event per line on stdout. Failed and cancelled runs return a nonzero exit code. Session inspection and doctor work without provider credentials.

`--cwd` chooses the workspace. `--state-dir` or `EJM_CLI_STATE_DIR` chooses durable storage. The default is `$XDG_DATA_HOME/ejm-cli`, macOS `~/Library/Application Support/ejm-cli`, Windows `%LOCALAPPDATA%/ejm-cli`, or `~/.local/share/ejm-cli` on other systems. Never commit that directory or use a public directory for private sessions.

### Capacity and host coordination

On macOS, the CLI selects automatic shared host capacity without a flag. On Linux and Windows, omission retains legacy process limits pending native acceptance: 100 managed sessions, eight model requests, four tool jobs, and 512 MiB of admitted tool memory, outside the shared registry. Use `--capacity fixed` for coordinated fixed targets on macOS.

```sh
ejm-cli --capacity auto
ejm-cli --capacity auto --model-concurrency 4 --tool-memory-mib 512
ejm-cli --capacity fixed
ejm-cli capacity reset
```

Explicit `--capacity auto` delegates operating targets, host observations, feedback, and local learning to the harness. Explicit `--capacity fixed` uses the former defaults for omitted numeric flags while sharing host admission. In either coordinated mode, `--max-sessions`, `--model-concurrency`, `--tool-concurrency`, and `--tool-memory-mib` are hard ceilings. Omitted values in automatic mode do not silently impose the old fixed defaults. Zero values are rejected. `--max-steps` and `--run-timeout-ms` still bound each run.

Coordination covers participating processes in the same current OS-user and visible host/container resource scope, even with different session-state directories. Other users, older clients, and unrelated applications are not coordinated; their effects can only be sampled. Missing scope or native boot identity can reject an explicit coordinated launch. Do not delete the shared ledger to recover capacity. These are admission budgets, not a native security sandbox or proof of a hard memory ceiling on macOS.

Capacity is prepared before enabled package code loads and is reused by the package runtime, final harness, and executor. Unused language hosts stay lazy. Host launch, imports, residency, callback peaks, and reload overlap use the same admission context; imports or reloads may wait for capacity. Existing trust and export grants remain required.

Ctrl+C, Unix SIGTERM, and supported Windows shutdown controls request startup cancellation before package loading and through plain-run provider setup. The CLI waits for initialized owners to clean up before releasing its prepared context. Normal shutdown stops the harness first, then shuts down the package runtime. Failed imports, theme or provider selection, and final composition use the same cleanup path. A cleanup failure is reported alongside the original error; uncertain native work is never replayed. Forced process termination cannot promise cleanup.

Claude inference and its in-run status checks use that executor's native ownership. Standalone sign-in, sign-out, account status and connection preflight retain the auth module's bounded process lifecycle outside shared agent admission.

Native supervision uses lazy helpers from the CLI executable, with no separate container or service installation. The harness owns their protocol, lifetime links, and cleanup. Shared ownership persists while its OS lock is held; missed heartbeats do not permit takeover. Confirmed owner loss can permit a replacement after lock release, but uncertain launches are never replayed. No per-job lifecycle journal is added. On macOS, arbitrary detached descendants and cleanup after abrupt owner death remain limited; fresh host observations can account for surviving load without proving that every descendant exited.

The working harness implementation gives each shared helper a separate coarse
commitment before dependent tool or package admission. Its 32 MiB baseline is
provisional, and its lifetime charge survives the creating client. The release
CLI and worker images pinned in the harness's
[embedding-memory evidence](https://github.com/ejmlabs/ejm-harness/blob/main/docs/benchmarks.md#real-embedding-helper-memory)
fit that estimate in startup and saturated control-state trials on this Mac.
The report retains earlier failures and limits. These trials do not calibrate
other images, platforms or full process trees. See
[helper commitments](https://github.com/ejmlabs/ejm-harness/blob/main/docs/host-capacity.md#helper-commitments)
for the eight-generation pool and protocol migration. Native work stages its
full demand before readiness and releases unused readiness whenever activation
must wait, including idle-helper reuse. Package startup admits resident memory
and import scratch together, then retains only the resident share after import.
The harness repairs the
[first-start lock publication gap](https://github.com/ejmlabs/ejm-harness/blob/main/docs/host-capacity.md#stable-lock-creation-repair)
with an exact public filesystem fork pin. Source builds require no crates.io
publication of that fork. Scoped macOS recovery and representative acceptance
passed; automatic defaults are enabled on macOS. Other platforms and embedding
images still need calibration and native acceptance. The sustained package idle measurement also missed its owner CPU
target. See
[the package evidence](https://github.com/ejmlabs/ejm-harness/blob/main/docs/packages.md#sustained-idle-measurement)
for the measured scope and limits.

`/status` and `doctor` explain mode, scope, active and queued model/tool work, targets, hard ceilings, wait reasons, observation age/availability, and local learning state. The TUI's managed-agent count covers running and waiting runs. Separate model, tool and provider counts show where work is active or queued. Their cached snapshots do not collect measurements or advance tuning. A target below a configured ceiling can reflect pressure, shared reservations, provider feedback, or learning. An unavailable observation is not a healthy reading. Sampled external load is distinct from coordinated reservations.

Native status reports owner-lock, loss-detection and resource-domain placement guarantees, descendant and owner-death cleanup, recovery, and cached local lifetime links as enforced, monitored, or unsupported. Observed owner deaths and the last dead epoch describe recorded recovery. The last authenticated owner PID is cached and does not prove current liveness. These capability labels do not establish platform-specific acceptance or a hard macOS memory limit.

`ejm-cli capacity reset` clears the harness's applicable local learning profiles. It does not reset active reservations, provider rate charges, session history, credentials, explicit ceilings, or tool/shell grants. The remote command catalog has no capacity-reset or host-policy authority. Learning does not promise an optimal concurrency value or measured thousand-agent throughput.

## Rendering and footprint

The TUI polls input and bounded pending work every 16 ms and paints changed content on a separate 32 ms cadence. Both intervals skip missed ticks. It subscribes to the selected session through the public harness interface and reads aggregate status for background work. Unselected sessions do not send all their output through the selected view.

Opening a session reads a consistent tail of at most 128 events and 64 KiB. Earlier output is marked as omitted from the display and remains durable. Session loading and recovery run outside the input handler, are tied to the selection generation, and preserve the draft. Each recovery captures a journal high-water boundary and applies at most 16 events or a 2 ms processing slice per cycle. Pages also have a 64 KiB byte bound. A single event too large for that bound causes a fresh bounded tail and an omission marker. Stop and draft input are processed between batches.

Filtered events carry a selected-session predecessor token. Numeric gaps in global event sequences are normal when other sessions emit output; only a token discontinuity or explicit loss signal starts recovery.

The session picker retains one 50-session page, renders visible rows, and accepts a literal title or ID prefix up to 256 UTF-8 bytes. Up/Down selects rows and crosses page boundaries. PageUp/PageDown changes pages; Home returns to the first page. At most 32 previous-page cursors are retained. Search and navigation completions from an older query are discarded. `/resume ID_OR_PREFIX` resolves ambiguity against the complete saved-session set. `sessions` and `sessions --json` retain their complete text and JSON-array output contracts while reading pages.

Local authority uses compact run identity and transition/cursor snapshots rather than reconstructing display history. Its cache retains at most 1000 views and evicts inactive, unselected, unshared entries. Live and shared entries stay protected. Before eviction, a cached harness run lookup clears stale live pins without reading the journal. If every entry is protected, a new selection returns a recoverable error; existing Stop actions remain available. Cache eviction does not revoke or recreate a share.

Local Stop holds the authority lock, discards pending prompts for the selected session, reads the live run through `Harness::active_run_id`, and calls `Harness::cancel_run` for that exact run. It does not refresh the journal-backed session view. `LocalBackend::cancel` returns `Result<Option<String>>`: `Some(run_id)` acknowledges a cancellation request for that run; `None` means no active run was found. A stale-run error leaves a replacement run untouched.

The app keeps that run's acknowledgement through tail and recovery snapshots. Only a matching `RunFinished` or `SessionView.last_transition` supplies the final outcome. A newer live or snapshotted run clears the old acknowledgement. An acknowledgement does not prove cancellation or cleanup has finished. Ctrl+C uses the same live lookup while the app is loading; a second press while stopping exits.

Visible transcript memory is capped at 256 KiB and 256 blocks. Individual tool output previews keep their most recent 24 KiB; complete output remains in the ejm-harness event store and executor artifacts. The composer accepts up to 64 KiB and retains the last 100 sent messages. Escape sequences and terminal control characters are removed from displayed provider and tool output.

Terminal raw mode, mouse tracking, bracketed paste and the alternate screen are restored on return, errors and Rust panics. The TUI requires a terminal. Use `run` in pipes and automation. `--no-color` or `NO_COLOR` selects a monochrome TUI. `--plain`, or a locale that is not UTF-8, switches to ASCII symbols and borders. The header uses the CLI's `>_` prompt and cursor mark as plain text beside its name in every mode.

The TUI uses the ejmlabs design kit palette (`ejm_theme.rs`) in light and dark variants and never paints a background, so it sits on your terminal's own. `--appearance auto|light|dark` (or `EJM_CLI_APPEARANCE`) picks the variant. Auto reads `COLORFGBG` and assumes dark when it is absent. Brand colors need a terminal that sets `COLORTERM=truecolor` or `24bit`; others get named ANSI colors. A package theme from `--theme` overrides all of this.

These bounds and refresh intervals describe implementation choices, not old-laptop performance claims. The 4-core, 8 GB and 100-session goal requires separate benchmark evidence.

## Remote access

`ejm-cli remote login`, `accounts`, `logout`, `migrate-credentials`, `devices`, `host` and `connect`
use a separate relay account. Set `--relay https://YOUR-RELAY` or
`EJM_CLI_RELAY_URL`. `remote host` grants view access; `--control` grants
prompt, steer, queue and cancel actions subject to the host's grants.
`--memory-only` uses temporary relay credentials and keys in that process. Before first persistent host/controller use, including a fresh installation, run `remote migrate-credentials` after all older host/controller writers stop. The command asks for local confirmation, preserves legacy identities and prepares a fresh endpoint only when no identity exists. A missing marked key requires recovery. Read the [credential recovery steps](remote-control.md#credentials-and-recovery).

The TUI accepts `/remote share [view|control]`, `/remote status` and
`/remote stop`. The account-protected `/account` dashboard links to `/control`
for live browser control. The native client protects against a forwarding
operator; the browser must trust the operator-served code. Read the
[remote guide](remote-control.md) before pairing, especially uncertain action
results, key storage, privacy and host revocation.
