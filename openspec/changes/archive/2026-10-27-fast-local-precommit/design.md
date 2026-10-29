# Design

## Context

See the proposal. The admitted Quality job at the pinned reusable workflow runs `./scripts/check` for the CLI on Linux and macOS.

## Goals / Non-Goals

Make local commits responsive without changing the full gate, CI, release checks, or product behavior.

## Decisions

Run staged whitespace validation and working-tree architecture and formatting checks locally. Keep Clippy, tests, release build, audits and PTY checks in the full gate before completion and in admitted CI. This avoids rebuilding at each commit while retaining full verification at the review boundary.

## Risks / Trade-offs

A commit can contain a regression detected only by the full gate. Document the fast hook as partial and require the full gate before completion and release. The hook still checks working-tree architecture and formatting, which may include unstaged edits.
