# ejm-cli

ejm-cli is the Rust terminal app for ejm-harness. The harness lives in the independent sibling project `ejm-harness`.

- Never use em dashes. Write plainly.
- Use only the harness's public interface. Keep model and tool execution out of the TUI.
- Use `ejm-harness-auth` for sign-in, credential storage and refresh. Only the CLI host opens a browser after binding its loopback listener. Never accept or display bearer tokens through arguments, logs or the TUI.
- Use trusted, maintained packages for general infrastructure. Write custom code for ejm-cli's distinct product behavior, such as TUI interaction. Document the package alternatives and evidence for any unmet need.
- Keep rendering and scrollback bounded. Slow output must not grow memory without limit.
- Restore terminal state after errors, cancellation, and panic.
- Treat terminal control sequences in model and tool output as untrusted text.
- Report model, agent, and tool concurrency separately.
- Keep Cargo jobs at most two. No paid provider calls in tests.
- Run formatting, clippy with warnings denied, and focused tests before finishing.

## Required workflow

Read the matching project skill under `.agents/skills`: `ejm-cli-change` for implementation, `ejm-cli-review` for reviews, `ejm-cli-terminal` for terminal lifecycle/input, `ejm-cli-performance` for latency or memory, and `ejm-cli-dependencies` for tools/dependencies/CI. Read `docs/contributing.md` for ownership and gates.

- Run `./scripts/check` before reporting a change complete. Local, pre-commit and CI use this command.
- Rust warnings and Clippy all/pedantic are denied. Unsafe code is forbidden. Production unwrap/expect, explicit panic, unfinished markers, debug macros, manual process exit and forgotten allocations are denied.
- Fix lint causes. Exceptions require a narrowly scoped expectation with a concrete reason. Do not weaken a workspace rule to pass a check.
- Use ownership, enums, small modules and explicit `Result` paths. Traits belong at real variable seams. Do not add a dependency injection container or factories for a single implementation.
- Keep Cargo.lock and tool pins current. New dependencies require boundary and license review.
- If an `openspec/` directory is added by the user, follow its workflow for all feature and bugfix changes. Do not initialize OpenSpec without a request.

