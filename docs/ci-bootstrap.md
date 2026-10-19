# CI setup and protected adoption

## Source scope and local checks

The current source includes the owned terminal input reader and mode guard,
command parsing, completion, and Help selection for implemented built-ins and
granted package commands. Clap parses command names and argument counts;
Ratatui renders the picker and Help. These remain CLI interaction and bounded
view state. `ejm-harness` owns provider transport, authentication mechanics,
tools, and run policy through its public APIs. The
[native remote source layer](native-remote-publication.md) adds host and
controller interaction through those public APIs. Worker/browser source and
provider or account switching inside the TUI belong to later source layers.

Use the reviewed harness revision in `.github/harness-revision` and the locked
dependency graph. Review any dependency or lock change with its owning source;
do not copy a larger feature branch's lockfile into a smaller source change.
The [development guide](contributing.md) records required tools and the full
gate. [Terminal input](terminal-input.md) records package choices, bounds,
notices, and platform limits. Keep Cargo jobs at most two.

From the repository root, run the existing product gate:

```sh
./scripts/check
```

It checks architecture, format, strict Clippy, workspace tests, a release build,
Rust documentation, dependencies, and real PTY interaction and restoration.
For command-parser diagnosis, use
`cargo test -p ejm-cli --locked commands::`, followed by the affected App,
render, or UI test filters. Local replay and fixtures need no paid provider,
live relay, or browser session. Each result applies only to its tested source
and host; the dated receipts and open acceptance checks below retain their
separate scope.

The current public page is authored at `site/dist/index.html`; this source has
no `site/content` guide tree or site build step. `ejm-ui` owns shared visual and
site contracts for the separate pending site migration. When that migration
lands, retain the command guidance in its authored guide sources and build the
CLI reference through its normal docs pipeline. Keep generated output and
private sibling implementations out of source changes. Site adoption, browser
checks, and publication require their own evidence.

## Adopted setup hook and completion guard

The combined CI bootstrap, terminal input boundary and command foundation
merged through CLI PR #2 as protected main commit
`8a603af45504579ed1234bfe92a34b33367e44bd`. Its tree
`cc2bc0f0330fb8464cca4060886fc2f1e4be9297` equals the reviewed prelude head
`c3fddcb5bfd2b6b9485e91d4eaa2ec682ade37ee`. It includes the local setup action
needed by the next central hook. This is not an available relay or a claim of
measured CI savings.

CLI PR #3 then adopted the scoped shared setup hook as protected main
`189170a37b3efab4e842266cb09d8aeeeb4d5d1b`, with tree
`08da201dcc40a3911ace424948246010342d6876`. Its PR caller, release caller,
and release `standards-ref` pin
`ae903129564926a14287f75d2012610c59cccb65`.

CLI PR #4 adopted combined shared implementation
`c4f2873ef0eef5bc3c7d75cf71fa3489a911d134` at those three references.
Shared PR #14 adopted that retained implementation as protected main
`d00d8bbc7231485eb918008c562d835f394e82ea`. It preserves the CLI setup hook
and Linux harness prerequisites, and rejects an incomplete AI result even with
no findings. Each pass must return JSON boolean `complete: true`. The earlier
`5309137` and `0adffdf5` targets remain obsolete history.

