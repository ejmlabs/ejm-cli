# Proposal

## Why

ejm-cli can only be built from source. The lab's first public release plan (the `first-public-releases` change in the ejmlabs repository) offers a Linux x86_64 download now and a macOS Apple silicon download later. This repository needs its own build and smoke jobs, release notes and feedback forms so that the shared draft release can attach checked files without running project code with write access.

## What Changes

- Add `.github/workflows/build.yml`, the shape the shared native build contract requires. A native Linux x86_64 job runs `cargo build --release --locked`, archives the binary with `LICENSE` and `THIRD_PARTY_NOTICES.md`, smoke-checks the archive and uploads it. A `collect` job writes one `build-sha256.txt` and uploads the single `release-files` artifact for the shared attach workflow in the `.github` repository. Workflow permissions are `contents: read` with no secrets.
- Leave macOS Apple silicon out of `build.yml`, with a marked placeholder, until it is notarized. The lab will not offer a macOS download until it has an Apple Developer membership and a notarization run for that exact release is recorded. `build.yml` runs on `workflow_dispatch` and on pushes to main.
- Record the Linux runtime baseline: the build runner's glibc floor and `libdbus-1`, plus a Secret Service for stored credentials.
- Draft release notes for 0.1.0 in `docs/release-notes/0.1.0.md`, stating supported platforms, that builds are unsigned and not notarized, the checksum steps and the issue-form links.
- Add `version` and `platform` fields to the bug and feedback issue forms, add a feedback form with the same fields, and keep `SECURITY.md` pointing at the private advisory.

## Out of scope

- Pushing, tagging, publishing, deploying or changing repository settings.
- The shared attach workflow, which lives in the `.github` repository.
- The ejmlabs.com download page, `install.sh` and `releases.json`.
- Windows and macOS Intel builds, signing and notarization.

## Impact

- Affected: `.github/workflows`, `.github/ISSUE_TEMPLATE`, `docs/release-notes`, `docs/contributing.md`, `CHANGELOG.md`, `scripts/package_release.py`.
- No runtime or API change. The version stays 0.1.0.
