# ejm-cli

## Overview

A Rust terminal coding agent built on [ejm-harness](https://github.com/ejmlabs/ejm-harness).
The CLI and harness are separate projects. ejm-cli demonstrates the same
public engine API available to community projects. Offline replay needs no
account or server. Hosted models need provider access.

## Status

The project is being prepared for public use. Build from source with access to
both repositories. There is no supported binary installer or package-manager
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

`ejm-cli doctor` prints the package registry origin. The default is
`https://registry.ejmlabs.com`. `--registry` or `EJM_CLI_REGISTRY_URL` selects
another plain https origin, or http on a loopback host. Model commands ignore
that setting. See the registry section of [the CLI reference](docs/ejm-cli.md).

See [the full CLI guide](docs/ejm-cli.md) for controls, configuration, and limits.

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

- [CLI reference](docs/ejm-cli.md): controls, providers, sessions, packages and trust.
- [Performance evidence](docs/benchmarks.md): measurements, method and limits.
- [Development guide](docs/contributing.md): tools, ownership and the required gate.
- [Release policy](RELEASING.md): versions and publication requirements.

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
