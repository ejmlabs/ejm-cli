## MODIFIED Requirements

### Requirement: Command validation and status
The CLI SHALL report command-specific usage for missing or unexpected arguments, retain failed drafts and expose effective model, account, grants and separate agent, model and tool concurrency through `/status`. Help SHALL describe the same command catalog used by the picker. Free-form message tails SHALL retain their original punctuation, spacing and line breaks. Capacity status SHALL also expose mode, scope, targets, ceilings, queues, limiting reasons and observation availability.

#### Scenario: Invalid command arguments
- **WHEN** the user submits `/resume` without an ID or `/stop` with arguments
- **THEN** the CLI reports the matching command usage
- **AND** the draft remains editable

#### Scenario: Inspect effective state
- **WHEN** the user submits `/status`
- **THEN** the output reports the effective connection, session, grants and separate concurrency counts

#### Scenario: Inspect automatic capacity
- **WHEN** the user submits `/status` while automatic capacity is active
- **THEN** the output distinguishes managed agents, active and queued model and tool work, operating targets and hard ceilings
- **AND** it reports the shared host scope, current limiting reasons and available, stale or missing observations

#### Scenario: Inspect fixed capacity
- **WHEN** the user submits `/status` while fixed capacity is active
- **THEN** the output identifies fixed mode and its configured ceilings
- **AND** any lower effective admission is explained through the host, provider or other current constraint
