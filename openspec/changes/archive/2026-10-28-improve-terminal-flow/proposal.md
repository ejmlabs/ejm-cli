# Proposal

## Why

A review of the shared-block screen in replay at 60 to 110 columns found that the parts around the conversation still get in the way. Without a key the TUI never opens: it prints one line and exits. Chrome takes seven rows, including counts that read `0 agents · models 0+0q · tools 0+0q` at all times and `0 in / 0 out tokens` when nothing was counted. The transcript starts on the row under the header, and nothing marks where a turn ended. Every saved session is titled with the folder name, so the session list shows identical rows. One long command usage pushes every picker description to column 44, and the footer shows composer keys while the picker is open. `/status` starts with the warning mark when everything is ready.

## What Changes

- Open the TUI when the provider cannot be configured. The screen says it is not connected, why, and the three ways to connect. Prompts keep the draft and explain instead of starting a run.
- One header line, then a blank row. One two-column margin on both sides for every region, a blank row before the footer, and footer keys at the right edge. Agent, model and tool counts share the working line and show only while something is active or queued. Token counts show only when nonzero.
- Each finished run leaves one quiet line in the transcript: its result, steps, time and tokens.
- The session list titles each session with its first prompt and hides sessions with no turns, other than the current one.
- The command menu sizes its column for ordinary usages; a longer usage shortens only its own description. While the picker is open, the footer names the picker keys.
- `/status` opens as a panel of aligned rows. The shortcuts panel is labelled Shortcuts.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `terminal-screen`: layout, first run, turn summaries, session list and command picker.

## Shared blocks

`SessionHeader`, `WorkspaceLayout` and `ResourceLine` are Rust workspace recipes in `ejm-ui/crates/ejm-tui`, outside the shared contract, and ejm-cli is their only user. `CommandMenu` is in the contract, so its column rule changes in both the Rust and Ink targets. `Status` gains a `line` method so hosts can compose it with other spans.

## Impact

Changes `main`, `app`, `ui`, `render` and the remote backend's session lookup in ejm-cli, plus `workspace.rs`, `status.rs`, `conversation.rs` and the Ink `conversation.tsx` in ejm-ui. Harness use, grants, plain and JSON modes, bounded memory and terminal restoration are unchanged. `ejm-cli run` and the `sessions` subcommand keep their current output.
