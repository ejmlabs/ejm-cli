# Design

## Context

See [the proposal](proposal.md). The deleted note describes an earlier source
extraction. `docs/ci-bootstrap.md` already holds protected merge receipts and
the current setup action. Its link and one relay design link target that note.
The contributor guide also uses its old title.

## Goals / Non-Goals

**Goals:** Keep source scope, the existing local gate, site limits, and inbound
links in the canonical CI guide while preserving each receipt's stated scope.

**Non-Goals:** Product code, workflow behavior, harness pins, site assets,
dependency resolution, and new acceptance or availability claims.

## Decisions

- Move useful guidance into a source-scope section in `docs/ci-bootstrap.md`
  and repair all inbound references before removing the one-off note.
- Retain exact protected-adoption receipts and their existing open checks.
  A PR title or a docs-only pass cannot establish another runtime result.
- Keep this a documentation-only change with `skip_specs: true`. Use existing
  architecture, OpenSpec, link, and whitespace checks; add no test dependency.
- Keep the CLI-to-harness public API direction and the separate `ejm-ui` site
  migration explicit. This cleanup adopts no new owner contract.

## Risks / Trade-offs

- Link migration can strand readers. Check every affected local link and the
  new source-scope anchor, including the existing relay design.
- Historical prep limits can look current. Separate current source facts from
  the dated receipts and leave unverified platform/release claims open.

## Migration Plan

Move the useful text, update links and the changelog, then remove the note and
verify the docs-only diff. Use the normal protected PR review afterward.
Reverting this docs commit restores the previous arrangement without a runtime
rollback or deployment.

## Local Verification

All 17 distinct affected local link targets resolved, including four anchors.
The existing architecture and skill checks, strict OpenSpec validation for all
three active changes, and whitespace checks passed. The relay design changes
only its old note link. No current guide still links to the deleted file.

The full native gate, CLI replay/Help execution, site build, browser checks,
hosted CI, AI review, deployment, release, and publication were not run for this
docs preparation. Existing receipts and open acceptance tasks keep their scope.
OpenSpec also reports the existing relay delta cannot be archived until its
missing main spec is reconciled; this cleanup does not alter that delta.