Central PR #11 passed its required checks and exact-head human approval, then
merged through protected `main` as
`8c304c8cdfc8b6fe50f3e442507215e615944f2b`. The merge and reviewed head share
tree `91b7f5c0990e27f2e0fdfcce26aac18afdcf6b81`. The exact reviewed
implementation is retained at
`refs/heads/workflow-pins/ae903129564926a14287f75d2012610c59cccb65` under
active update/delete rule `25369476`, with no bypass actors. Protected main
rules were unchanged in that receipt. CLI PR #3 completed the setup-hook adoption.
PR #4 installed the completion guard, but its adoption run still used the old
schema. Fresh execution was pending at that cutoff; the PR #7 receipt below
records later complete reports through installed C4. See the
[adoption receipt](#cli-pr-4-adoption-receipt). The earlier
caller revision was `5fae851502c2e3c31d4df9a44f8d56c2cb49e098`. That caller-only
adoption kept working harness pin
`4a751c6a3f24d7d143140f79ff53517fb93e3531` unchanged. The following
[native source preparation](native-remote-publication.md) records its separate
protected API dependency and unverified consumer graph.

The [deployment change](../openspec/changes/deploy-private-remote-relay/proposal.md)
tracks the full release. All its tasks remain open in this source preparation.
At that caller-only cutoff, the update added no product code or new harness API
consumer. Its action, locks, provider composition, account storage and CLI
arguments matched its protected parent. See the
[source scope and local checks](#source-scope-and-local-checks) and
[terminal boundary](terminal-input.md) for the earlier source layers.

## What the action prepares

The composite action at `.github/actions/setup-ci/action.yml` uses the pinned
`actions/setup-node` revision and installs Node 26.12.0, npm 11.20.0, the Rust
1.99.1 WebAssembly target and wasm-bindgen CLI 0.2.129. Cargo installation uses
two jobs. Linux setup installs `pkg-config` and `libdbus-1-dev` for the later
protected native credential service.

The admitted workflow exports the CLI source under `product/`. When
`product/relay/package.json` exists, the action runs `npm ci --ignore-scripts`
and the locked Playwright command `npm exec --no -- playwright install
--with-deps chrome` from the relay directory. Before relay source lands, that
one step is skipped. A relay manifest without its lockfile fails installation.
Tool installation needs network access and the hosted runner's package manager.

The prior `5fae851502c2e3c31d4df9a44f8d56c2cb49e098` caller did not invoke
this action. The `ae903129564926a14287f75d2012610c59cccb65` caller
added its exact local-action step only for the admitted `ejm-cli` profile in the
credential-free quality job, after Rust setup and before `./scripts/check`.
The installed `c4f2873ef0eef5bc3c7d75cf71fa3489a911d134` implementation retains
that exact hook.
The source release continues to use the same central release workflow, with
its `uses` and `standards-ref` pointing to the same revision. Product permissions,
triggers, secret forwarding and human controls are unchanged.

The caller-only cutoff used Rust and Python and kept the working harness pin.
The following native layer adds public API consumers and their PTY gate; it
still excludes Worker/browser source.
The hook installs Linux D-Bus build prerequisites before the later API pin
adoption; the browser install step skips only when the relay manifest is absent.
A manifest without its lock still fails.

`./scripts/check` remains the product quality gate. The later relay source change
must add its locked dependencies, tests and gate integration together. Run and
review a fresh clean-checkout gate for each proposed source snapshot.

## Protected adoption order

1. Confirm the reviewed local action is on trusted CLI `main`. The prelude
   receipt above records its protected merge; retain its exact action, working
   harness pin and lock through this prerequisite.
2. Retain the exact old/new implementation pair reviewed for PR #4 in the
   [source evidence](ci/shared-workflow-review/README.md):
   `ae903129564926a14287f75d2012610c59cccb65` and
   `c4f2873ef0eef5bc3c7d75cf71fa3489a911d134`. The three installed caller
   references now match the latter revision.
3. PR #4 completed normal protected adoption with the working harness pin
   unchanged, hosted Linux and macOS quality results, and two actual completed
   AI reviews. Its run used the old validator. Complete the separate
   [fresh installed-guard acceptance](#fresh-installed-guard-acceptance) before
   claiming hosted execution of the new completion contract.
4. Adopt the retained harness selector through the three caller references,
   keeping the working harness pin and lock unchanged. Follow the
   [selector adoption order](#protected-harness-selector-adoption) below and
   prove its installed execution in a subsequent admitted change.
5. Complete protected review of the required new harness APIs. After the
   setup hook and selector are installed, adopt the full reviewed API commit
   and its matching consumer lockfile through a protected change. Retain the
   private credential scope and expiry requirements below.
6. Publish native remote API consumers against that reviewed dependency pair,
   then the bounded Worker/browser source. Run hosted Linux
   and macOS quality checks for each exact admitted head. Retain the protected
   merge and release checks.

Readiness may be posted under the maintainer's explicit delegation. Approval
of each exact head requires its separate explicit human answer. Admission is
tied to a non-draft PR targeting `main`; each new head needs fresh decisions.
The installed C4 workflow reads `.github/harness-revision` from trusted base
`main`. Until selector adoption, a passing pin-only PR does not prove the new
API consumer pair. After adoption, the selector can test an admitted pin and
matching lockfile together, subject to the protected-source checks below.

The prepare job exports `product.tar` under `product/` and the selected private
harness under `ejm-harness/`, preserving the workspace's `../ejm-harness` paths.
The quality job receives source archives and has no provider credentials or
write token. A private dependency checkout needs a dedicated, expiring Contents:
read credential for `ejmlabs/ejm-harness` in `EJM_HARNESS_TOKEN`. Successful source
preparation does not verify the credential's scope or expiry. Validate those
separately through protected secret handling. Keep broad workstation credentials
local.

The current central review bounds are 200 changed files, 12,000 added plus
deleted lines, and 1,000,000 diff bytes per PR. Measure this caller prerequisite
against protected main and remeasure every later committed diff.
Keep generated build output, dependency caches, private environment files and
the separate site migration out of these source changes.

## Protected harness selector adoption

Original preparation started from official CLI main
`4d7756749b8ee1c1e9e91a70eefd3e36f345379f`. PR #7 head
`3738509a467a1a8e6d7b04736c9f81ea35a9054c` then passed the normal local
gate and hosted run
[`39309188976`](https://github.com/ejmlabs/ejm-cli/actions/runs/39309188976),
including both actual complete AI reports with no findings. Protected main
advanced to `e1efe7b5c05d090ccfb164bef70456429030b9d0` through PR #6 before
selector adoption. The normal merge preserved that publication-guide
consolidation and the standalone guide deletion. Those earlier results remain
historical; the completed PR #7 receipt below binds the later revision.
Before PR #7, protected main's three shared references used
`c4f2873ef0eef5bc3c7d75cf71fa3489a911d134`. PR #7's PR caller,
draft-release caller and release `standards-ref` advanced together to
`b2e43c9b9ba814e6d3681d86c88830f3c0b11a3c`. Central PR #16 protected-merged
the reviewed implementation as
`49df106b82cfd703411b3595a82cd4d597ed0933`. Before this source preparation,
root verified the exact
`workflow-pins/b2e43c9b9ba814e6d3681d86c88830f3c0b11a3c` ref and its active
update/deletion blocks with no bypass actors. Read back that full retained ref
and protection again before admission.

The complete old C4/new b2 pair is in the
[canonical manifest](../.github/review-context/manifest.json). The shared
exporter copies all 16 required source paths per revision and records their
full commits, trees, source modes, Git blob IDs, byte counts and SHA-256
digests. Its 33 files total 636783 bytes, below the 750000-byte context cap.
They are untrusted source evidence; the workflow callers, product gate and
runtime do not execute them. The older ae903/C4 documentation bundle remains
as [dated history](ci/shared-workflow-review/README.md), with its shared MIT
notice. Keeping those files avoids a deletion/addition patch above the
admission byte limit when rename detection is unavailable.

For CLI changes, the installed selector reads `.github/harness-revision` from
the exact admitted Git commit. It accepts a full lowercase 40-character SHA
only from an ordinary small Git blob. It then uses bounded, redirect-free
GitHub reads for the fixed `ejmlabs/ejm-harness` repository. The selected commit
must equal or be an ancestor of the exact reported protected `main`. Missing,
malformed, unprotected, inaccessible or divergent authority fails preparation.
The checkout must match the verified commit and tree. The admitted context
records that identity, the protected-main authority and source archive SHA-256.
The existing credential-free quality job builds that exact source archive
with the admitted CLI source and lockfile. Both independent static reviewers
receive the same authority context. Their model steps receive no checkout
credential. IDE selection keeps its trusted-base behavior. Draft release
invocation and permissions stay unchanged; its generated payload includes the
updated shared preparation library.

The caller-only change kept `.github/harness-revision` at
`4a751c6a3f24d7d143140f79ff53517fb93e3531` and preserved `Cargo.lock`
byte-for-byte. It changed no product runtime, dependency version, setup action,
tool pin, credential, permission or admission/approval gate. The owner's
public standards tasks and rollout records remained unchanged.

Run the normal local gate and exact-head hosted Linux/macOS Quality and
standards checks for this caller change. Require both actual independent
reports to state `complete: true`, all aggregate statuses, separate exact-head
approval and normal protected merge. PR #7 ran through installed C4;
its results cannot establish hosted execution of b2. After protected adoption,
a fresh admitted CLI change must confirm that b2 records the actual selected
commit/tree, protected-main authority and archive digest used by quality.
Only then claim installed-selector acceptance. Later API pin and matching
lockfile adoption remain separate from this caller change and may provide
that execution evidence through their own full gates. Local product gates,
hosted acceptance and protected adoption of the caller are recorded below.
The later installed-selector check remains pending. No product page, relay
deployment, release or site publication was part of that caller-only change.

### CLI PR #7 selector adoption receipt

On 2026-10-19, [CLI PR #7](https://github.com/ejmlabs/ejm-cli/pull/7)
protected-squash merged approved head
`dd0b1fb72e399623458debef8bc28554407c789a` as
`61aa4b4b137180ba6d76210b2cb5321cdc59658c`, tree
`504892783c5b2b49db490582f4a11a782b8542df`, with sole parent
`e1efe7b5c05d090ccfb164bef70456429030b9d0` and unchanged protection.
[Fresh run `39317164563`](https://github.com/ejmlabs/ejm-cli/actions/runs/39317164563)
passed Ubuntu/macOS quality, standards and required technical statuses; both
actual independent reports state `complete: true` with no findings. Exact-head
human approval and native approval then passed before protected merge.
That caller source still used harness `4a751c6a...`; it does not prove the
following native API pair. The native revision must prove actual B2 selection
of protected harness `220305fe...`, its tree and archive digest in a fresh run.

## Verification and publication record

The initial caller preparation checked canonical pins, the exact retained
central validator, actionlint, OpenSpec, documentation links and whitespace.
The normal commit hook then passed for CLI head
`92e423b67243b15e97a43c5ba4776057172d242c`. Its two admitted AI reports
explicitly said review was incomplete because the old and new external workflow
implementations were absent. Successful wrapper jobs and zero findings did not
complete that review or establish acceptance.

The [shared workflow source evidence](ci/shared-workflow-review/README.md)
supplies deduplicated exact plaintext copies of both implementations and their
readable source for the old `ae903129...` and adopted `c4f2873e...` pair.
The manifest includes full revisions, trees,
original paths and modes, Git blob IDs, and SHA-256 receipts. These files are
review data and add no execution authority. PR #4 changed the three caller
references; runtime, tools, locks and the working harness pin remained
unchanged.

### CLI PR #4 adoption receipt

On 2026-10-19, [CLI PR #4](https://github.com/ejmlabs/ejm-cli/pull/4) merged
normally as `d5113beb9c42565e6dc124052dbd633d0d12d25d`. Its tree
`f98ae57edd69a8a298616a1151a9473ba822b1cf` exactly matches reviewed head
`07116d5e1ead7c006f185f26951736b02449fbba`. All five required statuses and the
native approval passed; protection was unchanged, with no administrator bypass
or settings exception.

The [admitted run](https://github.com/ejmlabs/ejm-cli/actions/runs/39293429847)
used trusted base `189170a37b3efab4e842266cb09d8aeeeb4d5d1b` and shared
implementation `ae903129564926a14287f75d2012610c59cccb65`. Ubuntu 24.04 and
macOS 15 each passed the required gate, including 58 unit tests, 5 integration
tests, the release build, Rust documentation, dependency checks and actual PTY
interaction/restoration. Repository standards passed for version `0.1.0`. The
separate registry job succeeded but performs no registry checks for the CLI.

Both independent `gpt-6.1-sol` sessions used medium reasoning. Their actual
reports matched the reviewed head and pass numbers, stated completed static
scope, and had no findings. The old schema has no `complete` field. These
reports establish completed static review for adoption; they do not establish
hosted enforcement of the installed `complete: true` contract.

### Fresh installed-guard acceptance

Task 4.4 in the existing
[OpenSpec change](../openspec/changes/public-repository-standards/tasks.md)
remains open. Use a fresh PR containing only this adoption receipt and acceptance
record, based on protected main with the installed `c4f2873e...` callers. No
workflow, setup action, harness pin, runtime, dependency or canonical review
source changes are needed.

1. After ownership review of its exact head, obtain separate human readiness
   and approval for that full SHA through the existing admission process. A new
   head needs new decisions. Do not reuse PR #4's admission or rerun its jobs.
2. Read the actual new run, both report artifacts and both reviewer transcripts.
   Confirm execution of canonical
   `c4f2873ef0eef5bc3c7d75cf71fa3489a911d134`, including its result validator.
   Each report must match the new head and its distinct pass number, contain
   JSON boolean `complete: true`, and describe completed scope. Record artifact
   IDs, digests, distinct sessions, model and medium reasoning. Wrapper success
   or an empty findings list alone does not prove a completed review.
3. Require successful Ubuntu and macOS Quality jobs, Repository standards,
   Human readiness, Maintainer approval, AI review and native approval on the
   current head. Read back unchanged protection. Merge only after those exact-head
   human decisions and all gates pass; use the expected head without a bypass.
4. Keep task 4.4 open until that evidence exists. Keep task 3.2's remaining
   failure, stale-commit, revocation, missing-secret and release-refusal cases
   open until separately verified. A successful fresh run does not prove those
   negative cases.

Preparing this receipt starts no hosted work or model calls. It changes no
product behavior, source availability or release status. Source publication,
first releases and the relay's public setup and recovery docs retain their
separate gates. No site publication is claimed.
