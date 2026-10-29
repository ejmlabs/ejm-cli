# Tasks

## 1. Local checks

- [x] 1.1 Replace the full pre-commit run with staged whitespace, architecture and Rust formatting checks; run the hook and verify failure propagation.
- [x] 1.2 Update agent, dependency skill and contributor guidance to distinguish the hook from the full gate; check the documented commands.

## 2. Integration

- [x] 2.1 Run the unchanged full `./scripts/check` with at most two Cargo jobs and validate the change.

## Workflow follow-up

- Archive the verified tooling change.

The full gate passed locally, including strict Clippy, tests, release build, dependency checks and PTY interaction. The fast hook passed and propagated simulated whitespace, architecture and formatting failures.
