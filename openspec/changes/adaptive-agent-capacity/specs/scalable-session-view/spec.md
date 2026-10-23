## Purpose

Keep the terminal responsive and its working set bounded as managed agents and saved histories grow, while preserving ordered recovery and local and remote session authority.

## ADDED Requirements

### Requirement: Bounded foreground work

The CLI SHALL bound event, recovery, navigation, and presentation work before returning to input and painting. Sustained background activity SHALL not prevent Stop, cancellation, session navigation, or draft editing from making progress. Limits SHALL cover work already queued as well as newly arriving work.

#### Scenario: Continuous background activity

- **WHEN** many unselected agents produce continuous events while the user edits a draft or stops the selected run
- **THEN** the CLI continues processing input and painting the selected session
- **AND** it does not wait for the full background event backlog to drain

#### Scenario: Long saved history

- **WHEN** the selected session has a large durable history
- **THEN** opening or recovering the view uses bounded presentation data and returns to input between batches
- **AND** the durable history remains available through the harness

### Requirement: Session-scoped event delivery

The CLI SHALL consume selected-session events through a public filtered harness interface and use aggregate public status for background work. Switching sessions SHALL change the event scope without allowing a prior session's events or asynchronous view results to alter the new session or its draft.

#### Scenario: Unselected session flood

- **WHEN** thousands of other sessions emit events
- **THEN** the selected view is not required to receive and discard every other session's event
- **AND** the status still reports aggregate background work accurately

#### Scenario: Switch during recovery

- **WHEN** the user changes the selected session before an older recovery or page request completes
- **THEN** that completion cannot overwrite the newly selected session's view, cursor, or draft
- **AND** the CLI can cancel or discard work that no longer belongs to the selection

### Requirement: Ordered incremental recovery

The CLI SHALL recover a selected view from a bounded snapshot and ordered pages ending at a captured high-water mark. It SHALL preserve racing events, ignore duplicates, and verify selected-stream continuity using a public predecessor/token or explicit loss signal. Nonconsecutive global sequence numbers alone SHALL NOT establish loss.

#### Scenario: Event races with the snapshot

- **WHEN** selected-session events arrive while the CLI obtains its bounded snapshot
- **THEN** events represented by the snapshot are not applied twice
- **AND** events after the snapshot cursor are applied in durable sequence order
- **AND** the snapshot supplies the selected-session continuity anchor used by the stream handshake
- **AND** no accepted event is silently skipped

#### Scenario: Global sequences interleave without selected-session loss

- **WHEN** selected-session events have global sequences 10 and 14 while sequences 11 through 13 belong to other sessions
- **AND** the selected stream's predecessor/token confirms continuity with no explicit loss signal
- **THEN** the CLI applies both selected-session events in order
- **AND** the numeric jump does not trigger recovery or a missing-output warning

#### Scenario: Sustained output during catch-up

- **WHEN** the selected session continues appending events faster than the view can recover
- **THEN** the current recovery completes through its captured high-water mark in bounded batches
- **AND** input and painting remain available between batches
- **AND** any remaining lag starts another bounded recovery without an unbounded in-memory backlog

#### Scenario: Proven discontinuity or subscription overflow

- **WHEN** the public selected-stream predecessor/token identifies a discontinuity or the subscription reports explicit loss
- **THEN** the CLI resumes from an authoritative bounded snapshot or the last valid cursor
- **AND** it preserves the current draft and selected-session identity
- **AND** it does not treat partially recovered state as a fully current authority view

### Requirement: Paged session navigation

The interactive session picker SHALL use bounded pages and search through the public harness interface. Opening the picker, changing the query, and resuming an ID or prefix SHALL not require materializing every saved session. Matching and ambiguity decisions SHALL consider the full matching set rather than only the visible page.

#### Scenario: Browse a large session collection

- **WHEN** the user opens the session picker with thousands of saved sessions
- **THEN** the CLI loads and renders a bounded page around the visible selection
- **AND** the user can move through further pages without losing draft editing or input responsiveness

#### Scenario: Prefix matches outside the current page

- **WHEN** an ID prefix matches several sessions across different pages
- **THEN** the CLI reports the ambiguity
- **AND** it does not resume a session merely because that session is the only visible match

#### Scenario: Query changes before a page returns

- **WHEN** the user changes the search query while a prior page request is pending
- **THEN** only results for the current query and navigation generation affect the picker
- **AND** the current conversation draft remains intact

### Requirement: Safe bounded authority views

The CLI SHALL bound cached session presentation and authority views while preserving run identity, mutation ordering, share revocation, and drafts. Live, shared, selected, or in-flight authority entries SHALL remain valid until their protected use ends. If safe eviction is unavailable, admission to the cache SHALL remain bounded and report the condition without weakening authority checks.

#### Scenario: Visit more than one thousand sessions

- **WHEN** the user visits more than one thousand inactive sessions over time
- **THEN** evictable old views can be discarded and later rebuilt through public harness data
- **AND** the CLI does not permanently reject a session because it has seen many other sessions

#### Scenario: Remote mutation races with eviction

- **WHEN** a remote command or share-revocation operation is in flight
- **THEN** its authority state remains protected until completion
- **AND** eviction cannot reset mutation ordering, resurrect a revoked share, or attach a command to a different run

#### Scenario: Every cache entry is protected

- **WHEN** a bounded authority cache has no safely evictable entry
- **THEN** the CLI preserves the protected entries and existing authority checks
- **AND** it rejects or defers the new view with a recoverable explanation
- **AND** Stop and local draft editing remain available

### Requirement: Measured scale and responsiveness

The CLI SHALL validate one representative offline workload with 1000 total managed agents across two participating processes, bounded selected history, background events and cancellation. Evidence SHALL distinguish managed from executing work and retain latency, available resource observations, useful throughput, cleanup and limits. Claims SHALL be limited to measured workloads and systems.

#### Scenario: Compare capacity settings under one workload

- **WHEN** scale evidence compares automatic and fixed capacity
- **THEN** both runs use the same host, provider fixture, useful-work definition, and workload
- **AND** the evidence distinguishes active CLI overhead from the equivalent host workload without active terminal interaction
- **AND** it reports startup and steady-state results separately

#### Scenario: Evaluate terminal responsiveness

- **WHEN** the documented representative release-profile workload runs at 1000 total managed agents across two participating processes
- **THEN** its acceptance targets include p95 input and Stop acknowledgement at or below 50 ms and selected-session event-to-paint latency at or below 50 ms
- **AND** missed targets are reported with the measured hardware and workload rather than described as achieved

#### Scenario: Reuse evidence and bound additional measurements

- **WHEN** existing evidence covers an unchanged runtime and dependency path
- **THEN** that evidence may satisfy the relevant gate with a recorded source bridge
- **AND** a three-point safe fixed sweep and two alternating paired comparisons bound the representative performance work, with at most one additional pair for noise
- **AND** an unresolved result triggers investigation of its concrete cause rather than an open-ended test matrix

#### Scenario: Wider performance targets remain unmeasured

- **WHEN** the representative workload passes but broader scale, warm-startup, idle CPU, incremental memory, package-language or hardware targets have not been measured
- **THEN** those targets remain explicit follow-up work and are not described as passed
- **AND** they do not block that platform's activation unless a concrete safety or representative performance failure makes them relevant

#### Scenario: Bound claims about small systems

- **WHEN** benchmark results come from one development machine or deterministic local provider
- **THEN** docs identify those conditions
- **AND** they retain the separate four-core, 8 GB target until representative hardware and hosted-model evidence support it
