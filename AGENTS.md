# ejm-cli

ejm-cli is the Rust terminal app for ejm-harness, made by ejmlabs. The harness lives in the independent sibling project `ejm-harness`.

## Names

| Thing | Name |
| --- | --- |
| Public page | "cli by ejmlabs" at cli.ejmlabs.com |
| Program, crate, binary | `ejm-cli` |
| Environment variables | `EJM_CLI_*`, for example `EJM_CLI_MODEL` and `EJM_CLI_APPEARANCE` |
| State directory | `ejm-cli` |
| Engine | `ejm-harness`, at `../ejm-harness`, public interface only |
| Sibling desktop app | `ejm-ide`, at `../ejm-ide`, not part of this repo |

## Rules

- Never use em dashes. Write plainly, in sentence case.
- Package manifests use `ejm.package.json`. Registry operations require an explicit origin; local packages need no registry. Keep unrelated proprietary identity and defaults out of this project.
- Use only the harness's public interface. Keep model and tool execution out of the TUI.
- Use `ejm-harness-auth` for sign-in, credential storage and refresh. Only the CLI host opens a browser after binding its loopback listener. Never accept or display bearer tokens through arguments, logs or the TUI.
- Use trusted, maintained packages for general infrastructure. Write custom code for ejm-cli's distinct product behavior, such as TUI interaction. Document the package alternatives and evidence for any unmet need.
- Keep rendering and scrollback bounded. Slow output must not grow memory without limit.
- Restore terminal state after errors, cancellation, and panic.
- Treat terminal control sequences in model and tool output as untrusted text.
- Report model, agent, and tool concurrency separately.
- Keep Cargo jobs at most two. No paid provider calls in tests.
- Never claim performance, security or compatibility the repo cannot prove. Native execution is not a sandbox.
- Run formatting, clippy with warnings denied, and focused tests before finishing.

## Project map

| Path | Holds |
| --- | --- |
| `crates/ejm-cli/src` | `args`, `auth`, `browser`, `app`, `ui`, `render`, `extensions`, `packages`, `main`, plus the kit modules `ejm_theme` (generated, never hand edited) and `ejm_tui` |
| `crates/ejm-cli/tests/cli.rs` | Integration tests that run the real binary against `--replay` |
| `docs/` | `ejm-cli.md` reference, `benchmarks.md` evidence, `contributing.md` gates and ownership |
| `scripts/` | `check` (the one gate), `check_architecture.py`, `check_terminal.py`, `install-hooks`, `sync-design-kit.sh` |
| `site/` | The static cli.ejmlabs.com page and its Cloudflare Worker config |
| `.agents/skills/` | Project skills, listed below |

## Commands

```sh
./scripts/check                      # the full gate used by local work, the pre-commit hook and CI
cargo test -p ejm-cli NAME           # a focused test
cargo run -p ejm-cli -- --replay     # the TUI with the offline demo provider
```

## Required workflow

Read the matching project skill under `.agents/skills` before starting. Read only the skills the task needs, and read `docs/contributing.md` for ownership and gates.

| Task | Skill |
| --- | --- |
| Implement commands, session interaction or TUI behavior | `ejm-cli-change` |
| Review a patch | `ejm-cli-review` |
| Keyboard input, wrapping, scrolling, paste, terminal restoration | `ejm-cli-terminal` |
| Colors, light and dark appearance, glyphs, `NO_COLOR`, the ejmlabs kit | `ejm-cli-theme` |
| Sign-in, accounts, credentials, browser launch, loopback listener | `ejm-cli-auth` |
| Package commands, trusted native extensions, grants, package themes | `ejm-cli-packages` |
| Latency, memory, many managed agents | `ejm-cli-performance` |
| Tests, replay fixtures, render and PTY checks | `ejm-cli-testing` |
| Cargo dependencies, tool pins, quality tools, CI | `ejm-cli-dependencies` |
| The sibling `ejm-harness` crates, lockfile refresh, other projects | `ejm-cli-harness` |
| README, docs, help text, any claim | `ejm-cli-docs` |
| The cli.ejmlabs.com page and its deploy | `ejm-cli-site` |
| Repository, tags, CI variables, first public release | `ejm-cli-release` |
| Explore a feature or bugfix before planning | `openspec-explore` |
| Propose a change and its plan | `openspec-propose` |
| Revise an existing change plan | `openspec-update-change` |
| Implement planned tasks | `openspec-apply-change` |
| Sync completed requirements to the main specs | `openspec-sync-specs` |
| Archive a verified, completed change | `openspec-archive-change` |

- Run `./scripts/check` before reporting a change complete. Local, pre-commit and CI use this command.
- Rust warnings and Clippy all/pedantic are denied. Unsafe code is forbidden. Production unwrap/expect, explicit panic, unfinished markers, debug macros, manual process exit and forgotten allocations are denied.
- Fix lint causes. Exceptions require a narrowly scoped expectation with a concrete reason. Do not weaken a workspace rule to pass a check.
- Use ownership, enums, small modules and explicit `Result` paths. Traits belong at real variable seams. Do not add a dependency injection container or factories for a single implementation.
- Keep Cargo.lock and tool pins current. New dependencies require boundary and license review.
- This project has an `openspec/` directory. Follow its workflow for all feature and bugfix changes: explore, propose, apply, update the plan as needed, sync specs and archive verified work.

## Working here with other agents

- Other agents may be editing this repo or its siblings at the same time. Run `git status` and `git log` before you start, change only the paths your task needs, and never revert changes you did not make.
- Never edit `../ejm-harness` or `../ejm-ide` without the owner's agreement. If a sibling breaks the build, tell its session and wait.
- Commit, push, publish and deploy only when the user asks. The pre-commit hook runs the full gate against the working tree.
- Builds are large. When a disk fills up, run `cargo clean` in this repo and rebuild. Never delete files that belong to other projects.

## Done means

The change works for a user, `./scripts/check` passed (say if you did not run it), docs and `--help` match the behavior, and the report states what was verified and what was not.
