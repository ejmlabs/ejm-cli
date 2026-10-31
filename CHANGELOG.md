# Changelog

Changes follow [Semantic Versioning](https://semver.org/). Each repository is versioned independently.

## [Unreleased]

### CI

- Install the pinned wasm-bindgen CLI from its checksum-verified release archive instead of compiling it on every gate run.

### Website

- Move the site to ejm-ui `56301e3` and build it with the shared `ejmlabs()` Vite preset, the shared base TypeScript config and one stylesheet import. The site no longer lists the Tailwind and React build plugins, `resolve.dedupe`, TypeScript `paths`, the unused `@ejm/shots` link or the dead `budget` script. The build now also fails a page without exactly one h1, with a skipped heading level, without the skip link first or `main#main`, or with a link to another host that lacks `rel="external"`.

- Include the unchanged approved Relay privacy notice and service terms with
  explicit pending publication status and effective date.

- Rebuild the public CLI site and guides on shared UI exports, with direct source/contribution links, a labelled local recording, static documentation and unchanged route budgets. Keep the guides aligned with the accepted native runtime and preserve separate service and installer release gates.

### Fixed

- Reject a provider base URL that is not https, except http on a loopback host (`localhost`, `127.0.0.0/8`, or `::1`). The CLI checks this before it reads the API key. A rejected `run` exits 1 and the error does not print the URL. `doctor` marks a disallowed endpoint rejected. Userinfo, a query, or a fragment is rejected, and an endpoint line cannot carry a raw line break. An http loopback URL is also refused when `HTTP_PROXY`, `HTTPS_PROXY`, `ALL_PROXY`, or the lowercase form is set and `NO_PROXY` does not contain that host exactly. `*` and a bracketed IPv6 entry are not coverage. The error names the variable that was set and not its value. `.github/harness-revision` stays `42e4f9cf33bd3525aec344cfdffabc251271f782`. This change does not protect other ejm-harness callers. The pin bump waits until the harness https check is on main, and this note does not assign that harness release number.

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
