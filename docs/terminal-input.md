# Terminal input boundary

## Prepared source

This terminal boundary is part of the combined relay publication prelude. It
changes the input reader and terminal recovery used by the existing TUI. Provider selection, account storage and run policy retain the baseline
behavior. No remote controller or private prompt is included in this layer.

The extracted candidate has not passed Cargo or PTY checks. Its `Cargo.lock`
still belongs to the baseline and must be refreshed only for the terminal
additions against the retained reviewed harness pin
`4a751c6a3f24d7d143140f79ff53517fb93e3531` before any locked gate or publication.
Do not copy the larger native remote dependency lock into this prelude. Earlier checks of a larger local
tree do not verify this candidate. Windows runtime evidence is also pending.

## Ownership and limits

`terminal_input::Input` owns one Termwiz parser and its event queue. The TUI
polls that reader from its existing timer and adapts events to the existing
Crossterm/Ratatui renderer. On Unix, safe Rustix reads use a fixed buffer with
`VMIN=0` and `VTIME=0`; no background input thread is started. Empty reads resolve
ambiguous Escape input and check for a terminal resize.

| Limit | Source contract |
| --- | --- |
| Native read batch | At most 1,024 bytes or Windows input records per poll |
| Retained input bound | At most 64 KiB before another feed is rejected |
| Parsed queue | At most 1,024 events |
| Split UTF-8 carry | At most three bytes |

The retained-byte counter is conservative because Termwiz does not expose its
internal buffer length. These are input admission bounds, not measurements of
total parser memory. An incomplete bracketed paste cannot grow without limit.
Invalid UTF-8 and input overflow return an error without including the input.
The UI then exits through its terminal guard. The existing composer limit is
separate from the reader's bounds.

A reset drops the parser, discards complete events, clears the UTF-8 carry,
flushes unread native input and reapplies the nonblocking input settings. Queued
paste strings and owned byte buffers use Zeroize. Termwiz's private parser
storage cannot be zeroized through its public API, so this does not promise
forensic erasure of process memory.

`terminal::Modes` captures the original native mode before raw input starts.
The terminal guard restores that mode, cursor, mouse capture, bracketed paste
and alternate screen on return and setup errors. Its panic hook flushes OS input
and restores modes without locking the parser. Held Windows action keys retain
their release state across input resets and guard replacement; ordinary editing
repeats still work. Windows Crossterm also retains native surrogate and mouse
state internally. This source does not claim to erase that private state.

## Package choice

The existing Crossterm renderer stays in place. Its Unix input reader keeps a
global parser with no public reset for partial input. Flushing the OS queue alone
cannot discard input already retained by that parser. Termwiz 0.23.3 supplies an
owned `InputParser`, allowing each input boundary to destroy partial state.

The password readers considered, rpassword and dialoguer, use blocking input
and do not provide this bounded, resettable event reader. Termwiz's full terminal
implementation is also unnecessary here. Rustix, Crossterm and WinSafe provide
the native calls; the custom code adapts events and enforces product limits.

Termwiz's released paste decoder assumes valid UTF-8 when advancing its raw
buffer. The adapter validates bytes before feeding it, carries a split code
point across reads and rejects malformed input. Focused regressions cover this
failure, fragmented escape sequences, complete and partial paste reset, native
key adaptation, held actions and ordinary navigation/scrolling.

The exact dependency pins, narrow duplicate-version exceptions and complete
[third-party license notices](terminal-third-party-notices.md) travel with this source. Termwiz adds dependencies
even with its default features disabled. Resolve and inspect that graph before
publication. No startup, binary-size or speed improvement is claimed; collect
comparable release measurements before making one.

## Verification required before publication

With the retained reviewed harness pin checked out at `../ejm-harness`, first
refresh the candidate's lockfile through the normal Cargo resolution process.
Review the
resulting graph and license exceptions, then run the unchanged required gate:

```sh
./scripts/check
```

The gate includes strict Clippy, deterministic tests, a release build and the
real PTY contract. For a focused repeat after that build:

```sh
cargo test -p ejm-cli --locked terminal::tests
cargo test -p ejm-cli --locked terminal_input::tests
python3 scripts/check_terminal.py "${CARGO_TARGET_DIR:-target}/release/ejm-cli"
```

The PTY check exercises normal interaction and SIGTERM, and compares every
original terminal attribute after exit. Its supervisor performs one zero-timeout
readiness poll before comparison: macOS can otherwise retain `PENDIN` until the
terminal is polled. It does not read or mask a flag. Passing on one host does not
establish another host's behavior; hosted Linux/macOS and Windows checks remain
separate evidence.

The user-facing [CLI reference](ejm-cli.md) records the input limits and recovery.
The baseline public page's composer and clean-exit descriptions remain accurate;
this parser extraction adds no flag or new public workflow. The separate site
migration is excluded. No live-site publication is established by this source
preparation.
