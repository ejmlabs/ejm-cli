# Proposal

## Why

The installed CLI workflow tests a proposed harness pin and lockfile against the pin on trusted base main. Adopt the reviewed selector first so a later dependency update can test its exact submitted pin and matching lock against protected harness source authority.

## What Changes

- Advance the PR caller, draft-release caller and release `standards-ref` together from `c4f2873ef0eef5bc3c7d75cf71fa3489a911d134` to retained implementation `b2e43c9b9ba814e6d3681d86c88830f3c0b11a3c`.
- Use the shared exporter to provide both complete canonical 16-file implementations as bounded static review context.
- Update the CI adoption guide, contributor guidance and changelog with activation order, source authority and verification limits.
- Preserve `.github/harness-revision`, `Cargo.lock`, runtime, setup action, tools, credentials, owner rollout work and existing standards tasks.

## Capabilities

### New Capabilities

- `harness-source-selection`: Select and bind an admitted CLI dependency only after verifying its publication under protected official harness main.

### Modified Capabilities

None. The main capability inventory is empty; the existing standards change and its task history remain unchanged.

## Impact

The shared repository owns dependency selection and source archives. The CLI owns its two caller files, static context and affected documentation. The dependency direction remains CLI to public harness APIs and immutable shared workflow interfaces. No package, runtime API or dependency graph changes occur.

Original preparation started from official CLI main `4d7756749b8ee1c1e9e91a70eefd3e36f345379f`. After PR #7 head `3738509a467a1a8e6d7b04736c9f81ea35a9054c` passed its local and hosted checks, protected main advanced to `e1efe7b5c05d090ccfb164bef70456429030b9d0` through PR #6. A normal merge brings that main into the PR branch, preserving its consolidated publication guidance and deleted standalone guide. The updated source requires fresh local and exact-head hosted checks. The target implementation was protected-merged through central PR #16 as `49df106b82cfd703411b3595a82cd4d597ed0933`; root verified its exact protected retention. The caller PR still executes installed C4. Fresh installed-selector acceptance and any harness pin/lock update require later exact-head checks. No product behavior or public docs page changes, deployment or release are part of this change.
