# Changelog

Changes follow [Semantic Versioning](https://semver.org/). Each repository is versioned independently.

## [Unreleased]

### Added

- Build and smoke Linux x86_64 release files in the shared build contract shape with read-only jobs, leave macOS Apple silicon out until it is notarized, draft the [0.1.0 release notes](docs/release-notes/0.1.0.md), and add version and platform fields to the bug form plus a feedback form. No release, tag or download exists yet.
- Shared repository standards, explicit human admission, protected merge gates, two-pass Codex review, and a draft source release pipeline.
- Adopt native remote hosting and control with Noise pairing, protected invitations, exact-run authority and a shared credential vault through protected CLI PR #21. Its [source acceptance](docs/native-remote-publication.md) covers Linux/macOS; live acceptance remains pending.

### Fixed

- Publish relay account metadata and credentials atomically in the shared vault; stop normal JSON cache writes and preserve newer logins during logout. Protected PR #21 includes the correction and its exact-source acceptance.
- Correct the TLS advisory lock and native WSS root loading, reusing the existing certificate loader. Extend untrusted-peer coverage to WSS tickets; the patched native graph passed PR #21's gates.
- Correct snapshot authority through its delivered cursor and fresh expiry checks after mutation waits. The complete 061 review found both defects; d8's local macOS result remains historical alongside PR #21's later source acceptance.
- Require fresh approval after each displayed pairing confirmation, cancel vault lock waits, recover snapshots after server gaps and permit enrolled-controller restarts until share expiry. Both earlier f66 reviews blocked adoption on these defects; protected PR #21 contains the corrections.
- Retain responsive local revocation during completion observers, signal cleanup before protected startup, trusted local stop and real-command SIGTERM restoration. The f66 normal and Linux/macOS quality gates remain earlier evidence; PR #21 records the accepted successor.
- Adopt the retained 40d shared reviewer source context and isolated Linux CI Secret Service through protected PR #9. Its baseline Linux/macOS gates passed; installed 40d readable harness review and the native vault gate remain separate.
- Keep display recovery ordered through acknowledged journal catch-up, suppressing newer live events until recovery completes.
- Adopt immutable exact-run completion waits for accepted remote prompts, discard local-successor handoffs and reject text above 16 KiB of UTF-8 before acknowledgement. Protected PR #21 binds the selected 42e API and tested native graph.
- Adopt the shared review-completion guard through protected PR #4 while preserving the CLI setup hook and harness pin; retain its dated evidence in the CI guide.
- Adopt the retained shared harness selector and canonical C4/b2 source context through protected PR #7. The later native dependency pair has its separate PR #21 acceptance record.

### Changed

- Adopt the planned native remote-control guides and protected shared UI pin `24935aa85355ea99f49bdbecc4186a5e95049393` with complete install review context through CLI PR #20. That historical prerequisite retains C8's runtime; native PR #21 followed it. Hosted UI consumer acceptance and release remain separate.
- Prepare the [inert UI and Relay input prerequisite](docs/ci-bootstrap.md#current-inert-shared-ui-and-relay-inputs) on protected native merge `71d0f47f8b05c6314eca4dc0750ffaad833d6afb`, selecting protected UI `79a1aa8b1075f834717bc0c0299dc2c41a22b966`. Keep thirteen complete readable metadata bindings, reuse twelve verified fixtures, add the separate complete 28-path owner source supplement, and include the inert Relay lock with a complete candidate manifest and provenance. The earlier metadata-only head received two incomplete reviews; the successor still needs its own gates and protected adoption. No runnable Relay package or deployment is included.
- Consolidate source setup, local checks, and site-migration limits in the CI guide, replacing the obsolete publication prelude note and its inbound links.

## [0.1.0] - 2026-10-18

- Initial development baseline. This entry documents source version metadata; it does not assert a published release or production readiness.
