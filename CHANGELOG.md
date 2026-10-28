# Changelog

Changes follow [Semantic Versioning](https://semver.org/). Each repository is versioned independently.

## [Unreleased]

## [0.1.0] - 2026-10-27

First packaged build of ejm-cli, the terminal app for ejm-harness. See the [0.1.0 release notes](docs/release-notes/0.1.0.md) for install steps.

### Added

- Linux x86_64 release archive with `SHA256SUMS`, built from the pinned ejm-harness revision recorded in `HARNESS_REVISION`. It needs glibc 2.39 or later and `libdbus-1`; stored sign-in needs an unlocked Secret Service, and `ejm-cli --replay` works without one.
- Native remote hosting and control with Noise pairing, protected invitations, exact-run authority and a shared credential vault. Source acceptance covers Linux and macOS (see [native remote publication](docs/native-remote-publication.md)); live remote-control acceptance is still pending.
- Version and platform fields in the bug form, and a feedback form.

### Fixed

- Relay account metadata and credentials are published atomically in the shared vault, normal JSON cache writes stop, and newer logins survive logout.
- TLS advisory locking and native WSS root loading are corrected, and untrusted-peer coverage now includes WSS tickets.
- Snapshot authority follows its delivered cursor and checks expiry again after mutation waits.
- Each displayed pairing confirmation needs fresh approval, vault lock waits can be cancelled, snapshots recover after server gaps, and enrolled controllers can restart until the share expires.
- Local revocation stays responsive during completion observers, and SIGTERM restores the terminal.
- Display recovery stays ordered through journal catch-up, and newer live events wait until recovery completes.
- Accepted remote prompts wait for an exact-run completion, local-successor handoffs are discarded, and text above 16 KiB of UTF-8 is rejected before acknowledgement.

### Known limits

- The Linux build is not signed. A checksum shows the file matches what was published, not who built it.
- No macOS file is offered yet; the Apple silicon build waits for notarization. macOS Intel and Windows are not built.
- Native Linux acceptance for the shared process and credential code in ejm-harness is still open.
