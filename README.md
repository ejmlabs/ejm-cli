# ejm-cli

A Rust terminal coding agent built on [ejm-harness](../ejm-harness/README.md).
The CLI and harness are separate projects. ejm-cli demonstrates the same
public engine API available to community projects. No account
or server is needed.

## Try it

Check out `ejm-cli` and `ejm-harness` as sibling directories, then run:

```sh
cargo build --release --locked
./target/release/ejm-cli --replay
./target/release/ejm-cli --replay run "Hello"
```

`--replay` is an offline demo that echoes your prompt. For real coding work,
configure a hosted OpenAI compatible provider:

```sh
export EJM_CLI_MODEL=deepseek-chat
export EJM_CLI_BASE_URL=https://api.deepseek.com/v1
# Set EJM_CLI_API_KEY through your shell's secret handling.
./target/release/ejm-cli
```

Native file tools are read only by default. Add `--allow-write` to grant writing
and `--allow-shell` to grant shell execution. Grants stay visible in the TUI.
Keys come from environment variables and are never accepted as command arguments.

## Terminal experience

- Unicode composer, multiline input, bracketed paste, and sent-message history.
- Streaming transcript with code, Markdown, and diff styling.
- Visible run state and separate agent, model, and local tool counts.
- Saved sessions, steering, queued turns, and cancellation.
- Bounded rendering and scrollback, with durable history in the harness.
- Plain and JSON modes for pipes and automation.
- Terminal restoration on errors, panic, and handled signals.

Use `/help` for commands. Enter sends or queues, Ctrl+K steers, Ctrl+C stops,
and Ctrl+S opens sessions. `/agents` shows managed sessions.

```sh
ejm-cli run "Explain the parser" --json
ejm-cli sessions
ejm-cli --session SESSION_ID
ejm-cli doctor
```

See [the full CLI guide](docs/ejm-cli.md) for controls, configuration, and limits.

## Fast without flooding the host

Ratatui and Crossterm render the TUI. Rust and Tokio drive the harness directly.
There is no Node or Python runtime dependency for ejm-cli. Agents share a
resource governor instead of creating a shell each. The default envelope allows
100 managed sessions, eight model requests, four local jobs, and one heavy job.

Native execution runs as the current user and is not a security sandbox.
`ejm-cli doctor` reports actual enforcement capabilities. Hosted models, old-laptop
acceptance, and speed comparisons have distinct proof requirements. See
[benchmark evidence](docs/benchmarks.md).

## Develop

Read [AGENTS.md](AGENTS.md) and [the contribution guide](docs/contributing.md).
The project skills cover implementation, review, terminal behavior, performance,
and dependencies. Strict Rust all/pedantic checks, architecture checks, tests,
dependency policy, release builds, and real PTY checks run through one command:

```sh
./scripts/check
```

This source release has not been published or packaged. CI configuration is
included; remote checks and branch protection need a published repository and
a pinned ejm-harness revision. [MIT licensed](LICENSE).
