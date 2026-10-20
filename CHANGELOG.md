# Changelog

Changes follow [Semantic Versioning](https://semver.org/). Each repository is versioned independently.

## [Unreleased]

### Added

- Shared repository standards, explicit human admission, protected merge gates, two-pass Codex review, and a draft source release pipeline.

### Fixed

- Prepare the retained shared reviewer source-extraction update with exact b2/40d context and an isolated Linux CI Secret Service for existing persistent-account fixtures. Preserve the working harness pin, lock, provider and macOS setup. Fresh local, hosted and installed-reviewer checks remain pending.
- Prepare the shared review-completion guard with exact installed and proposed workflow source evidence, preserving the existing CLI setup hook and harness pin. Hosted adoption and installed-guard verification remain pending.
- Prepare the retained shared harness selector with complete canonical C4/b2 source context. Keep the current harness pin and lock unchanged. Normal product gates, protected caller adoption and installed-selector execution remain pending for this change.

### Changed

- Package commands default to `https://registry.ejmlabs.com`. `--registry` and `EJM_CLI_REGISTRY_URL` select another plain origin. `https` is required except on loopback. `ejm-cli package discard-retired-session` deletes a leftover meetgerry.ai session locally. The harness pin is unchanged.
- Consolidate source setup, local checks, and site-migration limits in the CI guide, replacing the obsolete publication prelude note and its inbound links.

## [0.1.0] - 2026-10-18

- Initial development baseline. This entry documents source version metadata; it does not assert a published release or production readiness.
