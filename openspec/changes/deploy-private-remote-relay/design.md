# Design

## Context

This docs/spec and UI source prerequisite retains protected C8's existing runtime,
tests, gates, working 4a harness pin and lock. It publishes planned guides, delta
requirements and static install review data. It selects actual protected UI
merge M `24935aa85355ea99f49bdbecc4186a5e95049393`, tree
`2d8bde67c8b5fe851b70ae90a3a2787091397780`, in `.github/ui-source.json`.
This owner merge includes the shared UI and Relay artwork changes; it is distinct
from the future protected CLI prerequisite P. Keep complete original license, root manifest, lockfile
and all ten workspace manifests under `review/dependency-sources/ejm-ui/`, with
commit, tree, path, mode, blob, size and SHA-256 provenance. Recheck the actual
workspace closure and tracked npm configuration before export. These are ordinary
nonexecutable review files, never imported, executed or packaged as product code.

The installation PR's old protected base has no UI source pin. Its checks cannot
prove the proposed checkout or install. The following native PR must bind the
installed pin, exact UI source archive and credential-free lifecycle-disabled
install, alongside the harness and product source, both complete reviews and all
protected checks. Keep installed callers and canonical `.github/review-context`
unchanged. Native implementation, main-spec adoption and release verification
follow separately; no task is completed by this source preparation.

See proposal.md for scope. At the start of this change, the local relay gate had passed on macOS, but the trusted shared workflow did not install the relay prerequisites. The later native CLI layer needs reviewed vault, Claude and registry APIs beyond the current trusted harness pin. The publication prelude retains that working pin and consumes no new harness API. Retain human admission and protected merge regardless of the separately owned public rollout. The concurrent public-site migration is separate and uses unpublished ejm-ui packages.

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

## Historical paired caller prerequisite

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
protections. The central prerequisite was complete at this cutoff; later caller adoption is
recorded below. This whole release task remains open. If the target changes, refresh
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
That caller-only layer added no UI source metadata or new UI dependency.

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

CLI PR #9 subsequently passed the unchanged gate on Linux and macOS in run
`39329367008`, with 58 unit and 5 integration tests per platform and two complete
head-bound reviews. Exact approval preceded protected squash merge C8 below.
Its run used installed b2 and harness 4a: the isolated Linux setup passed, but
installed 40d readable harness extraction and the native vault gate still need
the next admitted source. Keep all release tasks open.

## Native command termination correction

The admitted native review found that `remote::run` handles Ctrl-C but not
SIGTERM while a real remote command owns protected terminal input. Reuse
`ui::termination_signal` so both signals cancel the command and drop its
terminal guards. Extend the existing PTY supervisor with the built release
CLI at the controlled migration confirmation, using an offline relay origin
and stopping before any confirmation, credential access or network request.
Require restored native mode, alternate screen, cursor and paste state, no
private-input output, and the normal cancellation exit. Retain every ordinary
restoration assertion and all eight existing remote fixture cases. Root must
show the new regression fails on old `2c20695...`, then run the unchanged full
gate on the corrected source before exact-head hosted review and adoption.
No earlier local pass or passing platform job closes this correction.

## Native display recovery correction

After the display channel drops an event batch, retain the highest suppressed
`through_sequence` and stop forwarding live event batches. Queue ordered journal
snapshots through the existing bounded channel, wait for the display cursor to
acknowledge each page, and continue after every full page. A short or stale page
does not finish recovery while a higher suppressed sequence remains outstanding.
Release the barrier only after an acknowledged short page covers that sequence.
Keep at most one refresh in flight, preserve replay replies and control results,
and never resend a mutation. This adds only fixed-size progress state, not an
event buffer, protocol field or dependency. Protect the actual channel and event
application path with deterministic saturation, multi-page and late-page cases.
The [corrected local macOS gate](../../../docs/native-remote-publication.md)
later covered this recovery. The final native source after protected CLI
prerequisite P still needs its normal gate and hosted acceptance.

