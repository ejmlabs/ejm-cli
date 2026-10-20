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

- Package commands default to `https://registry.ejmlabs.com`. `--registry` and `EJM_CLI_REGISTRY_URL` select another plain origin. `https` is required except on loopback. `doctor` shows a rejected registry value without userinfo, a query, a fragment, or control characters, and uses `(invalid registry URL)` when the value does not parse. The harness pin is unchanged. `discard-retired-session` is not a user instruction until it ships in the same release as the pin bump to the merged harness pull requests 18 and 19.
- Refuse HTTP loopback registry access on the current harness pin when any uppercase or lowercase `HTTP_PROXY`, `HTTPS_PROXY`, or `ALL_PROXY` variable is nonempty. `NO_PROXY` does not override this temporary guard. Unset those variables for that invocation or use HTTPS. Rejection happens before account-token lookup and prints no proxy value. `doctor` keeps the rest of its report; model commands and local discard remain usable.
- Consolidate source setup, local checks, and site-migration limits in the CI guide, replacing the obsolete publication prelude note and its inbound links.

## [0.1.0] - 2026-10-18

- Initial development baseline. This entry documents source version metadata; it does not assert a published release or production readiness.
