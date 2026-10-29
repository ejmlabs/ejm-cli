---
name: ejm-cli-release
description: Prepare or review an ejm-cli release, including the version bump, changelog, release notes, release builds, the draft GitHub release, and install or download text in the README, docs and site.
---

# ejm-cli release

A release is a reviewed release PR, a green Build run on its merge commit, and a draft GitHub release that a human maintainer publishes. Read [the release policy](../../../RELEASING.md), [release binaries](../../../docs/contributing.md#release-binaries), `.github/workflows/build.yml`, `.github/workflows/release.yml` and `scripts/package_release.py`. `ejmlabs-workflow` holds the shared version, tag and draft release rules.

## Approval

- Prepare the release PR like any change, and carry it through merge.
- Running Draft release, publishing the draft, pushing a tag, uploading a file, publishing a crate and adding install or download text for a new release each need the user's explicit request. Draft release runs only for a current human maintainer: hand the user the exact command.

## Release PR

- Change `VERSION` and the `[workspace.package]` version in `Cargo.toml` together, and refresh `Cargo.lock`. `scripts/package_release.py` fails when they differ.
- Move the `[Unreleased]` entries in `CHANGELOG.md` into a dated `## [VERSION] - YYYY-MM-DD` section, with Added, Fixed and Known limits as in 0.1.0.
- Add `docs/release-notes/VERSION.md` in the shape of [the 0.1.0 notes](../../../docs/release-notes/0.1.0.md): harness revision, platform table, signing status, check and install steps, and report links that carry the version.
- `.github/harness-revision` holds the reviewed full 40-character ejm-harness commit, and `.github/ui-source.json` the ejm-ui commit that supplies `ejm-tui`. Each archive records the harness revision in `HARNESS_REVISION`.
- `./scripts/check` passes at the release commit, and every required status on the admitted PR is green.

## Build and draft

1. After the merge, `build.yml` runs on `main`. It builds Linux x86_64 with `--locked`, archives and smokes it through `scripts/package_release.py` (`--version` and `--replay run "Hello"` from the unpacked archive), checks the glibc 2.39 floor, and uploads `release-files` with `build-sha256.txt`.
2. Find that run: `gh run list --repo ejmlabs/ejm-cli --workflow build.yml --commit FULL_SHA --json databaseId,conclusion`.
3. A maintainer runs Draft native release with the full merge SHA, the version without `v`, and the run id:
   `gh workflow run release.yml --repo ejmlabs/ejm-cli -f commit=FULL_SHA -f version=VERSION -f build-run-id=RUN_ID`.
   It verifies the build, creates the immutable `vVERSION` tag and a draft release, and attaches the build files.
4. Review the draft's files, its checksums against `build-sha256.txt`, and its notes. The maintainer then publishes it.

## Claims

- Check `gh release list --repo ejmlabs/ejm-cli` and the tags before you say a version is released. A dated CHANGELOG section is not a published release.
- Linux x86_64 is the only built platform, and it is not signed. macOS Apple silicon joins `build.yml` and its `collect` job only after a notarization run for that exact release is recorded. macOS Intel and Windows are not built. Never call a file signed or notarized without that evidence.
- Performance numbers in release notes come from a fresh run of [the benchmark procedure](../../../docs/benchmarks.md) on the release build, with their limits stated.
- There is no crates.io release. It needs its own gate and credentials first.
- Add download or install steps to the README, [the CLI reference](../../../docs/ejm-cli.md) and the site only after the release is published and its files are verified, using `$ejm-cli-docs` and `$ejm-cli-site`. Keep source builds, draft releases and published files distinct.