## Earlier C8 native correction composition

Compose frozen integrated native tree `b7569e88633687e5a22e89f8a90d57504137c609`
on actual protected PR #9 merge `c8e2239392f71df82cbef6166b1827c8e5f5f1c5`,
tree `d49abd3f2881f8a2328063bc45d9ded4a078e00f`, identical to approved head
`7b5dc84632e79498e6cabc45a425595b9ac867a3`. Preserve its exact 40d callers,
readable source context, Linux Secret Service setup, owner standards and
consolidated-guide deletion. This source-only successor selects protected API merge 42e and updates status
docs. The approved registry consumer correction below changes only its CLI
routing and focused tests. Preserve every other frozen native production, test,
gate and lock byte. Recheck the protected source after the separate public
cutover before native admission.

The physical sibling contains exactly 358 tracked files from prepared harness
tree `826e02310f1097e13806ca5ca1bf5cc00d71e966`, with no Git state or
ignored caches. Its actual ordinary protected merge is
`42e4f9cf33bd3525aec344cfdffabc251271f782`, selected by the updated pin.
Root verified matching reviewed 826e source and unchanged checks at 22:44 UTC;
fresh post-cutover qualification at 22:46 UTC confirmed public CLI main C8, harness
main 42e, restored Actions/callers and unchanged protection. Preserve native lock
SHA-256 `5143432fc0201bedeef680d18ae1c4681e2215fddd6e2fb45cd4cbf0149f7d1b`;
pass locked resolution and the unchanged normal gate on this final source. All eleven
release tasks remain open. The following successor record is historical.

## Earlier native source successor

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
the then-current B2 authority and archive proof for harness `220305fe...`, both actual complete
reviews, explicit approval and protected native adoption. Source preparation
completes no release task. Real native key-service, terminal and live relay
acceptance retain their separate limits.

## Exact completion before queued admission

Native review of `2c20695...` found that durable `RunFinished` precedes the
harness's bounded run-end observers and active-map cleanup. Starting the queued
prompt at that event can reject it as busy and discard its accepted successors.
The selected 220305fe harness also limits prompt text to 16 KiB, while the native
wire validator admitted 64 KiB. These are source findings, not acceptance of a fix.

Prepare the additive exact-session, exact-run completion subscription in the
harness owner first. It reuses the existing Tokio watch receiver, including
internally queued successors, and adds no task, channel, polling or history map.
The native host retains one completion handle for its current revocable queue
target. It waits outside its authority lock through detailed completion, then
refreshes durable events and rechecks the same share epoch, expiry, target and
current run before popping a prompt. The harness decides `queued_successor`
under its active mutex before notification, after draining late steering. A true
flag discards the dependent remote chain even if that local successor finishes
before replay can observe it. A false flag is not an idle or admission permit. Local control and revocation can acquire that lock during the wait.
Stopping, expiry or discarding the queue clears the retained handle. A missing
subscription or a finished event alone does not prove successful completion.

Use the native protocol's one 16 KiB UTF-8 byte limit before acknowledging every
text mutation. Keep remote queue ownership in the host; do not move it into the
harness queue, retry uncertain writes, match error strings or sleep to guess
completion. The API is now protected at merge 42e and the exact pin selects it;
the final native source after protected CLI prerequisite P still needs its
normal locked gate and hosted source binding.
Keep all eleven deployment tasks open. The retained 0a7a/826e focused run passed
12 backend contracts, including blocked completion, revocation, fast local
successors and exact multibyte limits; event-only and 64 KiB controls failed as
intended. Recovery separately passed two corrected cases, while the original
SIGTERM case failed old 2c as intended. Corrected SIGTERM and the combined native
checks passed in the [local macOS gate](../../../docs/native-remote-publication.md).
The final native source after protected CLI prerequisite P still needs its
normal gate and exact-head hosted acceptance.

## Registry vault consumer correction

