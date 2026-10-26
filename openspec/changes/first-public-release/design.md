# Design

## Trust split

The build workflow never writes to the repository. Its jobs run project code (cargo build scripts and proc-macros), so they get `permissions: contents: read`, no secrets, pinned actions and `--locked`. The harness is checked out at the revision in `.github/harness-revision` with credentials not persisted. Attaching files to a draft release belongs to a separate shared workflow with `contents: write` that runs no project code and recomputes every digest. That workflow is a separate change in the `.github` repository, so this change only defines what it receives.

## Artifact contract

One workflow artifact per target, named `ejm-cli-<version>-<target>`, with these files at its root:

- `ejm-cli-<version>-<target>.tar.gz`, holding one top directory of the same name with `ejm-cli`, `LICENSE`, `THIRD_PARTY_NOTICES.md` and `HARNESS_REVISION`.
- `SHA256SUMS`, one `sha256  filename` line per file in the artifact, including the archive.

Targets are `x86_64-unknown-linux-gnu` and `aarch64-apple-darwin`. The version is read from `VERSION` and must equal the workspace version. The build records `GITHUB_SHA` in the job summary so the attach workflow can compare it with the tagged commit.

## Order and commit

Build and smoke run on the main commit that the draft release later tags. A failed smoke check fails the job and no artifact is uploaded.

## Linux baseline

The archive links `libdbus-1` dynamically and needs the build runner's glibc. The job builds on `ubuntu-24.04`, so the minimum is glibc 2.39 (Ubuntu 24.04 or Debian 13 or later). Debian 12 and Ubuntu 22.04 fail at launch with the loader's own message. Stored credentials need a session D-Bus and an unlocked Secret Service; without one the program still starts and `--replay` works.

## macOS waits for notarization

The macOS job exists so the build is proven, but it runs only from `workflow_dispatch` when a maintainer sets the `macos` input, and the release notes list no macOS download. Nothing says the build is signed or notarized until a recorded run exists.

## Smoke check

`ejm-cli --version` must print the `VERSION` value, and `ejm-cli --replay run "Hello"` must exit 0 with output. The job runs both against the unpacked archive in a fresh directory with an empty `HOME`.

## Issue forms

Forms carry a `version` and a `platform` field. Visitors reach them from the release notes by URL; nothing is prefilled with visitor data. The security link stays the private advisory URL.
