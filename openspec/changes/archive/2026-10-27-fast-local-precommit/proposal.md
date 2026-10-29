# Proposal

## Why

Running the full gate at every commit repeats release, audit and terminal checks even when a developer only needs quick feedback. The admitted CI workflow already runs the full gate.

## What Changes

- Run quick staged whitespace, architecture and Rust formatting checks at pre-commit.
- Keep the full gate for local completion, CI and release, and update contributor and agent guidance.

## Capabilities

### New Capabilities

None. This is development tooling.

### Modified Capabilities

None.

## Impact

Changes the local hook and its documentation. Product behavior and hosted quality checks are unchanged.
