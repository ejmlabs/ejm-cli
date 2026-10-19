# Proposal

## Why

The one-off publication prelude repeats source setup and verification guidance
already maintained in the CI guide. Consolidating it gives contributors one
current reference without discarding useful limits or protected-adoption
receipts.

## What Changes

- Move the useful source scope, local gate, and site-migration limits from
  `docs/command-foundation-publication.md` into `docs/ci-bootstrap.md`.
- Remove the old note and update its inbound links, including the reference in
  the existing relay design. Keep that design's accepted behavior and tasks.
- Record the docs cleanup in the changelog and verify local links, architecture
  guidance, OpenSpec, and whitespace.

## Capabilities

### New Capabilities

None. This is documentation-only and declares `skip_specs: true`.

### Modified Capabilities

None. The CLI, public harness contracts, and protected workflow behavior keep
their current implementation.

## Impact

`ejm-cli` owns these contributor guides and terminal interaction. `ejm-harness`
continues to own the engine's public runtime APIs; `ejm-ui` owns shared UI and
site contracts for the separate pending site migration. This cleanup changes
no dependency, public API, runtime, kit, workflow, license, or canonical review
source. It claims no new runtime test, hosted acceptance, release, publication,
or site deployment result.
