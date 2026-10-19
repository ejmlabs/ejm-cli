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

## Workflow follow-up

- Sync completed specs and archive after all tracked work and required verification succeed.
