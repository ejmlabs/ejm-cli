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

## Native source successor

Prepare this native layer on actual protected CLI merge
`61aa4b4b137180ba6d76210b2cb5321cdc59658c`, tree
`504892783c5b2b49db490582f4a11a782b8542df`. CLI PR #7 protected-squash
merged the approved `dd0b1fb72e399623458debef8bc28554407c789a` tree after
hosted run `39317164563`, both complete reviews, exact approval and unchanged
protection. Preserve its three B2 callers, readable context, owner standards and
PR #6's guide consolidation and standalone-guide deletion.

Apply only the reviewed 51-path delta from selector head
`dd0b1fb72e399623458debef8bc28554407c789a` to local native commit
`395524dd45a1096e4949d0eba742db17d1aaa940`, tree
`5e2e3c61ff6150353bbddda656266c7cd5ab39f8`. Keep all 40 native code,
test, lock, gate, spec and landing-page inputs byte-identical; refine only the
current status records. Move only still-needed native status from
the removed guide into the consolidated and native guides. Do not overlay a
candidate tree or copy a separate site migration. Worker/browser source and
their unconditional gates stay in their following layer.

Pin the public API dependency to actual protected harness merge
`220305fe4be7fecd33c3cf04b24f7a1b4b50503d`, tree
`518bd4ecfc28b31832465df083fe53df9cd053dd`. Its source equals the earlier
provisional API tree, so retain the exact reviewed native lock as the starting
graph without hand-editing it. Root must verify locked resolution and the full
gate on this exact pair. If Cargo requires a lock change, use Cargo's maintained
update command and review the resulting diff before acceptance.

The earlier native commit passed root's normal local gate on macOS: 116 unit
and 6 integration tests, formatting, strict Clippy, release, Rustdoc, Machete,
Deny and PTY. Its exact log and source are recorded in the
[native guide](../../../docs/native-remote-publication.md#historical-local-pass).
This protected-parent revision needs its own normal full gate. Review the
docs/publication source boundary before native admission; then require hosted
B2 authority and archive proof for harness `220305fe...`, both actual complete
reviews, explicit approval and protected native adoption. Source preparation
completes no release task. Real native key-service, terminal and live relay
acceptance retain their separate limits.
