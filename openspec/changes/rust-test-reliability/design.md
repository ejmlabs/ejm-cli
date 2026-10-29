# Design

## Context

The human-admitted gate runs the complete project check. This additional job is restricted to main and the daily schedule.

## Goals / Non-Goals

Detect inconsistent Rust outcomes without rerunning an unsafe build gate or making failures green. This does not replace cross-platform CI or track historical runs.

## Decisions

Run Cargo twice to retain shared-process behavior, and pinned cargo-nextest twice for its structured per-test JUnit results. The workspace has no library doctest target, so doctests are reported as not applicable. Keep errors and incomplete evidence failing.

Capture the PTY supervisor's output inside its test so that it cannot interrupt Cargo's result line. Include the captured output when the supervisor fails.

## Risks / Trade-offs

Nextest isolates tests in separate processes and cannot identify shared-process races by name. Cargo suite-level disagreement is reported separately. The two runs cannot prove absence of flakiness.
