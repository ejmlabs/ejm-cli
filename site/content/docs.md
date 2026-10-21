## Choose your guide {#choose}

ejm-cli is an open source terminal coding agent under the [MIT license](https://github.com/ejmlabs/ejm-cli/blob/main/LICENSE), built on the public Rust ejm-harness API. Use it in a terminal for a conversation, or run one turn from a shell script.

### Use the CLI {#use}

The [user guide](/docs/user-guide/) takes you from a source build through provider setup, file permissions, saved sessions, and scripts. It also explains optional packages, relay accounts, and native control from another terminal.

### Build and contribute {#develop}

The [developer guide](/docs/developer-guide/) covers sibling checkouts, required tools, module ownership, deterministic checks, and the static website. Start there before changing the TUI or adding a command. Read the [contribution policy](https://github.com/ejmlabs/ejm-cli/blob/main/CONTRIBUTING.md), report a reproducible problem in the [issue tracker](https://github.com/ejmlabs/ejm-cli/issues), or propose a code or documentation change.

## Open the CLI {#first-run}

Build with Rust 1.99.1 and the recorded compatible ejm-harness revision beside ejm-cli. The [source setup](/docs/user-guide/#build) gives the full steps. From the CLI repository:

```sh Build and open the CLI
cargo build --release --locked
./target/release/ejm-cli --replay
```

The offline replay needs no account or model request. Configure an API provider or ChatGPT account before launching for model work; see [provider setup](/docs/user-guide/#providers). Help, diagnostics and saved-session inspection work without provider credentials.

## Find a task {#find}

- [Connect an API provider or ChatGPT](/docs/user-guide/#providers).
- [Choose a workspace and grant file access](/docs/user-guide/#grants).
- [Create, switch, and resume sessions](/docs/user-guide/#sessions).
- [Share a session with a native or browser controller](/docs/user-guide/#remote).
- [Manage relay accounts and recovery](/docs/user-guide/#remote-account).
- [Stream plain text or JSON in a script](/docs/user-guide/#scripts).
- [Resolve setup and terminal problems](/docs/user-guide/#troubleshooting).
- [Run the required contributor checks](/docs/developer-guide/#checks).

## Work with a remote session {#remote}

Share one selected session through an operator's HTTPS relay. Both endpoints need the same relay account, and the host approves the controller's full fingerprint locally. View access is the default; control access adds submit, steer, queue and cancel. Local CLI use needs no relay account.

Installed native endpoints encrypt session traffic through a forwarding relay. The relay can observe routing, timing and traffic size. Browser access additionally requires trust in operator-delivered code. Native source checks do not prove Worker/browser adoption or a live hosted service. Start with [remote setup and recovery](/docs/user-guide/#remote) and the [remote control reference](/docs/reference/remote-control/).

Read the approved [Relay signup and privacy notice](/docs/relay-notice/) and [service terms](/docs/relay-service-terms/). Their publication status and effective date are shown on each page. Preparing or publishing these policies does not open relay admission.

## Source, status, and limits {#status}

The [native acceptance reference](/docs/reference/platforms/) records accepted source and Linux/macOS checks. Windows terminal behavior, native distribution and live service acceptance have separate gates.

Build from the public [ejm-cli](https://github.com/ejmlabs/ejm-cli) and [ejm-harness](https://github.com/ejmlabs/ejm-harness) source. There is no supported binary installer or package-manager distribution. The [release policy](https://github.com/ejmlabs/ejm-cli/blob/main/RELEASING.md) defines publication gates.

> [!WARNING]
> **Native execution.** File tools begin read only. Write and shell access require explicit launch flags. Native tools and trusted packages run as your user and are not a sandbox.

Use the [CLI reference](https://github.com/ejmlabs/ejm-cli/blob/main/docs/ejm-cli.md) for detailed commands, the [contributor reference](https://github.com/ejmlabs/ejm-cli/blob/main/docs/contributing.md) for development checks, and [benchmarks](https://github.com/ejmlabs/ejm-cli/blob/main/docs/benchmarks.md) for performance methods and results.
