# Design

## Context

See proposal.md for scope. At the start of this change, the local relay gate had passed on macOS, but the trusted shared workflow did not install the relay prerequisites. The later native CLI layer needs reviewed vault, Claude and registry APIs beyond the current trusted harness pin. The publication prelude retains that working pin and consumes no new harness API. Current private-repository CI must retain human admission and protected merge. The concurrent public-site migration is separate and uses unpublished ejm-ui packages.

At the initial Cloudflare inspection, Wrangler could access the account, but no ejm-cli-relay Worker or matching D1 database existed and custom domains did not include relay.ejmlabs.com. That inspection did not expose billing subscription details; it reported the standard usage model. The initial relay config had placeholder deployment values and closed admission. The D1 database has since been created and migrations applied. This caller-only layer contains no relay config and does not establish current deployment acceptance. Do not upgrade or downgrade the account; release and authenticated deployment checks remain open.

## Goals / Non-Goals

Goals: obtain exact committed dependency and product revisions with hosted evidence; deploy one inexpensive relay/UI origin; keep secrets out of source and logs; make pending launch steps visible.

Non-goals: changing repository visibility, activating payments or commercial collection, weakening human approval, publishing native installers, or taking over unrelated site and platform changes.

## Decisions

1. **Keep one Worker and one origin.** Use relay.ejmlabs.com for account, API, WebSocket and protected controller assets. Preserve run_worker_first, no-store and exact-origin controls. Keep workers.dev and preview URLs disabled. Use SQLite Durable Objects so the deployment remains eligible for free hosting. A separate static host would complicate authentication and add an origin.
2. **Stage with admission closed.** Create the real database and apply migrations, generate Better Auth's secret without printing it, deploy source-built production assets, and verify anonymous access controls. OAuth setup remains a human-held secret operation. Keep RETENTION_CONFIG unapproved until the operator reviews an actual notice. Start with two accounts only after real sign-in and notice checks pass. This is an explicit closed preparation deployment, not a claim that the production service is released.
3. **Respect private dependency ownership.** The user authorized a private harness PR. Prepare exact dependency changes in an isolated checkout, preserve concurrent work, and retain existing protections. Use expiring Contents: read access to that one repository for cross-repository checkout. Never copy the broad local GitHub token into Actions.
4. **Repair CI through the central owner.** Rebase the small setup hook onto the repaired central workflow, keep immutable pins, and use an allowed maintained installer. Split source publication within the existing file/line/byte admission caps. Use explicit maintainer delegation for routine readiness; exact-head approval requires its separate explicit human answer. A broken trusted-main caller needs its documented, explicitly reviewed recovery process.
5. **Bound scheduled work.** Audit operational cleanup, retention and privacy-task enqueueing together. Reduce oldest-first batches to fit the free 50-query invocation ceiling with headroom. Keep unfinished work durable. Use an hourly schedule for timely progress with small batches. Protect the total SQL/subrequest budget and repeated-drain behavior with a representative populated test, not a source-text assertion.
6. **Keep costs observable and claims limited.** Configure a conservative CPU cap if supported by the existing plan, retain per-account frame/byte/lease limits and two-account admission, and leave logging, Stripe and commercial programs disabled. Record artifact sizes and actual public responses. Local wall time does not establish production CPU or zero cost.

## Bounded source prerequisites

The first product PR combines the prepared local CI action, owned terminal
boundary and command foundation. Its logical base is merged CLI standards PR
#1, commit `4b263d4cb2a50a3641d566d6c18fdf7a0551b759`, whose tree equals the
reviewed `35dda43fcf94881971f7bddf3a0174ee7f18962a` source. Prepare from that
reviewed tree when the merge object is unavailable locally, then verify the
merged-base receipt before publication.

At its published cutoff, the combined prelude keeps the current harness pin
`4a751c6a3f24d7d143140f79ff53517fb93e3531` and both canonical callers at
`5fae851502c2e3c31d4df9a44f8d56c2cb49e098`. Its unchanged gate needs Rust and
Python. Its terminal additions do not introduce D-Bus, Node, WebAssembly or
browser checks. The prepared action is not invoked by that caller.

