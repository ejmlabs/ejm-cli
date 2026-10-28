# Spec Delta

## Purpose

Keep the interactive screen readable at a glance while every state remains visible without color.

## ADDED Requirements

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
