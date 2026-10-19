# ejm-cli

The `ejm-cli` binary is the reference client for the public Rust ejm-harness API. The CLI links ejm-harness directly and uses the same tool registration and session APIs as community applications. Offline replay needs no account or server. Hosted models need provider access.

## Start

Build from sibling source checkouts with Rust 1.99.1, as shown in the [quick start](../README.md#quick-start). The examples below use `ejm-cli` from your `PATH`, except for the source-tree `cargo run` examples.

```sh
cargo run -p ejm-cli -- --replay
cargo run -p ejm-cli -- --replay run "Explain this repository"
```

`--replay` uses a deterministic echo provider with no network calls. To exercise tool calls and failures, use `--replay-file responses.json`. That file is an array of public `ModelResponse` objects.

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

An account selector accepts a full registration ID or a unique prefix. If several accounts are saved, choose one explicitly. `accounts` reads cached metadata without contacting the provider. `models` retrieves the selected account's current model catalog. When `--model` and `EJM_CLI_MODEL` are absent, ChatGPT selects the first model from that catalog. Logout removes the saved registration and attempts provider revocation, reporting when revocation is unconfirmed.

Credentials live under the configured state directory's `auth` subdirectory. On Unix, the store enforces owner-only directory and file permissions, atomic writes and cross-process locking. This is a private local file store, not an encrypted keychain. Account storage fails closed on platforms without supported private-file handling. Keep the entire state directory private.

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

`login anthropic` shows official Claude Code sign-in guidance. ejm-cli does not collect or reuse Claude subscription tokens. API-key access falls back to `ANTHROPIC_API_KEY` for this provider.

## Interaction

| Key | Action |
| --- | --- |
| Enter | Send a turn, or queue it while a run is active |
| Alt+Enter, Shift+Enter, Ctrl+J | Insert a newline |
| Ctrl+K | Steer the active run with the draft |
| Ctrl+C | Stop the current run, or exit when idle |
| Ctrl+D | Exit when the composer is empty |
| Ctrl+S | Choose a saved session |
| Up, Down | Choose a command while the picker is open; otherwise recall sent messages |
| Tab | Complete the selected command without running it |
| Left, Right, Home, End | Move the text cursor |
| PageUp, PageDown, mouse wheel | Scroll the transcript |
| End, Escape | Follow current output |
| Escape | Close a panel or command picker without changing the draft |

Type `/` at the beginning of an empty composer to see built-in and granted package
commands. Keep typing to filter by command name. Up/Down chooses a result. Tab
completes it; Enter completes an unfinished command name or runs an exact command.
Completion never starts a run. Commands that need an argument leave a trailing
space for it. Escape dismisses the picker and keeps the draft editable.

`/help` shows keyboard shortcuts and the same command catalog. Up/Down scrolls the
command list, PageUp/PageDown jumps through it, Enter copies a selection into the
composer, and Escape closes the panel. The next Enter runs the completed command.
Invalid built-in arguments show usage and keep the draft. Package commands keep
qualified names when a short name is ambiguous or collides with a built-in.

Terminals vary in how they encode modifier keys. Ctrl+J is the newline fallback when Alt+Enter or Shift+Enter is unavailable. Bracketed paste inserts text without submitting it.

| Command | Action |
| --- | --- |
| `/help` | Show keys and choose a command to complete |
| `/new [title]` | Create a session in the configured workspace |
| `/sessions` | Choose a saved session |
| `/agents` | Choose among managed sessions |
| `/resume ID` | Resume by full ID or unique prefix |
| `/queue MESSAGE` | Add a turn, or start when idle |
| `/steer MESSAGE` | Send direction to the active run |
| `/stop` | Cancel current work |
| `/quit` | Cancel runs and exit |
| `/extensions` | List enabled packages, tool/command grants and hook counts |
| `/widgets pause`, `/widgets resume` | Pause selected widgets, or resume after a failure |
| `/theme NAME` | Select a theme asset from an enabled package |
| `/prompt NAME [INPUT]`, `/skill NAME [INPUT]` | Add a package asset and optional input to a run |
| `/reload @publisher/package` | Reload unchanged exports while all runs and callbacks are idle |

Switching sessions keeps the composer draft and sent-message history, selected
package palette, command catalog and widget lines. Other runs remain managed by
the same ejm-harness. `/stop` targets the selected session. Exiting cancels and drains all runs owned by this CLI process.

## Packages and trust

`package`, `packages` and `pkg` name the same command. Package state lives in the state directory's `packages` subdirectory. Code packages need Node 24 or later, or Python 3.11 or later. Asset-only packages need neither runtime. Installation does not install dependencies or run package code, package-manager lifecycle scripts or build backends.

**Enabled native packages run arbitrary code as your user.** Named grants control registration with the harness. They do not restrict file access, network access, secrets, subprocesses or runtime memory. Package hashes verify selected content; they do not establish that code is safe. Inspect the source and bundled dependencies before enabling a package.

### Install and enable

For a local package directory containing a valid `gerry.package.json`:

```sh
ejm-cli package install ./my-package
ejm-cli package list
```

Installation records an exact content snapshot and leaves it disabled with no grants. Local `.tgz` archives work too. Use `local:` or `./` for a local path that resembles a remote reference.

Remote source forms require exact versions or a full Git commit. Replace the names and versions in these examples with a package you have reviewed:

| Source | Form |
| --- | --- |
| Package registry | `@publisher/package@1.2.3` |
| Public npm | `npm:@publisher/package@1.2.3` |
| Public PyPI portable wheel | `pypi:distribution==1.2.3` |
| Public GitHub commit archive | `git:https://github.com/publisher/package.git#FULL_40_CHARACTER_COMMIT` |

Pass the source as one argument to `ejm-cli package install SOURCE`. Git support does not fetch submodules or Git LFS. npm and PyPI support do not install dependencies. Bundle unsupported layouts locally. See the [harness package contracts](https://github.com/ejmlabs/ejm-harness/blob/main/docs/packages.md) for archive, runtime and source requirements.

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

Registry commands use `https://harness.meetgerry.ai`. The endpoint and live service have their own availability and release requirements. These commands make network requests:

```sh
ejm-cli package search QUERY
ejm-cli package info @publisher/package 1.2.3
ejm-cli package login --no-browser
ejm-cli package whoami
ejm-cli package logout
```

Registry sign-in uses a revocable account session, separate from model-provider accounts. Approve only the request you started. Tokens stay in the private package store's `registry-session.json`; the CLI never accepts or displays them as arguments. Logout removes the local session and reports remote revocation failures.

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

`--max-sessions` defaults to 100. `--model-concurrency` defaults to eight. Local tools share the ejm-harness governor, so creating a session does not allocate a shell or reserve a command slot.

## Rendering and footprint

The TUI draws only when content changes, with a 32 ms maximum refresh cadence. Terminal rendering and tool execution use separate ejm-harness event delivery. Live broadcast hints can be recovered from durable events after lag.

Visible transcript memory is capped at 256 KiB and 256 blocks. Individual tool output previews keep their most recent 24 KiB; complete output remains in the ejm-harness event store and executor artifacts. The composer accepts up to 64 KiB and retains the last 100 sent messages. Escape sequences and terminal control characters are removed from displayed provider and tool output.

Terminal raw mode, mouse tracking, bracketed paste and the alternate screen are restored on return, errors and Rust panics. The input reader also discards unread input before returning to an echoing terminal. It rejects malformed UTF-8, more than 64 KiB of retained input, or more than 1,024 queued events with an error and restores the terminal. Action events marked as repeats do not resubmit or cancel; editing repeats still work. See [Terminal input boundary](terminal-input.md) for source status, platform limits and verification.

The TUI requires a terminal. Use `run` in pipes and automation. `--no-color` or `NO_COLOR` selects a monochrome TUI. `--plain`, or a locale that is not UTF-8, switches to ASCII symbols and borders.

The TUI uses the ejmlabs design kit palette (`ejm_theme.rs`) in light and dark variants and never paints a background, so it sits on your terminal's own. `--appearance auto|light|dark` (or `EJM_CLI_APPEARANCE`) picks the variant. Auto reads `COLORFGBG` and assumes dark when it is absent. Brand colors need a terminal that sets `COLORTERM=truecolor` or `24bit`; others get named ANSI colors. A package theme from `--theme` overrides all of this.

These bounds and refresh intervals describe implementation choices, not old-laptop performance claims. The 4-core, 8 GB and 100-session goal requires separate benchmark evidence.
