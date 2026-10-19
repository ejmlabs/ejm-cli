# Design

## Context

At the start of this audit, the projects were private and main had no branch protection. Product workflows ran on every PR with failing remote runs, and the two hosts required a sibling harness checkout. All products currently use version 0.1.0. See proposal.md for motivation.

## Goals / Non-Goals

**Goals:** Share maintenance policy and trusted automation, keep each product independently versioned, and require a human to admit and approve the exact PR head.

**Non-Goals:** Publishing product releases, package uploads, installer signing, deployment, history rewrites, and buying private-repository security products during setup.

## Decisions

1. Use the user-selected ejmlabs/.github repository for reusable workflows, executable standards, templates, and runbooks. Consumers pin a reviewed full SHA. Product checks keep their established gate commands.
2. Use trusted default-branch issue-comment or manual dispatch entry points. A human maintainer issues `/ready <full-head-sha>` before quality or AI runs. New commits have no inherited readiness. Outsider and bot events do not launch jobs. Restrict GitHub fork workflows to approval by all external contributors and keep untrusted jobs read-only.
3. Keep readiness separate from `/approve <full-head-sha>`. The latter requires a current human maintainer and records native bot approval for self review where GitHub forbids native author approval. Main requires at least one native approval and the maintainer gate, plus required checks, resolved discussions, linear history, and no force pushes or deletion. Administrators do not bypass the merge requirements.
4. Run exactly two independent Codex passes using `gpt-6.1-sol` with medium reasoning, trusted prompts and configuration, read-only permissions, no PR build or test execution, and isolated model credentials. Post results in a separate job. Missing credentials or invalid output fail closed. Bound the diff, output, time, and concurrent runs. Check replayed admission identifiers before spending budget.
5. Keep a root VERSION and CHANGELOG.md in each product. Validate all releasable package versions. A common manual pipeline accepts an exact checked main commit and matching version, refuses existing tags, and creates only a draft release with source archives, checksums, and provenance. Document native binary and registry publication gates explicitly.
6. Keep licenses and brand notices distinct. Preserve each product's MIT license and bundled font licenses; include license text in SDK packages. The studio's design assets retain their existing ownership terms.
7. Enable available free repository protections and dependency alerts now. Do not enroll private repos into billed secret protection. Record the required public-only activation checks before any visibility change.

## Risks / Trade-offs

- Shared workflow changes affect several products. Immutable caller pins and reviewed update PRs bound rollout.
- Human readiness costs a small metadata action and can delay outside contributions. The expensive checks remain unavailable until deliberate admission.
- During the initial rollout, CLI and IDE quality checks waited for public harness access under the owner's credential policy. Later CLI checkout authorization and its working source pin are documented in [CI setup](../../../docs/ci-bootstrap.md). This caller follow-up preserves that pin and credential path; IDE access and new harness API adoption remain separate.
- API review needs a scoped OPENAI_API_KEY and provider budget settings. Setup does not prove a successful paid review.
- Local disk space is constrained. Coordinate Cargo checks, use focused existing caches, and report missing full platform evidence honestly.
- Source and history scans reduce known risk; partial scan coverage and platform-specific tests remain explicit.

## Migration Plan

Create and test the shared repository first. Add pinned consumers and version metadata, then apply and read back repository settings. Run the project gates and first admitted PR through the new pipeline when the required credentials are present. Keep repositories private until blockers are resolved. Roll back a bad workflow update by restoring the previous immutable caller pin.

## CLI adapter

Keep ./scripts/check as the local and CI gate. Update its strict skill inventory to permit exactly the initialized OpenSpec workflows alongside the existing CLI skills. Align the stated Rust floor with the selected tested toolchain and make public source clone steps and package trust controls discoverable.

The package smoke check exposed a Clap argument-ID collision: global `--provider` selects a `ProviderKind`, while `package enable --provider` stores a list of export names and panics during conversion. Global and package `--widget` values can also mix. Use distinct `--grant-provider` and `--grant-widget` options and IDs for package grants. Preserve global host selectors and the other grant flags. Verify a single command containing both host selectors and package grants, then exercise the package lifecycle locally without a registry or provider request.

## Completion-guard caller follow-up

CLI main `189170a37b3efab4e842266cb09d8aeeeb4d5d1b` already contains the
reviewed setup action and pins shared implementation
`ae903129564926a14287f75d2012610c59cccb65`. Adopt combined implementation
`c4f2873ef0eef5bc3c7d75cf71fa3489a911d134` at the PR caller, release caller,
and release `standards-ref` together. It preserves the setup hook and requires
each actual AI result to contain JSON boolean `complete: true`. Missing,
nonboolean, or false completion fails validation, even with no findings.

Shared `.github` owns the workflows, result contract, and validator. The CLI
owns only its caller pins, inert review evidence, and contributor guidance.
The consumer continues to depend on the public shared workflow interface and
the same harness source pin. No runtime, setup action, lockfile, credentials,
site, or relay feature changes belong in this follow-up.

Rotate the existing plaintext review bundle in place to the exact installed
`ae903129...` and proposed `c4f2873e...` implementations. Include both generated
workflows, readable generator and runtime sources, review prompt and schema,
community templates, and their MIT notice. Record the source commits, trees,
modes, blob IDs, SHA-256 digests, sizes, and paths. Confirm both commits against
their protected canonical retention refs. These copies remain source data;
builds and the reviewer must not execute them. Remove obsolete revision records
while preserving unchanged blobs. Replacing the existing bundle with a second
full export layout would exceed the diff byte cap.

Check the entire staged patch against the existing 200-file, 12,000 added plus
deleted line, and 1,000,000 diff-byte limits, along with canonical callers,
standards, actionlint, documentation links, and strict OpenSpec validation.
Native, site, and full product gates are outside this local preparation.
The first adoption still runs through the old caller, so inspect both actual
reports for a completed review before normal protected adoption. A later fresh
admitted PR must verify hosted execution of the installed completion guard.
Keep live acceptance tasks open until their own evidence exists.
