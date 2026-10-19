# Relay publication prelude

## Source status

This prepared source combines the CI bootstrap, terminal boundary and command
foundation on 2026-10-19. The combined scope stays below the admission caps
before lock resolution; remeasure the final locked source before publication.
The combination has not been compiled, runtime-tested, reviewed in hosted CI or
released. No test count or platform result from the active development checkout
applies to this candidate.

The source includes the owned terminal reader and mode guard, the prepared local
CI action, and command parsing, completion and Help selection. Provider
composition, account handling and CLI arguments retain baseline behavior. The
catalog lists only its implemented built-ins and granted package commands.
Remote control, provider switching and provider account commands inside the TUI
belong to later source layers.

The current reviewed harness pin and canonical central callers remain unchanged.
The supported order is this prelude, central hook adoption with that working
harness, then the reviewed new harness API pin and native remote consumers. See
[CI setup and protected adoption](ci-bootstrap.md) for the credential and trusted
base requirements.

## Dependencies and checks

The command foundation uses the existing Clap dependency to parse command names
and argument counts. Ratatui renders completion and Help through the existing
widgets and layout helpers. The command policy and App state are CLI product
behavior. It adds no Cargo package or feature over the terminal boundary. The
terminal packages, bounds, notices and required lock review are documented in
[Terminal input boundary](terminal-input.md).

The retained tests cover argument errors, free-form package input, colliding
command names, bounded Unicode replacement, completion without execution, draft
preservation, paste, history and short monochrome screens. The UI test adapter
uses the baseline public Harness constructor. These tests are prepared, not
recorded passes. The two direct `CommandEntry::usage` mappings retain behavior
and address strict Clippy without importing later remote fixture changes.

Resolve only the terminal additions against the current reviewed harness, then
run the existing architecture, formatting, strict Clippy, workspace tests,
release build, Rustdoc, dependency and PTY checks through `./scripts/check`.
For focused diagnosis use `cargo test -p ejm-cli --locked commands::`, then the
App, render and UI module filters. Keep Cargo jobs at most two. No paid provider,
live relay or browser fixture is needed for this prelude.

## Public documentation

The README and canonical CLI reference describe the candidate interaction. The
historical source has an authored `site/dist/index.html`, so its keys paragraph
accompanies this extraction. It has no `site/content` guide sources. When the
separate Vite/React site migration is adopted, retain this content in its guide
Markdown and render the canonical reference through that build. Do not copy or
hand edit that migration's generated `site/dist` output. The later site build,
affected page checks and publication remain separate evidence.
