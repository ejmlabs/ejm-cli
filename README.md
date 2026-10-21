# ejm-cli

## Overview

A Rust terminal coding agent built on [ejm-harness](https://github.com/ejmlabs/ejm-harness).
The CLI and harness are separate projects. ejm-cli demonstrates the same
public engine API available to community projects. Offline replay needs no
account or server. Hosted models need provider access.

## Status

The CLI and harness source repositories are public. Build from compatible
sibling checkouts. There is no supported binary installer or package-manager
distribution. See [the release policy](RELEASING.md) for version and release gates.

## Requirements

- Rust 1.99.1, selected by `rust-toolchain.toml`.
- Git and sibling checkouts of `ejm-cli` and `ejm-harness`.
- A terminal for the TUI. Use `run` for pipes and automation.
- Provider credentials for hosted models. Offline replay needs none.

The core binary needs no Node or Python runtime. Optional code packages need
Node 24 or later, or Python 3.11 or later, for their selected runtime. See
[packages and trust](docs/ejm-cli.md#packages-and-trust).

## Quick start

Clone both repositories into the same parent directory:

```sh
git clone https://github.com/ejmlabs/ejm-harness.git
git clone https://github.com/ejmlabs/ejm-cli.git
cd ejm-cli
git -C ../ejm-harness checkout --detach "$(cat .github/harness-revision)"
cargo build --release --locked
./target/release/ejm-cli --replay
./target/release/ejm-cli --replay run "Hello"
```

`--replay` is an offline demo that echoes your prompt. The TUI needs a terminal.
Use `./target/release/ejm-cli` below, or add its directory to your `PATH`.

## Usage

For real coding work,
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
Native execution runs as the current user and is not a security sandbox.

### Terminal experience

- Unicode composer, multiline input, bracketed paste, and sent-message history.
  Up and Down recall sent messages outside the command picker.
- Streaming transcript with code, Markdown, and diff styling.
- Visible run state and separate agent, model, and local tool counts.
- Saved sessions, steering, queued turns, and cancellation.
- Bounded rendering and scrollback, with durable history in the harness.
- Plain and JSON modes for pipes and automation.
- Terminal restoration on errors, panic, and handled signals.

Type `/` to filter commands. Up/Down chooses a command, Tab completes it, and
Escape closes the picker without changing the draft. `/help` also lists granted
package commands; Enter copies the selected command into the composer. Enter
sends or queues a complete command or prompt, Ctrl+K steers, Ctrl+C stops, and
Ctrl+S opens sessions. `/agents` shows managed sessions.

```sh
ejm-cli run "Explain the parser" --json
ejm-cli sessions
ejm-cli --session SESSION_ID
ejm-cli doctor
```

See [the full CLI guide](docs/ejm-cli.md) for controls, configuration, and limits.

### Native remote access

Native CLI PR #21 is accepted at protected merge
`71d0f47f8b05c6314eca4dc0750ffaad833d6afb`; the
[acceptance record](docs/native-remote-publication.md) binds its tree and
Linux/macOS results. It uses the locked native graph and selects
protected harness merge `42e4f9cf33bd3525aec344cfdffabc251271f782`, the exact
358-file completion API tree `826e0231...`. A compatible separately operated
relay account service is required. Local use still needs no relay account.

Before first persistent host or controller use, stop older relay writers and
run `ejm-cli remote migrate-credentials`, including on fresh installations.
Confirm locally only after those processes have stopped. Then sign in with
`remote login`. `remote host` shares one session read-only; `remote host
--control` grants prompt, steer, queue and cancel actions. `remote connect`
reads the private invitation in a protected terminal. Compare the fingerprint on
both screens, then confirm pairing on the host. The host's tool grants still apply.

See [remote setup and recovery](docs/remote-control.md),
[dependency choices](docs/remote-dependencies.md) and the
[source acceptance and remaining release gates](docs/native-remote-publication.md). This native layer
contains no Worker, browser UI or deployment. It claims no formal crypto audit
or performance improvement.

### Resource limits

Ratatui and Crossterm render the TUI. Rust and Tokio drive the harness directly.
Agents share a resource governor instead of creating a shell each. The default envelope allows
100 managed sessions, eight model requests, four local jobs, and one heavy job.

`ejm-cli doctor` reports actual enforcement capabilities. Hosted models, old-laptop
acceptance, and speed comparisons have distinct proof requirements. See
[benchmark evidence](docs/benchmarks.md).

## Development

Read [AGENTS.md](AGENTS.md) and [the contribution guide](docs/contributing.md).
The project skills cover implementation, review, terminal behavior, performance,
and dependencies. Strict Rust all/pedantic checks, architecture checks, tests,
dependency policy, release builds, and real PTY checks run through one command:

```sh
./scripts/check
```

## Documentation

- [User guide](https://cli.ejmlabs.com/docs/user-guide/): source setup, providers, grants and recovery.
- [Developer guide](https://cli.ejmlabs.com/docs/developer-guide/): changes, ownership and site checks.
- [CLI reference](docs/ejm-cli.md): controls, providers, sessions, packages and trust.
- [Performance evidence](docs/benchmarks.md): measurements, method and limits.
- [Development guide](docs/contributing.md): tools, ownership and the required gate.
- [Release policy](RELEASING.md): versions and publication requirements.

### Native remote guides

These guides describe the accepted native source. Worker/browser adoption,
binary distribution and live acceptance remain pending:

- [Remote control](docs/remote-control.md): setup, controls, trust and recovery.
- [Remote dependencies](docs/remote-dependencies.md): package choices and dependency review.
- [Native remote source and review inputs](docs/native-remote-publication.md): native acceptance, the following inert UI/Relay prerequisite and remaining release gates.

## Contributing

Read [CONTRIBUTING.md](CONTRIBUTING.md) and the [code of conduct](CODE_OF_CONDUCT.md)
before opening a PR.

## Security

Follow [SECURITY.md](SECURITY.md) to report a vulnerability privately. Keep
provider keys and the state directory out of source control. Review native
package code before granting trust.

## Support

See [SUPPORT.md](SUPPORT.md) for questions and bug reports.

## License

[MIT](LICENSE). See [third-party notices](THIRD_PARTY_NOTICES.md) for bundled assets.
