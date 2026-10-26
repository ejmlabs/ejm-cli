# Tasks

## 1. Build and smoke

- [x] 1.1 Add `.github/workflows/build.yml` in the shape the shared build contract requires: a Linux x86_64 build and smoke job and a `collect` job that uploads `release-files` with one `build-sha256.txt`. The macOS job is left out until notarization. Verify the YAML parses and nothing holds a write permission or secret.
- [x] 1.2 Build the Linux x86_64 release locally the same way and smoke it. Verify `--version` prints 0.1.0 and `--replay run "Hello"` exits 0.

## 2. Release notes and forms

- [x] 2.1 Add `docs/release-notes/0.1.0.md` and link it from `CHANGELOG.md` and describe the build in `docs/contributing.md`; `RELEASING.md` stays a byte-identical shared copy. Verify it claims no signing, notarization or macOS download.
- [x] 2.2 Add `version` and `platform` fields to the bug form, add a feedback form beside the existing feature form, and keep the private advisory link. Verify each form parses.

## 3. Pin and verify

- [x] 3.0 Pin `pr.yml` and the native `release.yml` to `091b6b7095de57b29344459ca690282cad73fa19`. Verify the shared standards check passes.
- [x] 3.1 Run `openspec validate first-public-release --strict` and the repository gate. Record checks left unrun.

## Workflow follow-up

- Sync the delta spec to `openspec/specs/release-binaries` and archive after the release is published.
- Re-pin to the merge SHA and regenerate `release.yml` if the shared pull request is squashed (see the design note on the shared pin).
- The shared attach workflow, tag, publication and site are separate owner-gated changes.
