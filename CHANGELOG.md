# Changelog

Changes follow [Semantic Versioning](https://semver.org/). Each repository is versioned independently.

## [Unreleased]

### Added

- Shared repository standards, explicit human admission, protected merge gates, two-pass Codex review, and a draft source release pipeline.
- Adopt native remote hosting and control with Noise pairing, protected invitations, exact-run authority and a shared credential vault through protected CLI PR #21. Its [source acceptance](docs/native-remote-publication.md) covers Linux/macOS; live acceptance remains pending.

### Fixed

- Prepare automatic recipient follow-up on withdrawal and shared profile-expiry checks for individual and cohort disclosures. Protected acceptance and deployment remain pending.
- Prepare durable profile-retention scan progress, preserve deletion deadlines across withdrawal, opt-out and GPC, and honor live holds on unadmitted identities. The [retention reviews and corrections](docs/relay-publication.md#profile-retention-correction) remain subject to protected acceptance and deployment.
- Prepare bounded native relay retirement on ordinary stop and graceful exit, and authoritative browser recovery after Events gaps. Preserve reconnect reservations and uncertain actions. The [complete e0bf retry reviews](docs/relay-publication.md#native-retirement-and-browser-gap-correction) found both defects; corrected-source acceptance and deployment remain pending.
- Prepare atomic device revocation and credential deletion, and preserve every accepted Unicode device label in rights exports. Protected acceptance and deployment remain pending.
- Prepare the [device-history and quota corrections](docs/relay-publication.md#device-history-and-quota-review-corrections): keep active devices visible, page complete rights exports durably, bound cleanup scans and reject frames beyond a renewed reservation. Protected acceptance and deployment remain pending.
- Prepare the five [Worker review corrections](docs/relay-publication.md#worker-review-corrections): isolate replacement socket pairs, limit usage collection to consented fields, preserve original usage expiry, honor held-identity exceptions during deletion and track dashboard closure with a private one-time key. Both complete reviews rejected head cf1; the corrected source still needs its own checks and protected adoption.
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
- Adopt the [inert UI and Relay input prerequisite](docs/ci-bootstrap.md#current-inert-shared-ui-and-relay-inputs) through protected CLI PR #22 merge `78810826cd376588d3303f13bc237309fb5170a8`, tree `dca23befbbe5bf065807f8ed06a98aa5b4c7a159`. Exact head `f44c57396bc220562a8dae83767e188977e2c2ce` passed both platform gates and two complete reviews after the earlier metadata-only failure. It selects protected UI `79a1aa8b1075f834717bc0c0299dc2c41a22b966` with thirteen metadata bindings, the complete 28-path owner supplement and inert Relay lock/manifest review inputs. Its acceptance does not prove installation or runtime.
- Prepare the account-gated Worker/dashboard/browser source using only public ejm-ui components, active tokens and Relay artwork. Preserve accepted native behavior and add browser WASM, transport fixtures and the combined gate. Head cf1 passed its local stages and normal full gate, then failed both complete hosted reviews. Its corrected successor needs fresh acceptance before adoption. Updated deployment and policy publication remain pending; admission, billing and commercial collection stay off.
- Consolidate source setup, local checks, and site-migration limits in the CI guide, replacing the obsolete publication prelude note and its inbound links.

## [0.1.0] - 2026-10-18

- Initial development baseline. This entry documents source version metadata; it does not assert a published release or production readiness.
