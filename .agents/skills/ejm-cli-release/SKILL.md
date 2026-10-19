---
name: ejm-cli-release
description: Prepare or review the first public release of ejm-cli, including the repository, CI variables, version, tag and the site's get section.
---

# ejm-cli release

ejm-cli source is maintained at `https://github.com/ejmlabs/ejm-cli`. Check current visibility and release evidence before making publication claims. Read [the contributor guide](../../../docs/contributing.md), [the release policy](../../../RELEASING.md), `.github/workflows/pr.yml`, `.github/workflows/release.yml`, `tools.toml` and `LICENSE`.

Create a repository, push, tag, publish a crate, upload a binary or deploy new download text only when the user asks. Each one is outward facing and hard to undo.

Before a release is real, all of these must be true and shown by evidence:

- The sibling `ejm-harness` is public, and `.github/harness-revision` contains its reviewed full 40-character commit SHA for CI.
- The current required quality, standards and review checks have passed on the remote, including Linux and macOS quality jobs, and branch protection requires them.
- `./scripts/check` passes at the tagged commit, and the version in the workspace `Cargo.toml` matches the tag.
- The release binary is `ejm-cli`. Performance numbers come from a fresh run of [the benchmark procedure](../../../docs/benchmarks.md) on that build, with their limits stated.
- The MIT license is present, and `cargo deny` licenses and advisories are clean.

Source checkout and build instructions may link the real repositories before a packaged release exists. Add binary downloads or package-manager install steps to the README, [the CLI reference](../../../docs/ejm-cli.md) and the site's "Get" section only after that release is verified, using `$ejm-cli-docs` and `$ejm-cli-site`. Keep source availability, draft releases and published artifacts distinct.
