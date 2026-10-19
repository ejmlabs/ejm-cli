---
name: ejm-cli-release
description: Prepare or review the first public release of ejm-cli, including the repository, CI variables, version, tag and the site's get section.
---

# ejm-cli release

ejm-cli has no public repository and no packaged release yet. Read [the contributor guide](../../../docs/contributing.md), `.github/workflows/ci.yml`, `tools.toml` and `LICENSE`.

Create a repository, push, tag, publish a crate, upload a binary or deploy new download text only when the user asks. Each one is outward facing and hard to undo.

Before a release is real, all of these must be true and shown by evidence:

- The sibling `ejm-harness` is public, and `EJM_HARNESS_REPOSITORY` and a reviewed full `EJM_HARNESS_REVISION` are set for CI.
- The `quality` job has passed on Linux and macOS on the remote, and branch protection requires it.
- `./scripts/check` passes at the tagged commit, and the version in the workspace `Cargo.toml` matches the tag.
- The release binary is `ejm-cli`. Performance numbers come from a fresh run of [the benchmark procedure](../../../docs/benchmarks.md) on that build, with their limits stated.
- The MIT license is present, and `cargo deny` licenses and advisories are clean.

After that, and only then, add the real repository URL and install steps to the README, [the CLI reference](../../../docs/ejm-cli.md) and the site's "Get" section, using `$ejm-cli-docs` and `$ejm-cli-site`. Until then those places must say it is a source release that has not been published.
