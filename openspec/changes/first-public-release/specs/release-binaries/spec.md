# Spec Delta

## Purpose

Build, check and describe the ejm-cli release binaries without giving project code write access or claiming signing that has not happened.

## ADDED Requirements

### Requirement: Read-only reproducible build and smoke
The release build SHALL run `cargo build --release --locked` on a native runner for each target at the harness revision in `.github/harness-revision`, with `contents: read` and no secrets. It SHALL archive the binary with `LICENSE` and `THIRD_PARTY_NOTICES.md`, record the SHA-256 of every uploaded file and upload nothing when the smoke check fails.

#### Scenario: Build succeeds
- **WHEN** the workflow runs on a main commit
- **THEN** each enabled target uploads one artifact holding the archive and a `SHA256SUMS` file naming every file

#### Scenario: Smoke fails
- **WHEN** `ejm-cli --version` does not print the `VERSION` value or `--replay run "Hello"` exits non-zero
- **THEN** the job fails and uploads no artifact

### Requirement: macOS waits for notarization
The macOS Apple silicon build SHALL be manual-only and SHALL NOT be offered for download until a notarization run for that exact release is recorded.

#### Scenario: No notarization record
- **WHEN** the release notes are drafted before notarization
- **THEN** they list no macOS download and say no macOS build is signed or notarized

### Requirement: Honest release notes
Release notes SHALL state the supported platforms, that builds are not signed with a developer certificate and not notarized, the checksum steps, the Linux runtime baseline and the issue-form links.

#### Scenario: Linux release notes
- **WHEN** a visitor reads the 0.1.0 notes
- **THEN** they name Linux x86_64 with glibc 2.39 and `libdbus-1`, explain how to check `SHA256SUMS`, and claim no signature

### Requirement: Intake forms carry version and platform
The bug and feedback issue forms SHALL ask for the version and platform, and the security policy SHALL point at the private advisory.

#### Scenario: Reporting a bug
- **WHEN** a person opens the bug form
- **THEN** it asks for version, platform, steps and result, and links the private advisory for vulnerabilities
