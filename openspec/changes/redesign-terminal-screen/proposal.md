# Proposal

## Why

The terminal screen spent two fixed lines on a header, printed raw JSON for tool calls, showed Markdown markup literally and gave no sense of progress beyond a status word. Ctrl+C exited at once when idle, so a stray press lost the session view. Current terminal agents set a clearer bar: a conversation that reads at a glance, quiet chrome and keys that forgive mistakes.

## What Changes

- Draw every transcript block with a gutter glyph, render light Markdown, and show tool calls as one line with their main argument and a collapsible output preview. Ctrl+O shows complete output.
- Add a working line with a spinner, step, elapsed time and the stop key, and a footer with context-sensitive key hints, the model and the effective grants.
- Show the command picker as aligned rows under the composer, and align the help panel the same way.
- Escape stops a run. Ctrl+C clears a draft and needs a second press to exit. `?` on an empty composer opens shortcuts.

## Capabilities

### New Capabilities

- `terminal-screen`: layout, transcript presentation and interrupt keys of the interactive TUI.

### Modified Capabilities

None.

## Shared blocks

The conversation blocks are owned by `ejm-ui`: `Message`, `ToolCall`, `Markdown` and `CommandMenu` joined its component contract with Rust (`crates/ejm-tui`) and Ink implementations, and `ActivityLine` gained step, elapsed time and stop key details. ejm-cli now depends on `ejm-tui` by path, deletes its copied `ejm_tui.rs` and `ejm_theme.rs`, and follows the contract's containerless rule: the composer box and welcome card became a hairline rule and the shared two-line header, and the help and session popups became panels in the transcript area.

## Impact

Changes `app`, `render`, `ui`, `commands`, the workspace manifest, the README, CLI guide, AGENTS map and theme skill. Building now needs the sibling `ejm-ui` checkout. `.github/ui-source.json` pins its commit, and the release build checks it out. Harness use, grants, plain and JSON modes, bounded memory and terminal restoration are unchanged. The ejmlabs design kit's `ui/TUI.md` still describes the old header and status line and needs a matching update in that project.
