# Design

## Context

See proposal.md for the dependency mismatch. Fresh official CLI main is `4d7756749b8ee1c1e9e91a70eefd3e36f345379f`, tree `3e53f9728afbbda2ee18bbd34e65c76896d1180a`. Both caller files and the release standards reference use `c4f2873ef0eef5bc3c7d75cf71fa3489a911d134`. The current harness pin is `4a751c6a3f24d7d143140f79ff53517fb93e3531`; the lockfile SHA-256 is `064f6266d334ea2239f6857a463c0d8d779f73dd84c14146b54794832ef00830`.

The old review bundle under `docs/ci/shared-workflow-review/` names ae903/C4. The shared exporter at exact b2 produces the complete C4/b2 pair from 16 canonical paths per revision. Those source bytes total 627715 before its manifest, below the 750000-byte export cap.

## Goals / Non-Goals

**Goals:** Adopt the already reviewed shared selector through immutable consumer refs, supply offline review context and preserve the dependency pair used by current CLI main.

**Non-Goals:** Changing shared selection code, product runtime, setup action, dependency pins or locks, owner rollout history, existing standards tasks, credentials, permissions, admission controls, deployments or releases.

## Decisions

1. Change the PR `uses`, release `uses` and release `standards-ref` together to `b2e43c9b9ba814e6d3681d86c88830f3c0b11a3c`. Use the retained reviewed implementation, not the merge SHA or a mutable branch. Root verified the protected retention after central PR #16 merged as `49df106b82cfd703411b3595a82cd4d597ed0933`.
2. Reuse `scripts/export_review_context.py` from exact b2. Export C4/b2 into a fresh `.github/review-context/` directory and verify all 32 records and bytes against Git. Keep ordinary nonexecuting modes and the shared MIT notice. Do not write a custom exporter, trim canonical files, fetch source during model execution or add dependencies.
3. Keep the old copies as dated historical data and defer their removal, as authorized by root. Early measurement of their removal produced a 1152231-byte patch without rename detection, above the 1000000-byte limit. Retain their index and shared MIT notice. The complete canonical C4/b2 pair is the current review authority; no canonical file is omitted to fit the cap.
4. Preserve `.github/harness-revision`, `Cargo.lock`, every runtime/config/tool/setup path and existing standards tasks byte-for-byte. Update only directly affected CI/contributor guidance and changelog, plus this plan and static context. No public product page changes are needed because commands, behavior and availability do not change.
5. State activation accurately. This PR's admission uses trusted C4 and the unchanged working dependency. After protected adoption, a fresh admitted CLI change can prove b2 selects the admitted pin and emits the dependency authority record. Later reviewed API pin plus lock adoption is a separate change. Preserve IDE's trusted-base behavior and draft-release semantics.

## Risks / Trade-offs

- A large context migration can exceed admission limits. Measure early and retain dated legacy copies if removal would exceed 1000000 diff bytes. Keep the complete new export below 750000 bytes and the whole PR below 200 files and 12000 added plus deleted lines.
- Branch ancestry proves source publication, not dependency behavior. Preserve full local and exact-head hosted product gates, actual complete AI reports and separate approval.
- Official main can move while preparation runs. The selector records exact protected main, selected commit/tree and archive digest; later movement cannot change the chosen revision.
- Old acceptance can be mistaken for new execution. Keep historical records dated and leave installed-selector acceptance pending until a new run proves it.

## Migration Plan

1. Confirm the fresh base, root's exact retention readback and immutable source pair. Complete this plan before apply.
2. Export the canonical pair, update all three references, measure admission scope and update the affected guidance. Keep the harness pin and lock unchanged.
3. Run focused canonical workflow/context, actionlint, strict OpenSpec, docs-link and whitespace checks. Hand root a frozen staged tree and bounded patch; root owns the normal full gate and publication.
4. Obtain fresh exact-head readiness, two complete actual reviews, all required Linux/macOS and aggregate checks, separate approval and protected adoption. Confirm unchanged protection.
5. Prove installed-selector source binding in a subsequent admitted change before claiming it active for a new dependency pair. A rollback uses an ordinary reviewed caller update to the prior immutable ref; it does not move retained refs or bypass gates.