Keep only the terminal reader and native mode guard, minimum input/lifecycle
call-site hunks, terminal dependencies/notices, focused docs and the existing
macOS PTY readiness poll. Include command parsing, catalog, completion, picker
and Help for the implemented built-ins and granted package commands. Preserve
baseline provider composition, account handling and CLI arguments. Remote and
Claude UI, relay files, native fixtures and the separate site migration stay in
later layers. The existing authored public page receives only the matching
command keys guidance.

Combining the three layers reduces publication steps without claiming measured
CI savings. Resolve only the terminal dependency additions against the retained
harness pin. Do not reuse the larger native remote lockfile. Review the graph
and remeasure the final committed file, line and byte counts after lock
resolution against the unchanged admission caps. Pass the unchanged full local
gate, real PTY restoration checks and exact-source hosted checks before claiming
acceptance. See [source scope and local checks](../../../docs/ci-bootstrap.md#source-scope-and-local-checks)
and [terminal input boundary](../../../docs/terminal-input.md).

The supported order is: merge this prelude under the restored current caller;
review and adopt the central setup hook with the current reviewed working
harness dependency; then adopt the reviewed new harness API pin and publish its
native remote consumers, followed by Worker/browser source. Requiring the new
API pin before the hook creates a Linux D-Bus prerequisite cycle. Retain the
dedicated one-repository Contents: read credential and expiry checks throughout.
All human admission, approval and protected merge gates remain in force. Source
preparation does not complete task 2.3. Leave release tasks open until their
required verification and protected adoption exist.

## Risks / Trade-offs

- Private cross-repository access and human admission need owner input. Prepare exact PRs and commands; do not impersonate a human decision.
- A free-quota exhaustion stops service. Small admission and bounded work reserve headroom, but other applications share account allowances and a paid account can still incur usage charges.
- Browser code comes from the operator. Keep the existing disclosure that installed native clients have a stronger boundary.
- Concurrent source changes can invalidate a prior local pass. Capture the release source and rebuild it; record exact tested revisions and artifact hashes.
- Small scheduled batches trade more invocations for a lower per-invocation query and CPU footprint. Verify progress and document capacity before widening admission.

## Migration Plan

Publish and review the prerequisite dependency and CI changes. Configure and migrate a new D1 database without altering existing services. Build and deploy the closed relay/UI, then verify public origin controls. Complete OAuth, notices, human CI gates and authenticated live checks before opening two-account admission. Record Worker version, migrations, source fingerprint, checks and known gaps.

For rollback, close admission, restore a compatible recorded Worker version and preserve D1/DO state. No destructive database or namespace resets. If prerequisites remain blocked, leave the closed deployment and the OpenSpec tasks open with exact owner actions.

## Paired caller prerequisite

The combined prelude is now on protected CLI main at
`8a603af45504579ed1234bfe92a34b33367e44bd`, tree
`cc2bc0f0330fb8464cca4060886fc2f1e4be9297`. Prepare the paired PR/release
caller update and release `standards-ref` using reviewed central PR #11 implementation
`ae903129564926a14287f75d2012610c59cccb65`. It preserves protected central
main `d8b31f1debf095b104f6de76193bf531ec4ddd2c`, including its concurrent
docs change, keeps the CLI hook and adds fixed Linux D-Bus build setup for the
harness quality profile. The earlier `5309137` and `0adffdf5` targets are
obsolete history. Required central checks and exact-head human approval passed;
protected merge `8c304c8cdfc8b6fe50f3e442507215e615944f2b` has the same
tree `91b7f5c0990e27f2e0fdfcce26aac18afdcf6b81` as the reviewed head.
Keep the local
action, product source, locks and working harness pin
`4a751c6a3f24d7d143140f79ff53517fb93e3531` unchanged.

The exact implementation is retained at
`refs/heads/workflow-pins/ae903129564926a14287f75d2012610c59cccb65` under
active update/delete rule `25369476`, with no bypass actors and unchanged main
protections. The central prerequisite is complete. CLI caller adoption, its
hosted checks and this release task remain open. If the target changes, refresh
all three pin sites together and repeat review and retention verification.
Static candidate checks do not prove a hosted caller run or task completion. Keep existing human admission, approval, secret
scope and expiry requirements. After caller adoption supplies Linux native
prerequisites, adopt the reviewed new API pin separately, then publish native
and Worker consumers. See [CI setup](../../../docs/ci-bootstrap.md).

## Caller review source evidence

At CLI head `92e423b67243b15e97a43c5ba4776057172d242c`, both admitted AI
reports explicitly called the external-workflow comparison incomplete because
the previous and proposed central implementations were missing. Wrapper success
with no findings is not completed review. Keep the caller adoption tasks open.

Include [exact plaintext source evidence](../../../docs/ci/shared-workflow-review/README.md)
for both called workflows and their generator inputs in the ordinary committed
consumer archive. Deduplicate only byte-identical Git blobs and retain their
original paths, full revisions, content digests and MIT notice. Text suffixes
and ordinary data modes keep the evidence outside executable workflow and
product paths. It changes no caller, runtime, lock, gate or harness pin.

Pass the normal hook and obtain fresh exact-head admission, hosted quality,
two complete reviews and explicit approval before protected adoption. The
central merge and retention receipt does not replace consumer acceptance.

## Readable dependency review prerequisite

This follow-up starts from protected CLI main
`61aa4b4b137180ba6d76210b2cb5321cdc59658c`, tree
`504892783c5b2b49db490582f4a11a782b8542df`. It preserves the installed b2
harness selector, working harness pin and matching lock. Advance only the PR
caller, draft-release caller and release `standards-ref` together from
`b2e43c9b9ba814e6d3681d86c88830f3c0b11a3c` to reviewed shared PR #18 head
`40d446bb4c5113216fd05ec5423625b7af473987`. Its protected merge
`fbe8264f0ab47a4bf9260355dc8129728c13f261` has the same tree
`305c9c9be0bd80a36f33044eec9cdff639fb6c61`; its exact workflow-pins ref is
retained under the active update/delete rule with no bypass actors. Recheck
retention and protected main before admission.

The new shared reviewer setup extracts every prepared source tar with Python's
maintained `data` filter, including the already selected harness archive. Both
reviewers can inspect its source as plain files without extracting an archive
or fetching dependencies. Their existing read-only and no-network restrictions remain. They must not
execute project code. Use the maintained exporter for exactly the
old b2/new 40d shared source pair, with all 32 records and manifest below its
750000-byte cap. Retain the separate dated documentation bundle and MIT notice.
Do not add UI source metadata or install a new UI dependency in this consumer.

The existing persistent-account contract needs more than Linux D-Bus headers
on a headless runner. This change includes the root-owned local-action correction: install
Ubuntu noble-updates `gnome-keyring=46.1-2ubuntu0.2` with recommendations disabled,
start a maintained `dbus-daemon` session bus and unlock
`gnome-keyring-daemon --components=secrets --daemonize --unlock` using a random
ephemeral password on stdin. Keep XDG data/runtime in separate 0700 directories
under `RUNNER_TEMP`. Export only the bus address and XDG paths through
`GITHUB_ENV` to later credential-free quality steps. No real credentials, eval,
plaintext backend, product-vault change, memory-provider flag or test skip is
allowed. macOS setup and the product gate stay unchanged.

This source preparation proves no installed reviewer execution, Linux keyring
acceptance, hosted product result, protected consumer adoption or relay release.
Keep task 2.2 and every other release task open until their required evidence
exists. Run the unchanged normal hook and fresh exact-head hosted checks after
root integrates the action, then review actual complete, head-bound AI reports.
