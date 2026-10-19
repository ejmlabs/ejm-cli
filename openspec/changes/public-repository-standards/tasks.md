# Tasks

## 1. CLI readiness fixes

- [x] 1.1 Correct README source setup, links, status, and package trust/lifecycle reference; verify linked docs and commands match the implementation.
- [x] 1.2 Align Rust metadata and credential ignore rules; verify Cargo metadata and ignored credential paths.
- [x] 1.3 Permit the six initialized OpenSpec workflows while preserving CLI skill checks; verify the architecture gate accepts expected skills and rejects unknown ones.
- [x] 1.4 Fix package provider/widget grant argument collisions, preserve explicit trust and export validation, update examples, and verify focused parser and local lifecycle regressions without network calls.

## 2. Shared pipeline adoption

- [x] 2.1 Add uniform community docs, canonical version/changelog, and immutable shared workflow callers; verify shared standards and version checks.
- [x] 2.2 Apply available GitHub branch, approval, Actions, and dependency protections; verify API readback.

## 3. Verification

- [x] 3.1 Run the required CLI gate with at most two Cargo jobs, focused checks, workflow validation, and strict OpenSpec validation; record any unavailable evidence.
  - `./scripts/check` exited 0 on 2026-10-19, from `02:52:46.703300+00:00` to `02:54:51.150639+00:00`. Log: `/private/tmp/ejmlabs-public-readiness/cli-full-check-20261019T025246Z-e287e3b.log`.
  - Tested clean CLI revision `e287e3bbab83e4cc0d2eea94e61d0d819a6acf1f` against clean reviewed local harness revision `35205aa49b9261d133395c4d6116c37c456ecb95`. All six harness path dependencies resolved to `/private/tmp/ejmlabs-adoption/ejm-harness/crates/`.
  - Passed architecture, package boundary and skill checks; Rust formatting; strict workspace Clippy for all targets; 35 unit tests, including the package grant parser regression; 5 integration tests; release build; Rustdoc with warnings denied; cargo-machete; cargo-deny advisories, bans, licenses and sources; and actual PTY interaction and terminal restoration, including signal exit.
  - Platform: macOS `26.5.2`, Darwin `25.5.0`, arm64. Tools: Cargo `1.99.1` and rustc `1.99.1 (58cf8f9c3 2026-10-13)` from Homebrew, Python `3.15.0`, Node `24.19.0`, cargo-deny `0.20.2`, cargo-machete `0.9.2`, and OpenSpec `1.14.3`.
  - Resource settings: `CARGO_BUILD_JOBS=2`, `CARGO_INCREMENTAL=0`, `RUST_TEST_THREADS=2`, and `CARGO_TARGET_DIR=/private/tmp/ejmlabs-public-readiness/cli-target`. The target was an isolated APFS clone of the original ignored cache. The gate did not overwrite original binaries or delete files.
  - Shared standards, version and workflow validation passed through `/private/tmp/ejmlabs-standards/scripts/standards.py`; strict OpenSpec validation and `git diff --check` passed.
  - Remote limits: `.github/harness-revision` still pins `4a751c6a3f24d7d143140f79ff53517fb93e3531`. Hosted CLI and IDE quality checks remain blocked while harness is private and no private cross-repository token is provisioned. Update consumer pins only after the harness fixes merge to a reviewed commit. Linux, hosted checks and task 3.2 remain unverified. This local gate does not establish public readiness or release availability.
  - Public docs impact: none. This records local evidence and changes no product behavior or availability.

- [ ] 3.2 Verify human admission, self approval, stale commits, revocation, failed checks, missing secrets, and release tag refusal in GitHub; record live evidence before declaring public readiness.

## 4. Completion-guard caller follow-up

- [x] 4.1 Update both shared caller pins and the release `standards-ref` from `ae903129564926a14287f75d2012610c59cccb65` to `c4f2873ef0eef5bc3c7d75cf71fa3489a911d134`, preserving the working harness pin, setup action, runtime, dependencies, credentials, site, and separate feature work.
- [x] 4.2 Rotate the existing inert plaintext review bundle and manifest to both exact implementations; verify canonical commits, trees, source modes, blob IDs, SHA-256 digests, and bytes without accumulating the obsolete revision pair.
- [x] 4.3 Update directly affected contributor guidance and the changelog; verify pinned canonical standards and callers, actionlint, strict OpenSpec, affected docs links, whitespace, and the full staged diff limits. Record the focused evidence and unrun product checks.
  - Local preparation on 2026-10-19 starts at clean CLI main `189170a37b3efab4e842266cb09d8aeeeb4d5d1b`. The three caller references match the exact `c4f2873e...` consumer templates; its canonical standards/version validator, actionlint, strict OpenSpec, affected documentation links, and whitespace checks passed.
  - All 34 old/new source records match the canonical Git commits, trees, original modes, blob IDs, SHA-256 digests, sizes, and readable bytes. The 21 inert plaintext files total 462588 bytes, with no obsolete revision records or executable copies. Both retention refs match the selected revisions in the audited local mirror; remote protection still needs readback before admission.
  - The full staged Git patch fits the existing 200-file, 12,000-line, and 1,000,000-byte limits. Product, setup action, harness revision, dependency, credential, and site files remain unchanged. No native/site/full product gate, model call, hosted workflow, commit, push, admission, merge, publication, or release was performed in this preparation.
- [ ] 4.4 After ownership review, obtain fresh exact-head human readiness and approval, two actual completed reviews, all required checks, and normal protected adoption. Test the installed guard through a fresh admitted PR before claiming hosted completion enforcement for the CLI.

## Workflow follow-up

- Sync completed specs and archive after all tracked work and required verification succeed.
