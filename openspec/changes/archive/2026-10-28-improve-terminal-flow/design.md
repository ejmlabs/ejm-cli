# Design

## First run

`run_agent` builds the provider as before. When that fails for the interactive TUI (no subcommand, both streams are terminals), it installs an empty `ReplayProvider`, which fails every request, and records the message in `Args::setup_needed`, a field clap skips. `App::connection_ready` keeps its meaning for the remote relay. The empty state then shows "Connect a model to start", three ways to connect (`ejm-cli login chatgpt`, provider environment variables, or `--replay`) and the reason. It stays above any notices while not connected. Submitting a prompt or `/queue` while not connected returns an error before any harness call, so the draft stays. The empty replay covers any other path, such as a remote controller, so no reply ever comes from a stand-in model. A custom provider would need `async-trait` as a normal dependency; the empty replay needs none.

## Layout

ejm-cli insets the whole screen by two columns on each side, so the header, transcript glyphs, prompt, rule and footer share one left and one right edge. `WorkspaceLayout` is header (1), blank (1), transcript, workers (optional), activity (1), notices, rule and draft, blank (1), controls (1). `SessionHeader` draws from its first column, without a selection mark's offset. `workspace_controls` puts the grants at the left and the hints at the right edge. The footer names at most three keys; `?` lists the rest. The command menu clears one row above itself. Panel titles are followed by a blank row. The separate resource row is gone: the host right-aligns `ResourceLine` in the activity row, and `ResourceLine` is empty when every count is zero. Counts read `2 agents · 1 model call, 3 waiting · 0 tools`, and fall back to the compact form when narrow. `SessionHeader::line` draws `name version`, the connection, and the directory at the right edge, cut from the start.

## Turn summary

`RunFinished` pushes a `BlockKind::Summary(StatusLevel)` block: completed is `Ok`, cancelled `Warn`, anything else `Error`. It draws as `Status::line` followed by muted details, such as `✓ completed · 3 steps · 4s · 1.2k in / 300 out tokens`. The idle working line then stays empty. The block is counted in the transcript's byte and block bounds like any other.

## Sessions

`open_sessions` reads at most four events of each listed session and uses the first `RunStarted` prompt, with whitespace collapsed, as the displayed title; the row cuts it to fit. A title given with `/new TITLE` stays. Sessions without one are hidden unless current. The stored title is unchanged.

## Command menu

The usage column is the widest usage of at most 24 columns, within 8 to half the width. A wider usage keeps up to half the width for itself and shortens only its own description. Rust and Ink apply the same rule.

## Labels

`/status` opens `Overlay::Status` with label and value rows under a `Status · Esc close` rule. The help panel's rule reads `Shortcuts`, and its command list says `Enter use`. The session panel names only non-idle states, beside the age.
