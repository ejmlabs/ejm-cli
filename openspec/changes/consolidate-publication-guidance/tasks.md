# Tasks

## 1. Consolidate contributor guidance

- [x] 1.1 Move useful source scope, existing local checks, and site-migration limits into `docs/ci-bootstrap.md`; verify the gate and owner statements against current source without changing recorded acceptance.
- [x] 1.2 Remove `docs/command-foundation-publication.md`, update both inbound links and the contributor-guide title, and update the changelog; verify affected local links and the source-scope anchor resolve.

## 2. Verify the docs-only change

- [x] 2.1 Run the existing architecture check, strict OpenSpec validation, and whitespace checks; verify the diff preserves runtime, kits, workflows, locks, licenses, and canonical source evidence, and record checks left unrun.

## Workflow follow-up

- This docs-only change has no behavior spec delta to sync.
- Archive after local verification and the project's review requirements are satisfied.
- Use normal exact-commit human readiness and approval, both completed AI passes, and every protected check before merging.
- Publication, release, site adoption, and the existing governance acceptance tasks retain their separate gates.
