# terminal-screen Specification

## Purpose

Keep the interactive screen readable at a glance while every state remains visible without color.

## Requirements

### Requirement: Visible grants

The TUI SHALL show the effective grants at every supported width, in the warning role when write or shell access is granted.

#### Scenario: Narrow footer

- **WHEN** the footer cannot fit key hints, the model and the grants
- **THEN** hints are dropped first, then the model name, and the grants remain

### Requirement: Tool call presentation

The TUI SHALL show each tool call as its name, main argument and a state glyph, followed by a bounded output preview that Ctrl+O expands.

#### Scenario: Long output

- **WHEN** a completed tool returns more lines than the preview shows
- **THEN** the preview ends with the number of hidden lines and the key that expands them

### Requirement: Forgiving interrupt keys

Escape and Ctrl+C SHALL stop an active run. When idle, Ctrl+C SHALL clear a non-empty draft, and exit only on a second press within two seconds.

#### Scenario: Stray Ctrl+C

- **WHEN** the user presses Ctrl+C once with an empty, idle composer
- **THEN** the footer asks for a second press and the session stays open

### Requirement: First run without a provider

When the interactive TUI cannot configure its provider, it SHALL open, say that it is not connected and why, and name the ways to connect. Submitting a prompt SHALL keep the draft and start no run.

#### Scenario: No API key

- **WHEN** `ejm-cli` starts in a terminal with no provider key
- **THEN** the TUI opens with the reason and the commands that connect it, and a submitted prompt stays in the composer

### Requirement: Quiet chrome

The TUI SHALL use one header line. Agent, model and tool counts SHALL share the working line and show only while a count is nonzero. Token counts SHALL show only when nonzero.

#### Scenario: Idle screen

- **WHEN** no agent, model request or tool is active or queued
- **THEN** no count line is drawn

### Requirement: Turn summary

Each finished run SHALL leave one line in the transcript with its result glyph and word, steps and elapsed time.

#### Scenario: Completed run

- **WHEN** a run completes in three steps
- **THEN** the transcript ends with `✓ completed · 3 steps`, and the working line is empty

### Requirement: Distinct session titles

The session list SHALL title each session by its first prompt and hide sessions with no turns, other than the current one.

#### Scenario: Two sessions in one folder

- **WHEN** two sessions in the same folder began with different prompts
- **THEN** each row shows its own prompt

### Requirement: Readable command picker

One long command usage SHALL NOT move the description column of other rows. While the picker is open the footer SHALL name its keys.

#### Scenario: Remote usage

- **WHEN** the picker lists `/remote share [view|control]|status|stop` among short usages
- **THEN** the other rows' descriptions start after the widest short usage
