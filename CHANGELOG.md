# Changelog

Changes follow [Semantic Versioning](https://semver.org/). Each repository is versioned independently.

## [Unreleased]

### Added

- Shared repository standards, explicit human admission, protected merge gates, two-pass Codex review, and a draft source release pipeline.

### Fixed

- Reject a provider base URL that is not https, except http on a loopback host (`localhost`, `127.0.0.0/8`, or `::1`). The CLI checks this before it reads the API key. A rejected `run` exits 1 and the error does not print the URL. `doctor` marks a disallowed endpoint rejected. Userinfo, a query, or a fragment is rejected, and an endpoint line cannot carry a raw line break. An http loopback URL is also refused when `HTTP_PROXY`, `HTTPS_PROXY`, `ALL_PROXY`, or the lowercase form is set and `NO_PROXY` does not contain that host exactly. `*` and a bracketed IPv6 entry are not coverage. The error names the variable that was set and not its value. `.github/harness-revision` stays `4a751c6a3f24d7d143140f79ff53517fb93e3531`. This change does not protect other ejm-harness callers. The pin bump waits until the harness https check is on main, and this note does not assign that harness release number.
- Prepare the retained shared reviewer source-extraction update with exact b2/40d context and an isolated Linux CI Secret Service for existing persistent-account fixtures. Preserve the working harness pin, lock, provider and macOS setup. Fresh local, hosted and installed-reviewer checks remain pending.
- Prepare the shared review-completion guard with exact installed and proposed workflow source evidence, preserving the existing CLI setup hook and harness pin. Hosted adoption and installed-guard verification remain pending.
- Prepare the retained shared harness selector with complete canonical C4/b2 source context. Keep the current harness pin and lock unchanged. Normal product gates, protected caller adoption and installed-selector execution remain pending for this change.

### Changed

- Consolidate source setup, local checks, and site-migration limits in the CI guide, replacing the obsolete publication prelude note and its inbound links.

## [0.1.0] - 2026-10-18

- Initial development baseline. This entry documents source version metadata; it does not assert a published release or production readiness.
