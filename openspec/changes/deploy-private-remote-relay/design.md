# Design

## Context

See proposal.md for scope. The local relay gate passed on macOS, but the trusted shared workflow does not install the relay prerequisites. The later native CLI layer needs reviewed vault, Claude and registry APIs beyond the current trusted harness pin. The publication prelude retains that working pin and consumes no new harness API. Current private-repository CI must retain human admission and protected merge. The concurrent public-site migration is separate and uses unpublished ejm-ui packages.

The existing Cloudflare account is accessible with Wrangler. It has no ejm-cli-relay Worker or matching D1 database. Existing custom domains do not include relay.ejmlabs.com. Account billing subscription details are not available through this credential; the account reports the standard usage model. Do not upgrade or downgrade the account. The public relay config still contains placeholder deployment values and closed admission.

## Goals / Non-Goals

Goals: obtain exact committed dependency and product revisions with hosted evidence; deploy one inexpensive relay/UI origin; keep secrets out of source and logs; make pending launch steps visible.

Non-goals: changing repository visibility, activating payments or commercial collection, weakening human approval, publishing native installers, or taking over unrelated site and platform changes.

## Decisions

1. **Keep one Worker and one origin.** Use relay.ejmlabs.com for account, API, WebSocket and protected controller assets. Preserve run_worker_first, no-store and exact-origin controls. Keep workers.dev and preview URLs disabled. Use SQLite Durable Objects so the deployment remains eligible for free hosting. A separate static host would complicate authentication and add an origin.
2. **Stage with admission closed.** Create the real database and apply migrations, generate Better Auth's secret without printing it, deploy source-built production assets, and verify anonymous access controls. OAuth setup remains a human-held secret operation. Keep RETENTION_CONFIG unapproved until the operator reviews an actual notice. Start with two accounts only after real sign-in and notice checks pass. This is an explicit closed preparation deployment, not a claim that the production service is released.
3. **Respect private dependency ownership.** The user authorized a private harness PR. Prepare exact dependency changes in an isolated checkout, preserve concurrent work, and retain existing protections. Use expiring Contents: read access to that one repository for cross-repository checkout. Never copy the broad local GitHub token into Actions.
4. **Repair CI through the central owner.** Rebase the small setup hook onto the repaired central workflow, keep immutable pins, and use an allowed maintained installer. Split source publication within the existing file/line/byte admission caps. A human must post exact readiness and approval commands. A broken trusted-main caller needs its documented, explicitly reviewed recovery process.
5. **Bound scheduled work.** Audit operational cleanup, retention and privacy-task enqueueing together. Reduce oldest-first batches to fit the free 50-query invocation ceiling with headroom. Keep unfinished work durable. Use an hourly schedule for timely progress with small batches. Protect the total SQL/subrequest budget and repeated-drain behavior with a representative populated test, not a source-text assertion.
6. **Keep costs observable and claims limited.** Configure a conservative CPU cap if supported by the existing plan, retain per-account frame/byte/lease limits and two-account admission, and leave logging, Stripe and commercial programs disabled. Record artifact sizes and actual public responses. Local wall time does not establish production CPU or zero cost.

## Bounded source prerequisites

The first product PR combines the prepared local CI action, owned terminal
boundary and command foundation. Its logical base is merged CLI standards PR
#1, commit `4b263d4cb2a50a3641d566d6c18fdf7a0551b759`, whose tree equals the
reviewed `35dda43fcf94881971f7bddf3a0174ee7f18962a` source. Prepare from that
reviewed tree when the merge object is unavailable locally, then verify the
merged-base receipt before publication.

The combined prelude keeps the current harness pin
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
acceptance. See [prelude scope](../../../docs/command-foundation-publication.md)
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
