# ejm-cli

The `ejm-cli` binary is the reference client for the public Rust ejm-harness. It needs no account or server. The CLI links ejm-harness directly and uses the same tool registration and session APIs as community applications.

## Start

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
| Up, Down | Recall sent messages |
| Left, Right, Home, End | Move the text cursor |
| PageUp, PageDown, mouse wheel | Scroll the transcript |
| End, Escape | Follow current output |
| Escape | Close a panel |

Terminals vary in how they encode modifier keys. Ctrl+J is the newline fallback when Alt+Enter or Shift+Enter is unavailable. Bracketed paste inserts text without submitting it.

| Command | Action |
| --- | --- |
| `/help` | Show command and key reference |
| `/new [title]` | Create a session in the configured workspace |
| `/sessions` | Choose a saved session |
| `/resume ID` | Resume by full ID or unique prefix |
| `/queue MESSAGE` | Add a turn, or start when idle |
| `/steer MESSAGE` | Send direction to the active run |
| `/stop` | Cancel current work |
| `/quit` | Cancel runs and exit |

Switching sessions leaves other runs managed by the same ejm-harness. `/stop` targets the selected session. Exiting cancels and drains all runs owned by this CLI process.

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

Terminal raw mode, mouse tracking, bracketed paste and the alternate screen are restored on return, errors and Rust panics. The TUI requires a terminal. Use `run` in pipes and automation. `--no-color` or `NO_COLOR` selects a monochrome TUI.

The TUI uses the ejmlabs brand palette in light and dark variants and never paints a background, so it sits on your terminal's own. `--appearance auto|light|dark` (or `EJM_CLI_APPEARANCE`) picks the variant. Auto reads `COLORFGBG` and assumes dark when it is absent. Brand colors need a terminal that sets `COLORTERM=truecolor` or `24bit`; others get named ANSI colors. A package theme from `--theme` overrides all of this.

These bounds and refresh intervals describe implementation choices, not old-laptop performance claims. The 4-core, 8 GB and 100-session goal requires separate benchmark evidence.
