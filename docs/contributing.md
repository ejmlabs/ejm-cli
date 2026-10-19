# Contributing to ejm-cli

ejm-cli is a separate Rust project that consumes the public ejm-harness crates. Check out `ejm-cli` and `ejm-harness` as siblings. The CLI must build without source files from a Gerry platform product.

## Tools and the required gate

The versions in `tools.toml` are required. Use Rust 1.99.1 with Clippy and rustfmt, Python 3.11 or later, cargo-deny 0.20.2 and cargo-machete 0.9.2.

```sh
rustup toolchain install 1.99.1 --profile minimal --component clippy,rustfmt
cargo install --locked --version 0.20.2 cargo-deny
cargo install --locked --version 0.9.2 cargo-machete
./scripts/install-hooks
./scripts/check
```

The repository's `rust-toolchain.toml` selects Rust 1.99.1 when using rustup, without changing your global default. A Homebrew installation of Rust 1.99.1 also works. The gate checks the compiler's reported version. Tool installation and advisory database updates need network access. `Cargo.lock` is committed and every build in the gate uses `--locked`.

The complete gate validates architecture and skill routing, checks format, runs strict Clippy, runs deterministic tests, builds release binaries, checks Rust documentation, finds unused dependencies, checks licenses and advisories, and runs real PTY interaction/restoration tests. It is the entry point for local work, the pre-commit hook and CI. Build jobs are capped at two in `.cargo/config.toml`. `CARGO_TARGET_DIR` may share a local compilation cache.

The hook installer changes only this repository's `core.hooksPath`. It does not install a global hook. The quality gate runs on Linux and macOS. The trusted `.github/workflows/pr.yml` and `.github/workflows/release.yml` entry points read the reviewed full 40-character sibling harness commit from `.github/harness-revision`.

The workflows reject a missing or mutable harness revision. A private harness checkout needs a scoped read credential; public source does not. See [CONTRIBUTING.md](../CONTRIBUTING.md) for human admission and approval, and [RELEASING.md](../RELEASING.md) for remote checks and publication gates. The PTY gate checks interaction and terminal restoration on its host. Local macOS results do not prove Linux behavior. Windows terminal behavior requires separate evidence.

For the prepared CI action, source scope, and protected adoption order, see [CI setup and protected adoption](ci-bootstrap.md). It retains the current reviewed harness pin. Source preparation is separate from hosted product verification.

## Ownership and Rust design rules

| Module | Owns |
| --- | --- |
| `args` | Command parsing, environment selection and effective configuration |
| `auth` | Host browser launch, bounded loopback callback handling and safe account presentation through `ejm-harness-auth` |
| `browser` | Cancellable, timed OS URL launcher supervision for explicit sign-in |
| `app` | Bounded view state and durable-event application |
| `ui` | Input dispatch, harness calls and terminal lifecycle |
| `terminal` | Native mode capture/restoration and key-repeat policy |
| `terminal_input` | Owned bounded input parser, event adaptation and reset |
| `render` | Drawing and text layout |
| `main` | Process composition and plain/JSON entry points |
| Sibling ejm-harness | Shared authentication and credential storage, providers, run policy, context, tools, persistence and resource admission |

Keep each piece responsible for one coherent job. Extend real variable seams through the public harness traits. A provider or tool implementation must honor its contract, including cancellation, budgets and error reporting. Prefer small capability interfaces over a large “service” object. Compose the concrete app in `main`; pass the harness into the UI. Do not introduce a dependency injection container, factory hierarchy or trait for a single fixed renderer.

Use trusted, maintained packages for established infrastructure. Protocol parsing, cryptography and other standard concerns should use a reviewed package when one meets the requirements. Custom code should provide ejm-cli's distinct behavior, including its TUI interaction and app state. If a package cannot meet a verified requirement, document that gap before replacing it.

The CLI uses Hyper for loopback callback HTTP, `url` for URI validation and `open` for OS browser handlers. Alacritty's VTE parser removes control sequences from displayed output; keep its default features disabled so its OSC buffer stays fixed at 1 KiB. Termwiz owns terminal input parsing, with thin native mode and event adapters. See [Terminal input boundary](terminal-input.md) for limits, package choices and the candidate's open verification gates. Browser completion, cancellation and plain-text display policy remain CLI responsibilities.

These are Rust applications of single responsibility, substitutability, interface segregation and dependency inversion. Ownership, enums and `Result` should make illegal states and failure paths clear. A new abstraction needs a current second use or a documented variable boundary. Rendering must never launch a subprocess or contact a model provider.

## Strict lint policy

Rust warnings, Clippy `all` and Clippy `pedantic` are denied. Production code forbids unsafe code and denies `unwrap`, `expect`, explicit panic, unreachable markers, unfinished code markers, debug macros, process exits and forgotten allocations. Normal CLI stdout/stderr remains part of the product interface.

Fix the cause of a lint. Do not add workspace-wide allowances. An item-level `#[expect(..., reason = "...")]` is permitted only for a precise, reviewed exception. Unfulfilled expectations fail the build. Current intentional exceptions are independent boolean command switches in `Args`, and fixture `unwrap` calls within test-only modules. Neither exception applies to production state transitions or fallible setup.

The dependency checker denies unknown registries, unknown Git sources, wildcard versions, unapproved licenses, advisories and new duplicate-version families. `deny.toml` records exact-version exceptions for incompatible upstream requirements. Recheck those exceptions when updating the related dependencies; do not widen them to a whole crate family.

## Tests and review

Use replay or local HTTP fixtures. No paid provider calls belong in tests. Prove a changed user action or failure condition with a focused test before running the complete gate. Preserve failed drafts, durable event order, explicit grants, Unicode boundaries, bounded scrollback and prompt cancellation. Run the PTY check after terminal changes; a render snapshot alone cannot prove raw-mode restoration.

Record measured latency and memory with hardware, compiler profile and workload. The target is 100 managed agents with hosted models on four cores and 8 GB RAM. Do not claim that target from a single current-machine fixture.

## Project skills

The skills in `.agents/skills` route implementation, review, terminal and theme work, sign-in, packages, performance, testing, dependencies, the sibling harness, docs and claims, the marketing site, and releases. The six `openspec-*` skills handle exploration, proposals, plan updates, implementation, spec sync and archival. All feature and bugfix work follows OpenSpec because this project has an `openspec/` directory. `AGENTS.md` lists which skill fits each task. The gate checks each skill's frontmatter and references, that the exact set matches `scripts/check_architecture.py`, and that `AGENTS.md` routes to every skill. Keep them concise and specific to this project. Read only the skill that matches the task, and update the owning reference when a convention changes. When you add a skill, add it to the check script and to the table in `AGENTS.md`.
