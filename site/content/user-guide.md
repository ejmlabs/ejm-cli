## Build and open the CLI {#build}

You need Git, Rust 1.99.1 and sibling checkouts of the public CLI and harness repositories. Select the harness revision recorded by the CLI before building:

```sh Check out compatible source and try the offline CLI
git clone https://github.com/ejmlabs/ejm-harness.git
git clone https://github.com/ejmlabs/ejm-cli.git
cd ejm-cli
git -C ../ejm-harness checkout --detach "$(cat .github/harness-revision)"
cargo build --release --locked
./target/release/ejm-cli --replay
```

`--replay` uses a deterministic echo provider without model requests. It demonstrates terminal interaction, not a model's coding ability. Use a configured [provider](#providers) for model responses. The TUI needs a terminal; `run` also works in pipes. The core binary needs no Node or Python runtime.

The repository selects Rust 1.99.1 through `rust-toolchain.toml`. See [developer setup](/docs/developer-guide/#setup) for tools and Linux credential-store dependencies. There is no supported binary installer or package-manager distribution. Linux and macOS native remote checks are recorded in the [source acceptance reference](/docs/reference/platforms/); they do not establish every platform or a signed release.

The remaining examples use `ejm-cli` from your `PATH`; `./target/release/ejm-cli` also works.

### Native Windows source use {#windows-source}

Windows terminal behavior needs separate native verification. Cross-compilation or WSL does not establish native Windows support. Keep [release limits](https://github.com/ejmlabs/ejm-cli/blob/main/RELEASING.md) and native platform evidence separate from a successful source build.

## Choose a workspace and grants {#grants}

The current directory is the workspace. Set `--cwd` to choose another existing directory. Native read tools are available by default. Add each grant when starting the process:

```sh Choose the workspace and explicitly grant native tools
ejm-cli --cwd ./my-project
ejm-cli --cwd ./my-project --allow-write
ejm-cli --cwd ./my-project --allow-write --allow-shell
```

`--allow-write` registers file write tools. `--allow-shell` registers shell execution as the current user. These separate grants are fixed for the process. Configure a provider before these commands, or add `--replay` for offline interaction.

> [!WARNING]
> **Native execution is not a sandbox.** Tools and trusted packages run as you. Registration grants do not confine file, network or process access. Run `ejm-cli doctor` to inspect the executor's actual capabilities.

## Connect a model {#providers}

The available provider choices are `openai`, `responses`, `chatgpt` and `anthropic`. Hosted use needs provider access and can incur provider charges. Keys come from environment variables; there is no API key argument. Configure the provider before launching the TUI.

### OpenAI compatible APIs {#compatible}

The default provider uses Chat Completions. Set `EJM_CLI_API_KEY` through your shell's secret handling, or use the `OPENAI_API_KEY` fallback. Choose a model and endpoint supported by your provider:

```sh Select an OpenAI compatible endpoint and model
export EJM_CLI_MODEL=deepseek-chat
export EJM_CLI_BASE_URL=https://api.deepseek.com/v1
ejm-cli
```

`--model` and `--base-url` override environment settings. `--api-key-env NAME` selects another key variable. The default endpoint is OpenAI's API and the default compatible model is `gpt-4.1-mini`.

### ChatGPT account access {#chatgpt}

ChatGPT and native Responses use fixed official endpoints. Clear a custom base URL before selecting them. Start only a sign-in you intend to approve:

```sh Sign in to ChatGPT and inspect saved accounts
unset EJM_CLI_BASE_URL EJM_CLI_MODEL
ejm-cli login chatgpt
ejm-cli accounts
ejm-cli --provider chatgpt models
ejm-cli --provider chatgpt
```

The CLI binds a local callback listener before opening the browser. Keep it running until completion, timeout or Ctrl+C. `ejm-cli login chatgpt --no-browser` prints a checked manual sign-in URL; do not share it. Codes and bearer tokens never become command arguments, logs or TUI content.

With several accounts, add `--account REGISTRATION`, using a full registration ID or unique prefix from `accounts`. Choose a model with `--model MODEL`; otherwise ChatGPT selects the first model in its current account catalog. `accounts` reads cached metadata; `models` contacts the provider. `ejm-cli logout REGISTRATION` removes the saved registration and attempts provider revocation, reporting when revocation is unconfirmed.

### Claude subscription access {#claude}

This published CLI source has no Claude subscription provider. `login anthropic` points to the official Claude Code sign-in guidance; it does not import subscription tokens. Use the native Anthropic API provider below with your API credentials.

### Responses and Anthropic APIs {#native}

Set `OPENAI_API_KEY` or `EJM_CLI_API_KEY` for native Responses. Set `ANTHROPIC_API_KEY` or `EJM_CLI_API_KEY` for Anthropic. Replace `MODEL` with a model available to your account:

```sh Choose a native API provider
ejm-cli --provider responses models --json
ejm-cli --provider responses --model MODEL
ejm-cli --provider anthropic --model MODEL
```

Anthropic requires an explicit model. `--reasoning-effort LEVEL` applies to native Responses providers and must be supported by the selected model. Read `--help` for accepted values. `--account` applies only to ChatGPT.

### Saved credentials and recovery {#credential-recovery}

The selected harness revision encrypts provider and native relay credentials and protects the vault key with the native key service. Provider state lives in the configured state directory's `auth` folder. Keep the entire state directory private. Locked, denied or corrupt key access fails closed. Preserve the original vault and native key for recovery; do not replace a missing key or treat unavailable storage as signed out. See the [account reference](/docs/reference/cli/#provider-accounts-and-models).

Relay and registry credentials have separate scopes. Their legacy migrations require older clients to stop first; the new lock cannot exclude old writers. Follow [remote recovery](/docs/reference/remote-control/#credentials-and-recovery) before persistent relay use and the [package reference](/docs/reference/cli/#registry-and-publishing) for existing registry state.

## Work in the terminal {#terminal}

Type a request and press Enter. Type `/` at the start of the composer to browse commands, then type to filter. Up and Down choose a row, Tab completes without running it, and Escape closes the list while keeping the draft. Enter completes a partial command or runs an exact command. `/help` lists commands and keys; `/status` shows local session, connection, grants and work metadata.

<!-- table: Everyday terminal controls -->

| Key | Action |
| --- | --- |
| Enter | Send a turn, or queue it during an active run. |
| Alt+Enter, Shift+Enter, Ctrl+J | Insert a newline. Ctrl+J is the fallback. |
| Ctrl+K | Steer the active run with the draft. |
| Ctrl+C | Stop the current run, or exit when idle. |
| Ctrl+S | Open saved sessions. |
| Tab | Complete the selected command without running it. |
| Up, Down | Choose a command or panel row; otherwise recall sent messages. |
| PageUp, PageDown, mouse wheel | Scroll the transcript. |
| Escape | Close a panel or command list, or follow current output. |

Bracketed paste inserts text without sending it. `/queue MESSAGE` queues a turn, `/steer MESSAGE` redirects current work and `/stop` cancels the selected session's run. Provider and model selection are launch settings in this source. Exit and restart with the appropriate flags to change them.

## Save, switch and resume {#sessions}

Sessions are stored by the harness. Use `/new [title]` for a new session and `/sessions` or Ctrl+S for saved sessions. `/agents` chooses among managed sessions. Switching does not stop another session's run; `/stop` targets the selected session.

```sh List and resume saved sessions
ejm-cli sessions
ejm-cli sessions --json
ejm-cli --session SESSION_ID
ejm-cli --session SESSION_ID run "Now explain its tests"
```

Replace `SESSION_ID` with a saved ID. `/resume ID` also accepts a unique prefix. Exit cancels and drains the runs owned by this process.

Use `--state-dir DIRECTORY` or `EJM_CLI_STATE_DIR` to choose storage. The default is `$XDG_DATA_HOME/ejm-cli`, macOS `~/Library/Application Support/ejm-cli`, Windows `%LOCALAPPDATA%/ejm-cli`, or `~/.local/share/ejm-cli` on other systems. Keep it private and out of source control.

## Share one session remotely {#remote}

Native remote access needs a compatible separately operated HTTPS relay and an account on that service. Local CLI use needs no relay account. The host and controller must trust their installed clients and operating systems. The [native source acceptance record](/docs/reference/platforms/) separates source checks from Worker/browser adoption, live service acceptance and binary distribution.

### Sign in and start sharing {#remote-host}

Before persistent host or controller use on each machine and relay origin, including a fresh install, stop every older relay writer. Run the migration command and confirm locally only after those writers have stopped:

```sh Prepare relay credentials and share an offline session
export EJM_CLI_RELAY_URL=https://YOUR-RELAY
ejm-cli remote migrate-credentials
ejm-cli remote login
ejm-cli --replay remote host
```

Approve only the device sign-in you requested. Hosting grants view access by default. Add `--control` to permit submit, steer, queue and cancel actions. For model work, configure the host's provider instead of using replay. Existing file and shell grants still apply.

The host displays a private invitation in a protected terminal. Send it through a private channel. Compare the full controller fingerprint on both endpoints, then approve it on the host. Account recovery cannot replace this approval. Use `/remote share [view|control]`, `/remote status` and `/remote stop` from an existing local TUI. Stopping or changing the shared session discards queued remote prompts; a started run can continue locally.

### Use a native controller {#remote-native}

On the controller machine, set the same relay origin, stop older writers and complete the migration above. Then run:

```sh Sign in and connect a native controller
ejm-cli remote login
ejm-cli remote connect
```

Paste the invitation only into the protected prompt. In control mode, Enter submits when idle or queues during a run. `/steer TEXT` or Ctrl+K steers, `/stop` or Ctrl+C cancels, `/status` checks a pending result, and `/quit` leaves. View access cannot mutate. `--memory-only` explicitly keeps the foreground process's relay credentials and identity out of persistent storage.

### Browser access {#remote-browser}

Worker and browser source adoption has separate gates from the accepted native client. A browser controller requires trust in code delivered by the relay operator: that operator can change its JavaScript or WebAssembly to capture pairing secrets or plaintext. Independently verified native clients provide the stronger boundary against a malicious relay operator. Follow the selected operator's current preview limits; native source checks do not prove a hosted browser service.

### Accounts and recovery {#remote-account}

A relay account controls service admission; endpoint pairing controls session access. The forwarding relay can observe routing, connection timing and traffic size. Encryption does not protect a compromised endpoint. Never share an invitation, token or private state in a bug report.

Keep the original endpoint key and vault together. A missing marked key requires recovery rather than silent replacement. `remote logout` requests revocation and reports local or remote failures. Device revocation, migration, reconnect and private-terminal limits are in the [complete remote reference](/docs/reference/remote-control/). No payment or data-sale configuration is activated by this CLI source.

## Use plain text or JSON in scripts {#scripts}

```sh Run a turn without the TUI
ejm-cli run "Explain the parser"
ejm-cli run "Explain the parser" --json
ejm-cli --replay run "Hello"
```

Plain mode streams assistant text to stdout and tool status to stderr. JSON emits one durable event per line. Failed or cancelled runs exit nonzero. `doctor`, `sessions` and `--help` work without provider credentials.

## Add optional packages {#packages}

Install only a reviewed exact source. Local directories and archives, exact npm/PyPI versions and full Git commits have separate source rules in the [package reference](/docs/reference/cli/#packages-and-trust). Code packages need Node 24 or later or Python 3.11 or later; asset-only packages need neither.

```sh Install and inspect a reviewed local package
ejm-cli package install ./my-package
ejm-cli package list
```

Installation records a content snapshot and leaves it disabled without grants. It does not install dependencies or run package-manager lifecycle scripts. Inspect source and bundled dependencies before enabling declared exports. For a package named `@publisher/package` with tool `tool_name`:

```sh Enable a reviewed package and one declared tool
ejm-cli package enable @publisher/package --trust --tool tool_name
```

Other export grants are `--hook`, `--grant-provider`, `--command` and `--grant-widget`. Themes, prompts and skills need no separate export grant. Enabled code runs as your user; named grants control registration, not confinement. Updates clear trust and grants. Disable/remove affects future processes; restart to unload an active snapshot.

Hosted registry setup is not part of this public release. Use reviewed local or exact npm, PyPI and Git sources. Existing registry credential recovery remains documented in the canonical reference; changing a website does not migrate saved credentials or publish a package service.

## Adjust appearance and limits {#appearance}

The TUI uses your terminal background. Choose `--appearance light` or `--appearance dark` when auto detection is wrong. Auto reads `COLORFGBG` and otherwise assumes dark. `--no-color` or `NO_COLOR` selects monochrome; `--plain` selects ASCII symbols.

```sh Try an offline light or monochrome view
ejm-cli --replay --appearance light
ejm-cli --replay --no-color --plain
```

Default process limits are 100 managed sessions, eight model requests, four local tool jobs and 512 MiB of admitted tool memory. Set `--max-sessions`, `--model-concurrency`, `--tool-concurrency` and `--tool-memory-mib` at launch. These controls do not widen grants. `--max-steps` and `--run-timeout-ms` bound each run.

Visible transcript memory is capped at 256 KiB and 256 blocks. Complete history stays in the harness. Read [benchmark methods and results](/docs/reference/benchmarks/) for hardware, measurements and limits.

## Resolve common problems {#troubleshooting}

### The build cannot find ejm-harness.

Keep the repositories as siblings and check out the exact `.github/harness-revision` before using the committed lockfile. See [developer setup](/docs/developer-guide/#setup).

### Credentials are missing.

Set the key variable before starting, or run `ejm-cli login chatgpt` and launch with `--provider chatgpt`. `--replay` works without model access. Inspect `doctor` and `--help`; do not paste secrets into reports.

### A native provider rejects the base URL.

Remove `--base-url` and unset `EJM_CLI_BASE_URL` for `responses` or `chatgpt`.

### The wrong ChatGPT account is selected.

Run `ejm-cli accounts`, then use a full registration ID or unique prefix with `--provider chatgpt --account REGISTRATION`.

### File editing or shell access is unavailable.

Restart with the relevant launch grant after reviewing the task. Grants cannot be widened inside a running process.

### Pairing or recovery failed.

Check both endpoints' relay origin and account, a fresh invitation, host approval and `/remote status`. Follow [remote recovery](/docs/reference/remote-control/#credentials-and-recovery) and preserve the original endpoint identity.

### Keys or colors look wrong.

Use Ctrl+J for a newline, choose an explicit appearance or try `--no-color --plain`. Use `run` when a terminal is unavailable. Windows terminal behavior needs its own native verification.

## Reference and support {#reference}

Run `ejm-cli --help`, `ejm-cli COMMAND --help` or `/help` for the command contract. Read [Support](https://github.com/ejmlabs/ejm-cli/blob/main/SUPPORT.md) for reports and [Security](https://github.com/ejmlabs/ejm-cli/blob/main/SECURITY.md) for private vulnerability reporting. Exclude keys, tokens, private state and personal data.
