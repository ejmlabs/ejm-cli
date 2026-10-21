# Relay source publication prerequisites

This Worker/browser layer is being integrated on actual protected CLI parent
`78810826cd376588d3303f13bc237309fb5170a8`, tree
`dca23befbbe5bf065807f8ed06a98aa5b4c7a159`. The parent accepts native source
and inert install/review inputs. It does not accept this complete Worker,
new UI installation or browser runtime. Worker PR #23's first correction head
`9336df2fdd551fdc7a0d5d870788301d116b555c` passed its normal local gate,
but both complete hosted reviews reported four medium blockers and one low
advisory. The [second review corrections](#second-worker-review-corrections)
are being prepared locally. Their checks, fresh hosted acceptance, protected
adoption, deployment and authenticated live acceptance remain pending.
The historical closed deployment is recorded in [deployment status](remote-deployment.md).

The approved [notice](relay-notice.md) and [terms](relay-service-terms.md) remain
unpublished with effective dates pending actual publication. Their approved
wording and retention policy have not changed. Admission remains closed with a
two-account limit. Payments, commercial collection and buyer exports stay off.

## Protected shared source and inert prerequisite

| Layer | Accepted source and evidence | Boundary |
| --- | --- | --- |
| Native CLI PR #21 | Protected merge `71d0f47f8b05c6314eca4dc0750ffaad833d6afb`, tree `3adf3701aec18883b2aabd710e72fa3c513c7fdf`; [run 39511620937](https://github.com/ejmlabs/ejm-cli/actions/runs/39511620937) | Linux/macOS: 133 unit and six integration tests per platform plus the native PTY gate; both exact-head reviews complete with no findings. See [native acceptance](native-remote-publication.md). |
| Shared UI PR #11 | Protected merge `79a1aa8b1075f834717bc0c0299dc2c41a22b966`, tree `c18c74fd336dbad9515bff8ecf22ca980b889a51`; reviewed head `4e4a630710ac504669437f3fcc8609ae98577f35` | Shared owner source accepted. Consumer installation and runtime are separate. |
| Inert CLI PR #22 | Reviewed head `f44c57396bc220562a8dae83767e188977e2c2ce`; [run 39520822069](https://github.com/ejmlabs/ejm-cli/actions/runs/39520822069); [human approval 6305211216](https://github.com/ejmlabs/ejm-cli/pull/22#issuecomment-6305211216) | Linux/macOS quality, registry, standards and both complete reviews passed with no findings. Normal protected squash merge at 2026-10-21 00:51:27 UTC produced parent 78810826 above with the exact reviewed tree. |

PR #22 preserves the native Cargo baseline SHA-256
`02ba0f0097754444a40e106d683ce530886ec7c31d30009f0c5cc43750d6118b`,
harness protected merge `42e4f9cf33bd3525aec344cfdffabc251271f782` and its
358-file tree `826e02310f1097e13806ca5ca1bf5cc00d71e966`, and the installed
40d workflow callers/context. Its inherited UI selector names protected 79a1.

The accepted inert scope includes thirteen complete metadata bindings, the
28-path current-owner source supplement, Relay's exact lock and a complete
51-line manifest fixture. Those review files remain nonexecutable data. Do not
import, execute, install or package them. The first metadata-only PR #22 head
received two incomplete reports; its revoked admission remains historical and
does not qualify any source. The successor's complete reports qualify only the
inert successor.

## Source and dependency boundaries

The Worker layer activates the matching runnable manifest and selectively adds
`relay/`, `crates/ejm-relay-web/`, two native transport fixtures and the gate
adapter. It excludes generated assets, installed dependencies, local Cloudflare
state, credentials, old native source and the separate public-site migration.
Native `queue_ready`/`settle`, atomic account vault publication, cancellation,
pairing, snapshot recovery, fresh expiry checks and all fourteen native PTY
cases across seventeen PTY sessions remain intact.

The browser uses Snow's public Noise API through wasm-bindgen. It has no
harness, provider, execution, credential-store or native HTTP dependency.
Native execution and persistence remain harness-owned. Better Auth owns account,
OAuth, device flow and cookie infrastructure. Stripe is installed for the
optional billing adapter but remains disabled. No duplicate auth or crypto
engine is introduced.

All visuals and Relay artwork use public `@ejm/core`, `@ejm/tokens` and
`@ejm/ui` exports. ejm-ui owns reusable presentation and interactions. Product
composition, account state, pairing and HTTP adapters stay in ejm-cli. React is
build-time only. See [shared appearance](relay-web.md#shared-appearance-preparation)
and [UI build notices](../relay/UI_BUILD_NOTICES.md).

The new gate must bind all 941 ordinary files of selected UI tree c18c74f, then
install its public local packages with `npm ci --ignore-scripts --install-links`.
Package version `0.1.0` or a local path alone is not a source identity. Metadata
bindings and the review supplement do not prove installation. Preserve licenses,
notices and reserved asset rights from the owner source.

Add only the browser workspace/package and required feature edges through Cargo,
starting from the accepted native lock. The resulting lock hash will change;
compare package identities and the native target graph. Preserve patched
rustls-webpki `0.103.16`, webpki-root-certs `1.0.10`, native roots and the existing
tempfile graph. Never copy the obsolete preview lock or broaden upgrades.

## Verification and release evidence

The release source needs the unchanged full entry point `./scripts/check`,
including native Rust, dependency and PTY checks, debug/production WASM, strict
frontend/Worker types, schema drift, populated cleanup tests, native/workerd
round trips and real Chrome fixtures. Fixture-only tests are driven by their
supervisors from the current Cargo artifact record; ignored fixture declarations
are not a substitute for running them. Record Windows evidence separately.

The visual budgets are common CSS at most 14,336 bytes gzip, finite templates
at most 49,152 raw bytes, bootstrap at most 2,048 bytes gzip and fonts at most
34,536 raw bytes. Production WASM is capped at 1,572,864 bytes. Record Worker
bundle size and any incremental presentation budget against a comparable build.
Earlier sample screenshots and 111 mocked Worker tests do not prove this complete
source, real authentication, WASM, execution, provider CPU or cost.

Before requesting hosted admission, measure the actual final patch against the
unchanged 200-path, 12,000-changed-line and 1,000,000-diff-byte caps. Preserve
all required behavior if a real split is needed. Obtain fresh human `/ready`
for the exact revision, hosted Linux/macOS quality, standards/registry checks,
two complete exact-head reviews, separate human `/approve`, then protected merge.
A new revision or failed review needs the repository's fresh admission process;
old approvals do not authorize the Worker revision.

### Worker review corrections

Both reviews in [run 39534461293](https://github.com/ejmlabs/ejm-cli/actions/runs/39534461293)
completed for Worker head `cf1efa10edc61647feac62b35b9e9c3420661283` and reported
five distinct defects. The reports blocked adoption. They were complete reviews,
not capacity or timeout failures. The first correction scope was:

- Retire an old socket pair before replacement admission, isolate replacement
  pairs from delayed close/error callbacks and require both endpoints to start
  a fresh handshake. Legacy socket attachments close on upgrade; no database
  migration is required.
- Collect only usage fields covered by current approved consent, keeping only
  the counters needed for those fields.
- Merge only unexpired monthly usage, retain the earliest included contribution's
  expiry and exclude expired rows from access and exports before cleanup.
- Honor operational holds during rights deletion, report retained identity as an
  explicit exception and still revoke service access.
- Route dashboard closure through durable rights, recipient and operational
  tracking, displaying its one-time private tracking key before the user leaves.

These first corrections passed the normal local `./scripts/check` at head
`9336df2fdd551fdc7a0d5d870788301d116b555c`, tree
`11e3b1a9980cddecc358db2136ee9d57c29cddba`, on macOS arm64 on October 21,
2026 UTC, from 03:21:57 through 03:23:07. The gate recorded unchanged source.
Both fresh hosted reviews then found further defects and blocked adoption.
The earlier results below remain cf1-only history. No approved policy body,
retention duration, admission or commercial flag changed.

### Second Worker review corrections

Both reviews in [run 39539622114](https://github.com/ejmlabs/ejm-cli/actions/runs/39539622114)
completed for head `9336df2fdd551fdc7a0d5d870788301d116b555c`. They reported
four distinct medium blockers and one low advisory. Both reports were complete;
no runtime timeout or capacity failure caused their result. The prepared fixes are:

- Persist each Room's account owner and original share deadline independently
  of sockets. Enforce that owner after disconnection and eviction, and claim
  the Room before publishing a ticket. Retries cannot extend its deadline.
- Allow a fresh verified account session to stop its owned share without
  enrolling a transport device. Preserve exact-Origin checks, device-bound
  connection tickets and local controller pairing.
- Retain pending Room closure targets after quota revocation until the Room
  acknowledges closure or verified original expiry invalidates every transport
  lease. Failed or remaining unexpired targets keep account-closure work pending
  through the existing durable rights workflow. Expired targets are removed
  without letting an old retry retire a later owner of the routing ID.
- Revalidate included usage rows and their deadlines inside the final disclosure
  transaction after the awaited digest. Expired or missing rows cannot be released.
- Invoke `scripts/check.py --relay` through `python3` from the PowerShell shortcut. A real
  macOS PowerShell smoke showed that the old wrapper failed before reaching the
  gate; the corrected wrapper reached its prerequisite check and propagated
  exit status 1. This does not verify Windows runtime or a complete gate run.

The lifecycle and disclosure corrections remain local preparation. Their focused
regressions, the normal full gate and fresh exact-head hosted reviews remain
required before separate human approval and protected adoption. They add no D1
migration, dependency or service and change no approved policy body, retention
duration or release control. Unowned legacy Room state requires a fresh share ID.
Preserve ownership, pending closure and retirement records; rollback to code that
ignores them is unsupported while that state remains live. See
[deployment and rollback](../relay/README.md#deployment-and-rollback).

Deploy only production artifacts from the accepted source, with admission closed,
then bind the Worker version, configuration and artifact hashes. Verify anonymous
access controls and protected asset denials. Real account/device sign-in, native
and browser pairing, view/control, reconnect and revocation at HTTPS/WSS remain
separate acceptance checks. Do not infer them from an OAuth start or a sample
preview. Preserve existing secrets and durable state on retries.

## Current local checks

The following October 20 results describe head cf1 before the review corrections.
Its preparation ran on macOS arm64 with Rust 1.99.1, Node 26.12.0,
npm 11.20.0 and wasm-bindgen 0.2.129. Local stages passed 133 native unit tests,
six CLI integration tests, five browser-crypto tests, 118 Worker tests and eleven
real Chrome cases. The native PTY supervisor passed all seventeen sessions.
Native-to-Worker and native-to-browser encrypted fixtures used the current locked
Cargo test-artifact record. No paid provider was called. The normal full
`./scripts/check` also passed for cf1 before its commit. These results do not
qualify later source; the first correction's bounded local result is recorded
[above](#worker-review-corrections).

The cf1 storage fixture permitted only the Room's bounded retirement metadata and
scanned every bounded D1/DO row and captured log for content and keys.
Two controls confirmed rejection of an unexpected Room table and an extra
retirement field. Separate old-code controls reproduced expiry during D1/quota
waits; corrected tests reject expired tickets, sessions and either endpoint's
expired lease while permitting a valid renewed lease.

The complete selected UI tree contains 941 ordinary files. All 374 installed
public package files match its bytes and Git executable modes. The native feature graphs match
protected parent 78810826 for macOS and Linux after local-path normalization.
The browser graph excludes the harness, Tokio, native TLS and ring. Cargo added
only the local browser crate and required getrandom JavaScript edges; existing
package versions and checksums remain unchanged.

| Artifact | Local measurement | Budget |
| --- | --- | --- |
| Shared CSS | 13,807 bytes gzip | 14,336 bytes gzip |
| Finite templates | 48,345 raw bytes | 49,152 raw bytes |
| Appearance bootstrap | 289 bytes gzip | 2,048 bytes gzip |
| Two fonts | 34,536 raw bytes | 34,536 raw bytes |
| Production WASM | 484,136 raw bytes | 1,572,864 raw bytes |
| Minified Worker | 1,206,144 raw bytes; 294,631 bytes gzip | Record the full bundle separately from presentation growth |
| Presentation growth | 788 bytes gzip | 32,768 bytes gzip |

The presentation comparison uses the same 36 functional/config/dependency
inputs and toolchain, replacing only seven recorded presentation inputs with
their earlier bytes. Its baseline Worker is 293,843 bytes gzip. Both bundles
use Node's default gzip compression. No component framework runtime is emitted.
Local bytes and loopback timings establish neither production CPU nor zero cost.
The two complete hosted reviews rejected cf1. Fresh corrected-source hosted
acceptance, protected adoption, updated deployment, public policy publication
and authenticated live acceptance remain separate.

## Public documentation and policy publication

This checkout's public page is static `site/dist/index.html`, with no guide tree
or docs build. Its single remote-access paragraph can be corrected without a site
migration. The current live CLI docs deployment cannot be matched to this saved
source and complete asset set. Do not overwrite the live site with this older
static tree or claim these Markdown guides are live.

The separately coordinated approved notice/terms overlay must pass the active
site's normal build and publication process. Verify live routes, content and
effective dates after publication. The active UI and source revisions, complete
assets and deployment receipt must be bound before publishing. A closed Relay UI
update can proceed through its own release gates while account admission stays
closed; opening accounts still requires the published/configured policies.

The approved brief bootstrap window is for creating the operator account after
those gates. The two-account cap limits admitted relay accounts, not every
possible GitHub identity created during an open OAuth window. Close admission
again after bootstrap, verify the operator flow and complete live acceptance
before the bounded beta opens. Payments and commercial collection remain off.

The [deployment change](../openspec/changes/deploy-private-remote-relay/proposal.md)
retains eleven whole release tasks. Keep tasks open until all their criteria
are verified. Sync requirements and archive only after the full release is done.
