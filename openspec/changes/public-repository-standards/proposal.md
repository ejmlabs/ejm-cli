# Proposal

## Why

The CLI has no branch protection, failed remote CI, missing public community guidance, and incomplete source setup and extension documentation. These gaps prevent a reliable public release.

## What Changes

- Adopt the shared workflows and standards in `ejmlabs/.github`, pinned to immutable commits.
- Require a human maintainer to admit each exact PR commit before quality checks or AI review can run, with bounded work and no automatic reruns.
- Require a PR, current maintainer approval, resolved conversations, and passing quality, standards, and two Codex review passes before merging.
- Allow a maintainer to approve their own current commit explicitly, without bypassing checks.
- Standardize contributor, security, support, license, documentation, and dependency maintenance guidance.
- Add independent semantic versions, release notes, version consistency checks, immutable tags, checksums, and a manual draft release pipeline.
- Preserve existing work and keep publication, package uploads, deployment, and paid security enrollment separate from setup.
- Correct cross-repository links, source status, package trust documentation, and Rust compatibility metadata.
- Teach the architecture gate about the six initialized OpenSpec workflows without relaxing project skill routing.
- Fix package grant arguments that collide with global provider and widget selectors, and add a parser regression before documenting them as usable.
- Follow the installed CLI setup hook with the shared completion guard: update both caller pins and the release standards reference from `ae903129564926a14287f75d2012610c59cccb65` to `c4f2873ef0eef5bc3c7d75cf71fa3489a911d134`, with exact old and new source evidence. Preserve the harness pin, setup action, product code, dependencies, credentials, and site.

## Capabilities

### New Capabilities

- `repository-governance`: Consistent contribution, maintainer review, security checks, and repository policy.
- `semantic-releases`: Independent SemVer releases with checked metadata and reproducible source artifacts.


### Modified Capabilities

None. OpenSpec was initialized for this work and has no existing capability specs.

## Impact

README, CLI reference, contributor docs, Cargo metadata, architecture checks, package argument parsing and its regression tests, workflow callers, and GitHub settings.