Use the harness owner's public origin-bound registry credential APIs. Login
already saves its session in the shared encrypted vault. Replace the CLI's
obsolete plaintext-file logout removal with `delete_registry_token`: retain the
existing load-first errors, attempt local deletion after remote revocation, and
return a storage failure or the revocation failure before reporting success.
Do not restore plaintext fallback or infer missing credentials from error text.

Expose `package migrate-credentials --old-clients-stopped` as the controlled
legacy handoff to `migrate_registry_token`. Clap must reject the missing flag
before opening storage. The harness owns origin checks, exclusion, key recovery,
commit/reload verification and permanent retirement markers. A no-legacy result
can still initialize vault storage. Keep output free of credentials and avoid
new storage code, dependencies or public interfaces in the CLI.

Protect local deletion after failed revocation, deletion failure before a
success claim, and the real command parser's confirmation requirement. Reuse
the harness's existing vault tests rather than copy their implementation.
Update the canonical registry guide in this source layer; the separate docs
composition and final pair's normal locked and hosted gates remain required.
This source correction completes no deployment task.

## Native lifecycle review correction

Both complete reviews of native head `e7409919...` found that awaiting completion
inside the UI tick stalls local revocation and that host startup enters protected
input before signal registration. One also found local cancellation relied on
replay idleness while the harness still retained run-end cleanup and successors.
Poll the retained completion once per tick with `FutureExt::now_or_never`, after
local input, keeping the existing exact-run and authority checks. The transport
also awaits that receiver outside authority as a wake only, with no idle clock;
cancellation and expiry precede a fresh admission check. Install the
biased signal scope before preparation, cancel its parent token and revoke the
share on exit. Keep terminal futures scoped and host tasks cancelled on drop.
Trusted local stop uses session cancellation; remote cancellation stays run-bound.
Exercise the production terminal loop with blocked observers and the actual
memory-only host startup using a test-only offline login transport seam. Prove
input acknowledgments precede observer release or timeout. Fresh normal and hosted
gates remain required; all eleven tasks stay open. The separate certificate
duplicate allegation is not accepted evidence against the passing dependency gate.

## Native pairing and recovery review correction

Run `39344263873` passed Linux/macOS quality, but both complete f66 reviews
blocked adoption on four defects; its admission was revoked. Correct them as follows:

- Draw each new confirmation, purge buffered input, then accept a fresh key.
  Remove confirmations whose existing oneshot receiver has closed.
- Pass cancellation to public vault locks; use a child-token drop guard at the
  existing blocking-worker boundary. Native key calls remain uninterruptible.
- Feed server gaps into existing snapshot recovery before enabling work.
  Missing events never establish Idle.
- Decode against share lifetime; retain host enrollment deadlines and pinned keys.

Use real terminal, network/Noise and memory-vault contention regressions, covering
cancelled and dropped callers. Keep the harness, lock, dependencies and caps exact.

## Snapshot authority and mutation expiry

Head `06154b18...` passed its local gate and hosted Linux quality, but its second
complete review found two backend defects. Its admission was revoked. A snapshot
can include a completion journaled after the first authority refresh; refresh
authority again after selecting that bounded page and before returning active run
state. This adds one ordered journal pass per snapshot, not one per returned
event, and must cover its delivered cursor without treating missing output as Idle.

Recheck the existing UNIX-millisecond platform clock after the authority lock and
asynchronous refresh, immediately before direct or queued mutation admission. Receipt time cannot
authorize work after expiry. A per-backend test-only clock and request checkpoint
exercise contention and the refresh boundary deterministically; production adds
no clock service, task or dependency. Cover completion during real snapshot
construction, a subsequent ordinary Submit, and expiry while a request waits.

First publish the planned remote guides and delta contract as a bounded docs
prerequisite on C8, retaining its working pin, runtime, tests and gates. Keep the
main spec with the native implementation. Measure native scope against the actual
prepared prerequisite tree, then reconcile its real protected merge without
rewriting PR history. No prerequisite publication verifies native behavior or
closes any of the eleven release tasks.
