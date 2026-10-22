# CLI capacity verification

## Status

Implementation and acceptance remain incomplete. This change stays open at
33/39 tasks. Automatic CLI defaults remain gated. No release, publication or
deployment occurred.

The harness owns admission, native lifecycle, storage and learning. The CLI
consumes those public contracts and owns bounded terminal presentation and
its test-only PTY observer. Dependencies remain CLI to harness. No shared
runtime API changed in the journal-envelope follow-up.

## Current-source macOS native crash check

The harness's existing `native_owner_smoke` passed after an offline, locked,
two-job debug build on 2026-10-21 UTC. The fixture took 19.296 seconds and the
complete wrapper took 22.090 seconds. This verifies the identified harness
checkpoint; it is not a release CLI measurement or full task 1.2 acceptance.

Report:
`/private/tmp/ejm-capacity-current-owner-recovery-20261021/20261021T042249Z-85ad0f2e/report.json`,
SHA-256 `ccceeaf1621139a8cea8094ebc834f74edfe6b9c604f690f382835c91d30da7e`.
Reviewed wrapper SHA-256:
`226ac401519d9c95846509818377155cbd277bced2e29761832b1fe9164b1647`.
The copied debug fixture matched its build output, SHA-256
`e7f54f89df101f567416328f8f915e97c46de1534604a18208f058a376bd6c45`,
15,007,840 bytes. Full Git-listed harness source stayed stable during the run.

The fixture asserted idle/busy client and owner crash progress, paused-owner
exclusion, fresh-grant progress, retained executor cleanup, spawn rejection and
launcher/witness lifecycle behavior. The detached-descendant case truthfully
reported survival after owner death. Both workload and cleanup passed. Both
command children were reaped, all 11 captured process identities ended, all
five retained witness handles were released after path removal, and final
exclusive-executable discovery was empty. No wrapper cleanup signals,
observer errors or audit requirement were recorded across 796 samples.

Two short-lived processes disappeared with `ESRCH` before birth capture. Their
final exact-path absence is recorded without a captured-identity death claim.
The observer saw the before-spawn witness only after release; the fixture's
own assertions checked it while held and after release and reaped the launcher.
The wrapper's phase/cleanup/180-second total guards remain cooperative. The
fixture keeps its internal 45-second timeout. Arbitrary descendants, wrapper
failure paths, ignored dependency/compiler inputs and other native platforms
remain outside this result. Minimum sampled free space was 26,805,829,632 bytes.

No task was closed. Automatic CLI defaults remain gated on the full native,
package and overhead requirements. Existing public recovery docs already cover
these limits; this verification-only record changes no public contract or site
claim.

## Current release helper calibration and corrected pool

The harness's separate
[current-image index](https://github.com/ejmlabs/ejm-harness/blob/main/docs/evidence/native-helper-current-image-memory.macos-arm64.json)
and raw bundle retain three fresh startup and three control-state saturation
trials per release CLI and worker image after the stable-lock repair on this
Mac. CLI SHA-256 is
`79193b2d6de3032f6c74d74e8a4d4b466c9b4a4b4cbe0345ce91ef3bc1c1199e`;
worker SHA-256 is
`1ca0a09f388406318f66221c25b80be66bef4331c24fb85e46daa45d55376dcc`.
Startup lifetime RSS maxima were 7.625 and 7.453125 MiB; saturated control-state
maxima were 14.703125 and 14.421875 MiB. All 14 saturation stages passed in
each trial. Startup held authenticated readiness for six command-free seconds.
No native commands ran. All clients were reaped, exact helper identities ended
and original witnesses released, including the retained-inode saturation proof.
The provisional 32 MiB charge stays unchanged. These trials do not calibrate
other images, targets or full process trees, or establish cold/warm speed.

Startup report SHA-256:
`9ae5cccf1f43f9738d07e9eec09a38e7e03a7fdfda6e3b6d5d2eb70c1ef2f48b`.
Saturation report SHA-256:
`24bd6f3f26bc25297b764f490860e19c2595eb9459a5e085c3a551e258c1a3ac`.
The [harness method and limits](https://github.com/ejmlabs/ejm-harness/blob/main/docs/benchmarks.md#real-embedding-helper-memory)
identify the original reports, build inputs and raw samples.

A separate corrected benchmark pool completed 6,400 commands with eight charged
helpers. Its ninth claimant started only after eight commitments, exact helper
identities and held witnesses were confirmed, then failed actual pool admission.
All nine clients were reaped and all eight helper identities ended and witnesses
released. The 20 sequential scans over 4.753665 seconds reached 67.984375 MiB
helper, 190.890625 MiB client and 258.875 MiB combined RSS scan-sum high-waters.
These are not simultaneous or complete process-tree peaks and do not establish
eight saturated CLI or worker helpers. Pool report SHA-256:
`ca520a09b6b91965e9ebc5ce8009878df2c9d8d0fed390ed8dcfc1c2e1a56514`.

The failed pool, its cleanup audit and ordering diagnosis, the source-preflight
failure and the corrected barrier remain retained. A fresh build reproduced all
four earlier image hashes; that identity bridge is not another memory trial.
The separate pool build preserves its own source provenance. Runtime source
checks passed while an excluded harness contributor-doc change was recorded.
No all-source-stable claim or task closure follows. The fresh 4096-agent cases
below have their own source receipts. The final docs checks below cover the
public-doc updates. Automatic defaults stay gated.

## Current-image 4096-agent useful-progress diagnostics

The fresh V5 checkpoint passed strict CLI Clippy, release helper and fixture
builds, configuration and full-envelope tests, parser/observer checks and both
100-agent fixed/automatic shared-root smoke cases. All commands and observers
were reaped; both repositories' source checks passed with no excluded changes.
Checkpoint:
`/private/tmp/ejm-cli-capacity-fixture-checks-20261020/20261021T033611Z-36545fb5/report.json`,
SHA-256 `56e83e6347d9d655ceee60025d86f891435e3ff37a4e7d361affe403ddbbc028`.
The exact docs input and input-only runner adaptations remain in
`/private/tmp/ejm-cli-current-4096-prep-20261020/prepared-20261021T033554Z-72c8cbe2/lineage.json`.

The ordinary release helper is 18,109,840 bytes, SHA-256
`79193b2d6de3032f6c74d74e8a4d4b466c9b4a4b4cbe0345ce91ef3bc1c1199e`.
The test-only fixture is 19,587,840 bytes, SHA-256
`66ed13c51c7136a57d4021526dcf7fab538dcc05cf2bf09bc6a079caa26367c2`.
Both separate diagnostics used those exact frozen images on this Mac. Each
case ran two processes with separate session stores and one shared capacity
root. Each process managed 4096 agents with ceilings of two model requests,
two jobs, one heavy job, 2000 CPU millis, 512 MiB memory, 32 processes, queue
256, 1 MiB output, 64 MiB artifacts and a 120-second job deadline. The work
window and existing observer cap both stayed at 120 seconds; the case deadline
was 270 seconds within a 420-second budget. No paid provider calls ran.

| Mode  | Participant | Completed turns | Completed tools | Initial completion p50 | Initial completion p95 |
| ----- | ----------- | --------------- | --------------- | ---------------------- | ---------------------- |
| Fixed | 1           | 1657            | 4096            | 103.030 s              | 119.209 s              |
| Fixed | 2           | 1857            | 4096            | 101.044 s              | 119.139 s              |
| Auto  | 1           | 1363            | 4096            | 111.442 s              | 119.285 s              |
| Auto  | 2           | 2377            | 4096            | 102.442 s              | 118.619 s              |

All four observers passed with zero active or drain failures. Neither mode
completed repeat turns. Each participant had 4096 pending turns at cutoff,
all accounted for as drain cancellations, with zero drain completions. Pending
counts include replacement turns started after earlier successes. They are
not additional useful completions. Latencies describe completed initial turns
only and are censored by the work window.

The sums of participant completed/controller-elapsed rates were 29.2835 turns/s
for fixed and 31.1615 turns/s for automatic. Approximate work-window overlap
was 99.9978% and 99.9756%. These are single sequential cases in fixed-then-auto
order, with uncontrolled host noise and no safe static sweep. They establish
useful progress for this envelope, not a causal speed improvement, steady
state, tuning optimality, UI overhead, complete latency percentiles or task
7.4 acceptance. The fixture's active-window snapshots do not measure settled
idle CPU or complete process-tree peaks.

Both case completion and cleanup passed. Exact client/helper identities ended,
participant and helper witnesses released, all observers were reaped and no
audit remained. Source checks passed without excluded changes; binaries and
runner inputs stayed unchanged. The new
[evidence index](../../../docs/evidence/native-coordinated-current-4096.macos-arm64.json)
and raw bundle retain exact original reports, sidecars, source/build receipts,
commands and limits. Original failures remain unchanged.

- Fixed report: `/private/tmp/ejm-cli-capacity-comparison-20261020/20261021T033818Z-d6d10718/report.json`,
  SHA-256 `c7592e12d70681717629247b91381fd8ca59db9326189ec0ca48a7ff8d1e649d`.
- Automatic report: `/private/tmp/ejm-cli-capacity-comparison-20261020/20261021T034125Z-2e364127/report.json`,
  SHA-256 `bd4a64e4f9c790832762b7e2b36681b600a029ba7ea606e9a777100ecf4dca63`.

No task closes. Native Linux/Windows and crash acceptance, repeated workload
and static-sweep comparisons, terminal latency/memory/idle-CPU targets and
automatic defaults remain open. Final public-doc rebuild and wire-budget
checks are recorded separately below.

## Final docs checks after current-image evidence

Both sites passed serialized `npm --prefix site run check` and
`npm --prefix site run budget` after the current-image helper and CLI evidence
updates. Full captured source, symlinks, HEAD and index stayed unchanged during
each gate; all direct children were reaped. Receipts:

- Harness: `/private/tmp/ejm-capacity-consumer-required-gates-20261020/20261021T034615Z-67fe5002-harness-site/report.json`,
  SHA-256 `eb1d3e0b0c49ebcafbce6d923dae2bf87603195a587bd435db196aa5218c77c6`.
- CLI: `/private/tmp/ejm-capacity-consumer-required-gates-20261020/20261021T035335Z-bcf5a23d-cli-site/report.json`,
  SHA-256 `f601bb2b53a46417a2ae97f5285dbf69fdd03dc133e46a6c4fde151b812b9f91`.

Fresh local browser QA passed all seven derived gates: 16 layout captures,
32 behavior contexts, no tab-chunk requests on plain docs, both performance
reports, source/dist stability and cleanup. Coverage was the harness developer
guide and CLI developer guide, reference and benchmarks, with phone/desktop,
light/dark and scripts enabled/disabled. No console errors/warnings, page errors
or unexpected request failures remained. The raw report retains 116 same-origin
JavaScript CSP blocks with scripts disabled and 16 cancelled home requests
whose final destination and heading rendered. Their internal cancellation cause
is unproved. V4 corrects the capture tool's integer failure-count interpretation;
the failed V3 summary and all raw bytes remain unchanged. Independent read-only
review found no hidden behavior or performance failure.

Three throttled mobile loads per route and mode retained the unchanged 95 KiB
(97,280-byte), header-inclusive JavaScript wire budget:

| Site    | Route                         | JavaScript wire bytes |
| ------- | ----------------------------- | --------------------: |
| Harness | `/docs/developer-guide/`      |                95,141 |
| CLI     | `/docs/developer-guide/`      |                96,861 |
| CLI     | `/docs/reference/cli/`        |                96,950 |
| CLI     | `/docs/reference/benchmarks/` |                96,876 |

The tightest route has 330 bytes of headroom. CSS, font and HTML budgets also
passed; sampled CLS and TBT were zero. All direct phase/server children were
reaped and Chrome closed. Minimum sampled free space was 24,919,728,128 bytes.
The V4 summary is
`/private/tmp/ejm-capacity-site-final-qa-20261020/20261021T035358Z-37217c58/qa-summary.v4.json`,
SHA-256 `b843d9902b6ca53851827ce755cad171c56870e2bbea33c1bf514e50eaa69688`.

The separate linkage verifier passed final log receipts, serialized chronology,
full gate-to-QA source equality, stable QA dist manifests and all six shared-UI
file pins from the earlier owner receipt. Linkage:
`/private/tmp/ejm-capacity-current-docs-linkage-20261021/linkage-20261021T035358Z-37217c58.json`,
SHA-256 `cdb396696b3fc8af66a4bcc01aa496c69594f7bb8e7731be227876c528374146`.
Dist was first captured at QA start, so the receipt cannot prove that built
files were untouched between build exit and that capture. Ignored dependency
trees are not fully frozen.

Root reviewed eight viewport content images covering all four routes, both
sizes and themes, with no actionable layout defect observed. All 16 tall
captures were truncated; omitted content, physical devices, screen readers,
deployed Worker headers and external links were not reviewed. These local lab
results establish neither field performance nor publication. Only this
verification record changes after the linked checks. Task progress stays
32/39 in the CLI and 40/45 in the harness; broader acceptance and automatic
defaults remain open.

## Required gates and shared docs repair on 2026-10-20

The literal CLI `./scripts/check` passed in 131.618 seconds after the approved
TLS repair and npm cache restoration. It passed Rust format/lint/build/test/doc,
dependency checks, terminal restoration and interaction, relay native/local
fixtures, WASM build and browser checks. CLI, harness and shared-UI source,
symlinks, HEAD and index stayed unchanged. The direct gate child was reaped.
Receipt:
`/private/tmp/ejm-capacity-consumer-required-gates-20261020/20261021T025242Z-63379e59-cli-gate/report.json`,
SHA-256 `9506e54e5b95ed15a7af374666518e2f445356bf29cfcb9ea575a5db91eaec35`.
Together with the focused TLS evidence below, this closes task 8.4. The earlier
missing-cache failure remains retained; normal locked `npm ci --ignore-scripts`
restored the cache without source or lockfile changes.

The shared docs repair resolves the existing tab frame before synchronous
server rendering, then loads it lazily in browsers through a matching local
Suspense boundary. It reuses the frame, renderer, reducer and preferences,
without changing public exports or dependencies. Shared guidance was updated.
The focused owner check passed 12 tests across four files, TypeScript, the
tree-shaking fixture and source-entry checks. It covers cold SSR without an SSR
flag, custom code rendering, complete server panels before delayed hydration,
one browser import, keyboard/shared selection and copy after hydration, and
the absence of console or hydration errors. Receipt:
`/private/tmp/ejm-capacity-ui-docs-checks-20261020/20261021T024253Z-fc4d24f4/report.json`.
The six owned files still match that receipt. An unrelated later change to
`packages/ui/scripts/check-sources.mjs` was preserved.

Both rebuilt product sites passed `npm run check` and `npm run budget` with
fully stable source. The CLI built-HTML suite passed 94 tests. Receipts:

- Harness: `/private/tmp/ejm-capacity-consumer-required-gates-20261020/20261021T025519Z-74b91f50-harness-site/report.json`.
- CLI: `/private/tmp/ejm-capacity-consumer-required-gates-20261020/20261021T025550Z-5a7171e1-cli-site/report.json`.

The subsequent local browser pass used those rebuilt files. All 16 capture
checks and 32 interaction contexts passed their derived gates across four
routes, two viewports, both themes and scripts enabled or disabled. No tab
chunk was requested on the plain docs pages during initial load or reload.
There were no console errors/warnings, page errors or unexpected request
failures. The raw interaction report flags 116 script requests blocked in
disabled-script contexts and 16 cancelled home requests whose destination
rendered successfully; those observations remain classified and retained.
The capture tool returns an integer failure count. A new summary adapter fixes
the original Boolean type check without editing the raw reports or v3 summary.

Three throttled mobile samples per route and mode retained the 95 KiB
(97,280-byte) JavaScript wire budget, including response headers:

| Site    | Route                         | JavaScript wire bytes |
| ------- | ----------------------------- | --------------------: |
| Harness | `/docs/developer-guide/`      |                95,141 |
| CLI     | `/docs/developer-guide/`      |                96,861 |
| CLI     | `/docs/reference/cli/`        |                96,950 |
| CLI     | `/docs/reference/benchmarks/` |                96,876 |

All CSS, font and HTML budgets passed. Sampled CLS and TBT were zero; these
local lab samples do not establish field performance. Full source and built
files stayed unchanged during QA, all direct children were reaped, and Chrome
closed. Final summary:
`/private/tmp/ejm-capacity-site-final-qa-20261020/20261021T025609Z-6d6fef32/qa-summary.v4.json`,
SHA-256 `4a5076bfd596a0fb47bdfee8cb534543e4af2162695cefd7a46ca415a4a185b4`.
Source/build linkage and its limits:
`/private/tmp/ejm-capacity-docs-build-qa-linkage-20261021.json`, SHA-256
`55b277173d42039be19a0f42d9b3684671dd393517c857c7fa9c19225392c413`.
A separate read-only image review found no actionable defect in the eight
original-detail harness-developer and CLI-reference viewport images, covering
both themes and sizes, plus their eight reduced tall overviews. Tall captures
were truncated; omitted content, native devices and screen readers were not
reviewed. This closes task 8.5, not publication or wider performance acceptance.

The IDE's post-adoption public path-consumer check also passed
`cargo check --offline --locked --tests -p ejm-ide`. Its only source change
removed the registry identity from `rightkit-fs 0.4.4` in `src-tauri/Cargo.lock`;
no version or dependency edge changed. Metadata selects the harness vendor
manifest with `fs`, libc and thiserror. Generated inputs stayed unchanged and
all command children were reaped. Completion:
`/private/tmp/ejm-capacity-ide-post-adoption-20261020/run-500adae73d3748d99be77fef336d0ff9/completion.json`;
its report SHA-256 is
`119b49a05b27c928ed05e095e8893d17aa9bc2df068593499acb81685459b6a5`.
This compiles test targets but runs no IDE tests. Current-image calibration
was still pending at that checkpoint and is recorded separately above. Native
Linux/Windows, whole-turn 4096-agent acceptance, fair fixed/automatic comparisons
and automatic defaults remain open.

## Verified before the live matrix

- Locked offline all-target CLI check and strict all-target Clippy passed.
- The full CLI suite passed: 163 unit tests and 12 integration tests; five
  internal fixtures were ignored by the ordinary run.
- Fresh Unix PTY checks passed against the current production and release test
  executables, including terminal restoration and real local provider fixtures.
- The omitted-tail view test rendered an omission notice while preserving
  durable bytes, cursor and authority. Five focused view tests passed.
- The public benchmark response-marker regression passed.
- Python compilation and Ruff passed before the journal-envelope follow-up.
- The previous site gate passed 99 tests, typecheck, build budgets and headers.
- Strict capacity validation passed before the follow-up.

The earlier full CLI and architecture gates were attempted and stopped at missing
`wasm-bindgen`. Nothing was installed and no gate was weakened. Linux, Windows,
live-provider and full relay/WASM/browser acceptance remain unverified.

## Historical live-matrix fixture inputs

Build: `cargo test --release --offline --locked --no-run -p ejm-cli`, with
`CARGO_BUILD_JOBS=2`, `CARGO_INCREMENTAL=0` and the shared Cargo lock.

- Production helper: 17,892,512 bytes; SHA-256
  `e29555413618192a8de447a6fcdd0c0a4dc707d080fa46b662b39504586c902e`.
- Release test executable: 19,072,480 bytes; SHA-256
  `29ad4f6236d641fc83ab858dac160c322a4c50fd2dcc387fef86055acaec6f43`.
- macOS arm64 unsigned archive smoke passed. Archive: 8,135,802 bytes; SHA-256
  `7f5cfe5517e55d11c5ad38c931192121b6ad0d12e68180d480f5af3f25ec5583`.

The checkout is dirty. Selected instrumentation hashes do not establish a
complete immutable build provenance record.

## Live measurements and failures

`docs/evidence/capacity-live-summary.json` links all 12 raw cases and preserves
their counters, populations, memory snapshots and boundaries. Public results
and limits are in `docs/benchmarks.md` and the developer guide.

- Stop cases at 100, 1000 and 4096 managed agents recorded p95 input, navigation
  and acknowledgement below 50 ms in this local fixture. Each recorded 20
  distinct stopped sessions and no failures. General response-paint p95 remains
  unverified because the larger cases have only three and two samples.
- The 30-second 4096 interactive/headless pair missed the 5% throughput target
  by recording 30.368% less useful completed work. Both have initial completions
  only. Keep this result separate from the longer pair.
- The 120-second pair counted 8192 successful turns each. Interactive also
  recorded 4096 failures; headless recorded none. Equal success counts do not
  establish acceptance.
- The interactive journal reached exactly its inherited 512 MiB budget with
  no free pages. Durable state retained third-turn starts without terminal
  outcomes. This strongly supports storage exhaustion; individual failure text
  was not retained, so the literal SQLite error remains unavailable.
- Zero trace omissions do not prove loss-free selected-event observation:
  receive errors resubscribe without counting lagged messages.

## Verified test-only follow-up

The internal fixture now has an explicit bounded journal setting, supplied as
2 GiB by the observer, and records physical main/WAL sizes plus at most eight
failure samples with bounded text. The observer retains its report and fails
when workload or drain failure counters are nonzero, independently of libtest
success. Public storage defaults and runtime behavior are unchanged.

The all-target locked offline check and strict Clippy passed after these edits.
The focused Rust failure-diagnostic test passed, as did both Python report-verdict
regressions. The current full CLI suite passed: 164 unit tests, 12 integration
tests and five ignored internal fixtures. Rustfmt, Python compilation and Ruff
passed before the replacement pair. The earlier site gate passed 99 tests;
final evidence and documentation checks are recorded below.

### Replacement build and pair

The same offline locked release command produced the frozen pair inputs, with
two Cargo jobs, incremental compilation disabled and the shared Cargo lock:

- Production helper: 17,892,512 bytes; SHA-256
  `2347ddd37cf1efc8e19ea0aff05e2a5982d46291a5baf979653003125976c150`.
- Release test executable: 19,072,496 bytes; SHA-256
  `1d349e5a9ef06ec250a27ecc7a7f86925351c4457c0c0d9797e348a8d3cd89e0`.

`docs/evidence/capacity-live-4096-journal2g-summary.json` retains selected source
hashes and the two raw report hashes. This series includes the fixture changes
and the harness prepared-composition guard. Historical source/binary hashes stay
unchanged; this is not a controlled comparison with the prior build.

The 120-second 4096 pair ran headless first, then interactive. No team build,
test, site check, preview server or other team timing workload ran during the
pair. Headless counted 8085 successful turns at 67.375726 turns/s; interactive
counted 8192 at 68.267664 turns/s. Both recorded zero active/drain failures,
zero active cancellations and no failure samples. Each had 4096 pending turns
at cutoff, all cancelled during drain, with zero drain completions. Journal
main/WAL bytes were 460,468,224 / 362,592 headless and 504,303,616 / 354,352
interactive, under the explicit 2,147,483,648-byte envelope.

Headless initial/restarted sample counts were 4096 / 3989; interactive was
4096 / 4096. Initial/restarted p95 was 57.824 / 65.697 seconds headless and
51.708 / 59.716 seconds interactive. The interactive input, worst picker and
switch p95 values were 33.497, 33.997 and 33.537 ms, with 20 samples each.
Tracing and Stop were disabled. No response-paint or trace-overhead conclusion
follows from this pair.

The apparent interactive loss was -1.323826%. This single pair did not miss the
5% target. Different restarted-turn populations, cutoff sensitivity, uncontrolled
host load and the retained earlier miss prevent general acceptance. No scale,
package, I/O or native/default acceptance task was closed from this evidence.

### Final evidence checks

- Final Rustfmt, Python compilation, Ruff, both Python report-verdict regressions
  and strict capacity OpenSpec validation passed.
- The final CLI site gate passed typecheck, build budgets, headers and 99 tests.
- Local desktop previews showed the replacement table and its limits in light
  and dark themes. The developer capacity guidance was also reviewed locally.
  Phone-width and scripts-disabled previews were not repeated for this prose
  update. The temporary browser tabs and loopback server were closed.
- An independent read-only review found no defects in the replacement counts,
  rates, latency populations, hashes, journal scope, cutoff/drain accounting or
  docs. It also confirmed all 12 historical raw hashes still match their manifest.
- That full-gate attempt stopped at missing `wasm-bindgen`. The later startup
  follow-up below reached workspace tests and stopped at sandbox socket denials.
  The full gate remains unverified. No tool was installed and no required gate
  was weakened.

At that checkpoint, progress was 17/37 after task 2.4. Task 7.5 still needs startup/steady-state I/O and package
feature-cost evidence; the successful pair closes no broad scale, package or
native/default task. Automatic CLI defaults remain gated. No release, public
site publication, commit or deployment occurred.

## CLI startup and shutdown lifecycle follow-up

Task 2.4 is complete after root review of CLI startup, shared package lifecycle
and native-owner evidence. Native platform and performance gates stay open.
No automatic default, public protocol, production dependency or capacity limit
changed in this follow-up.

The CLI installs one termination listener before capacity preparation and package
loading, passes cancellation through package imports and plain-run provider
preparation, and retains that listener into the run or terminal loop. Cancelling
startup awaits the in-progress owner and cleanup. A prepared capacity clone stays
alive through harness shutdown followed by package shutdown. Cleanup diagnostics
retain the original typed operation cause and report a second cleanup failure.
The terminal sign-in handoff still re-arms its listener after a child receives
Ctrl+C.

The new Unix real-binary tests use public package installation and explicit
`--capacity fixed`. They signal only the CLI PID and verify recorded interpreter
PIDs have exited after the CLI completes cleanup:

- Node and Python blocked imports each receive SIGINT and SIGTERM.
- Import failure after Node startup and attempted Python import, theme failure,
  and missing package-provider selection each check every initialized host.
- Normal provider completion checks both initialized language hosts.
- SIGTERM during blocked Claude status preflight checks the package host and
  the provider-preparation child. The executable fixture supplies the status
  gate; production startup has no fixture mode.

The original sandboxed focused attempt could not bind its local provider fixture.
The allowed native run then passed all four startup tests in 25.54 seconds.
A temporary mutation removed provider cancellation. After correcting an initial
unused-variable compile error, the mutation reached execution and failed the
preflight regression at its expected startup-cancellation diagnostic. The source
was restored before final checks.

Final current-source checks, with two Cargo jobs, incremental compilation off,
offline/locked Cargo commands and the shared Cargo lock:

- Rustfmt passed. Strict workspace/all-target Clippy passed with warnings denied.
- The focused original-cause/cleanup-diagnostic unit test passed.
- The CLI all-target test command passed 165 unit tests and 16 real-binary
  integration tests, with five internal fixtures ignored. Unit execution took
  5.72 seconds; integration execution took 26.77 seconds. Native fixtures ran
  outside the sandbox to allow local sockets and controlled child signals.
- Five existing focused PTY checks passed with current debug executables:
  configured normal exit, configured SIGTERM, unconfigured normal exit, Claude
  sign-in handoff, and SIGTERM during sign-in. They check terminal flags, cursor,
  alternate screen, sign-in child reaping and private-output exclusions. The
  unrelated remote PTY cases and performance matrix were not repeated.
- Strict `adaptive-agent-capacity` OpenSpec validation passed.
- The site gate passed typecheck, build budgets, headers and 99 tests. Local
  desktop previews showed the new startup guidance in light and dark themes,
  with an empty browser console. Phone-width and scripts-disabled previews were
  not repeated for this prose update. Preview tabs and the server were closed.

The required `./scripts/check` was attempted again. Its tools were available on
this attempt. Four Python tests, architecture/skill checks, Rustfmt, strict
workspace/all-target Clippy and fixture build passed. The gate then stopped in
sandboxed workspace tests: nine existing local socket fixtures returned
`Operation not permitted`; 156 unit tests passed and five were ignored. The
separate allowed CLI suite above passed those tests. Later full-gate steps,
including release build, docs/dependency checks and relay checks, were not
reached by this attempt. No tool was installed and no gate was weakened.

This is controlled cleanup evidence from macOS on the current dirty checkout.
It does not prove detached-descendant containment, forced-termination recovery,
shared-owner crash recovery, Windows or Linux behavior, broad throughput
acceptance, or production release availability. Shared runtime tests own
reload cleanup, grant retention until child exit, and multi-owner accounting.
Root marked task 2.4 complete after combining the CLI, package and native-owner
evidence. No commit, publication or deployment occurred.

## Shared contracts and session navigation follow-up

The public prepared-context and package-runtime contract review closes tasks
1.1 and 2.3's functional scope. Preparation precedes provider/hook/package work
and final composition reuses the same governor and grant. Existing legacy
literals, changed-envelope rejection, worker omitted-capacity semantics, SDK
facades and local authority fixtures remain covered by the checks above and
the harness verification. Public package fixtures cover lazy admission,
resident charges, callback increments, import/reload cancellation and cleanup.
Their timing and platform gates remain separate.

Task 2.5 uses the public reset interface, rejects `--all`, and adds no remote
mutation. Isolated harness fixtures preserve sessions/events, encrypted
credentials, artifacts, active leases, provider accounting, explicit caps and
grants. The user's host registry was not reset. Tasks 5.1 and 5.4 combine the
bounded 50-row picker and 32-cursor history, cancelled obsolete queries and
stale-generation rejection with the earlier large saved-session and site checks.

Two further navigation fixtures pass in 0.20 seconds after a 38.10-second debug
build. They seed 1003 saved sessions, select an ID outside the recent 1000 and
resume it through both initial selection and `/resume`. They also check unique
and missing prefixes, ambiguity across more than one 50-row page, exact failed
draft preservation, Down/Up page crossings, Enter while loading and Escape.
`docs/evidence/capacity-session-navigation.macos-arm64.json` retains the command,
fixture/binary/log hashes and source-freeze limit. This closes task 5.2. The
timings overlap development and establish no performance budget. The new real
binary text/JSON listing and output-error fixtures await disk space.

The harness restored additive protected-main completion subscriptions and
native vault cancellation/deadline waiting without replacing capacity,
scheduling, encryption or serialization. Fourteen core and 21 vault tests pass.
Strict core/auth Clippy stopped in two executor draft findings; their owner
fixed both, and the rerun waits for disk space. Immutable release pins and final
combined consumer checks remain integration gates.

The fresh local docs gates pass 39 harness and 99 CLI tests, typechecking,
builds and route budgets. Two stale homepage claims about the removed replay
provider were corrected. Site logs are
`/private/tmp/ejm-capacity-harness-site-20261020.log` and
`/private/tmp/ejm-capacity-cli-site-20261020.log`. No publication occurred.

## Bounded recovery and complete session listing

Current-source checks on macOS arm64 pass 177 CLI unit tests and all 18
real-binary integration tests, with five internal fixtures ignored. The
executor suite passes 59 tests with two internal fixtures ignored. Strict
all-target Clippy passes for the CLI and executor/core/auth crates with
warnings denied. Cargo ran offline and locked, with two jobs, incremental
compilation disabled and the shared Cargo lock. Native fixtures had system
access for private loopback sockets, process inspection and controlled-child
cleanup. These debug checks establish behavior, not performance budgets.

The selected view subscribes before loading a bounded snapshot, captures a
fixed recovery endpoint and yields between bounded application batches.
Thirteen focused view fixtures cover unselected floods, snapshot races,
continuing appends, selected-stream continuity, duplicate suppression,
oversized events, long history and stale generation or session completions.
Two input-handler regressions exercise real Ctrl+C and Enter. Historical
completion cannot make a current run idle during recovery, and a stale idle
snapshot refreshes when a newer accepted local action changed control state.
Current Stop state survives historical batches and tail finalization.

The sessions integration fixture seeds 125 sessions and verifies every page
appears in complete text and one JSON array. It also checks output
sanitization and a closed output pipe. A second fixture verifies invalid
storage errors in text and JSON modes. The first output-error attempt used
a read-only descriptor whose behavior differed on this Mac. The final
fixture closes a standard-library pipe reader before launching the CLI.
Earlier failed fixture, linking and lint logs remain retained.

`docs/evidence/capacity-bounded-recovery.macos-arm64.json` records commands,
counts, log hashes, selected source hashes observed after the checks and
their provenance limits. It supports tasks 4.1 through 4.4 and 5.3. The
independent public-contract review supports task 8.3: fixed omissions,
legacy constructors, public prepare/load/compose ordering, relay wire and
provider-window recovery retain their contracts. No scheduler, probe,
execution ledger or persistence owner was copied into the CLI.

At that checkpoint, task 1.4 stayed open because the shared native helper did
not yet have a separate memory/process baseline charge. The package reservation
covered the interpreter. The helper-accounting and staged-admission follow-up
below records the later implementation and its verification. CLI performance
tasks and native platform gates stay open.

The runtime fixes enforce the existing documented recovery and Stop
contracts, so they add no new public command or workflow. Homepage and docs
metadata now describe real provider setup without shipped replay responses.
The later site gate passes 99 tests, typecheck, build budgets and headers in
`/private/tmp/ejm-capacity-cli-site-replay-cleanup-20261020.log`.

## Helper accounting and staged admission follow-up

The CLI keeps its narrow synchronous call to the public
`ejm_harness_exec::native::dispatch_helper` before Clap, ordinary Tokio, auth or
terminal setup. The harness owns helper framing, authentication, admission,
launch, cleanup and recovery. Protocol and lifetime traffic use bounded
authenticated private local connections. A separate Unix kernel witness, the
locked owner-file description, is inherited through helper stdin before its
runtime starts and remains held through runtime teardown. The protocol sockets
are not inherited. User children do not inherit the witness.

The harness now accounts for one on-demand coarse helper commitment per occupied
generation, independent of its creating participant. The pool holds at most
eight pending, running, paused or draining generations. Each committed helper
reserves a provisional 32 MiB and one PID; Linux task constraints count its three
runtime task slots. Interpreter memory and command reservations stay separate.
Waiting agents do not create a helper each. A timeout or dropped permit does not
refund a commitment. This is an admission estimate, not enforced memory
containment. Startup-peak and native platform gates still control automatic
defaults.

Startup and helper reuse consume public staged admission. Demand enters the
bounded local queue without a resource lease, retains queue order and
cancellation, and releases an unused readiness hold before waiting for resource
activation. Activation checks the installed grant generation before effects.
Package startup stages its complete resident/import peak and splits that admitted
envelope into its owning leases once. Shared helper selection must preserve a
useful dependent turn after retained resident floors. The CLI adds no controller,
registry, estimate, per-job shared write or lifecycle ledger. The canonical
contracts remain in the harness's `docs/executor.md` and
`docs/host-capacity.md#helper-commitments`.

### Frozen-source consumer checks

The final CLI window ran on macOS arm64 from 18:51:02Z to 18:55:24Z on
20 October 2026. It held the shared workload lock for the sequence, used at most
two Cargo jobs, disabled incremental compilation, used offline locked Cargo
commands, and enforced a 10 GiB free-disk floor. Existing Node 26.12.0 and npm
11.20.0 were selected only for those subprocesses. All 289 scoped harness/CLI
input hashes stayed unchanged. The checkout remains dirty; these scoped hashes
are not a committed release provenance record.

Evidence is under
`/private/tmp/ejm-capacity-consumer-refresh-20261020/`:

- `cli-final-source.provenance.json` records commands, source manifests, tool
  versions, timestamps, disk checks and binary receipts; its matching summary
  reports nine successful commands and no changed inputs.
- The all-target CLI check and Cargo Deny passed. Advisories, duplicate/version
  bans, licenses and sources all passed after the path dev-dependency version
  and two exact upstream duplicate exceptions were fixed. `Cargo.lock` stayed
  unchanged. Contributor documentation records the incompatible upstream ranges
  and the conditions for removing those exceptions.
- The fresh CLI suite passed 177 unit tests and all 18 integration tests, with
  five internal fixtures ignored. The Claude fixture and production release
  build passed. The production executable is 18,093,360 bytes, SHA-256
  `b74cd06bde4d61249afe2835cef62f4fa7806692d5ce19fee9a99f11fc60a77f`.
- The full Unix PTY check passed with the fresh production and native test
  executables. It covers configured/unconfigured startup, sign-in handoff,
  handled signals, remote interaction, private output and terminal restoration.
  All three recorded executable hashes stayed unchanged through the sequence.
- `cli-recovery-regressions.json` binds the full test log to the source manifest
  and records the existing full-ID search beyond 1000 sessions, completion during
  loading, and historical completion during recovery/Stop regressions. All three
  passed without a separate focused rerun or added test.
- Direct relay phases passed using existing dependencies: fixture WASM build,
  TypeScript/schema checks, 85 tests across seven files, production controller
  build, Worker dry run, local Worker smoke, two-native-client round trip, and
  all four browser checks. The production controller WASM is 484,136 bytes,
  below the existing 1.5 MiB budget. The dry run did not publish the Worker.

The earlier Node 26 `./scripts/check` attempt passed through strict workspace
Clippy, workspace tests, release build, warnings-denied Rustdoc and Cargo Machete,
then stopped at the dependency findings above. The final sequence verified those
fixes and the later terminal/relay phases directly. It did not run `npm ci`, so
the complete repository gate remains recorded as incomplete. No package was
installed and no gate was weakened.

The CLI site gate also passed with existing dependencies: typecheck, build,
headers and 99 tests; all 14 routes met the existing transfer budgets. Its 1,012
scoped CLI and public UI inputs stayed unchanged during the check and at the
native-freeze checkpoint. `cli-site.provenance.json`,
`cli-site-native-freeze-checkpoint.json` and `cli-site-rendered-content.json`
retain that evidence and the rendered contributor paragraph. This was a prose
and evidence update with no layout change; browser layout/performance checks
were not repeated. No site publication occurred.

### Shared-owner execution evidence

The final executor run passed 76 tests with two ignored fixtures, strict Clippy
and formatting in
`/private/tmp/ejm-native-diagnostic-checks-20261020/20261020T182234Z-e34dc696/`.
Stable executor input hashes bind the executed regressions to that source:

- `parked_readiness_shares_one_connection_and_pins_its_generation` and
  `staged_demand_precedes_helper_readiness_and_spawn_failure_releases_witness`
  cover shared readiness and demand before helper startup.
- `reused_helper_readiness_is_released_before_waiting_after_failed_activation`
  and `stale_readiness_cannot_launch_in_a_replacement_generation` cover reuse
  waits and stale readiness after helper replacement.
- `staged_demand_keeps_resident_floor_visible_without_a_job_lease`,
  `staged_queue_bound_fifo_cancellation_and_drop_share_the_existing_queue` and
  `staged_activation_rejects_changed_fences_and_keeps_its_prospective_turn`
  cover lease-free staging, queue bounds/order, cancellation and generation
  checks before activation.
- `resident_transfer_keeps_retained_output_memory_with_the_job` and
  `activated_startup_envelope_splits_without_recharging_or_releasing_early`
  cover the complete startup envelope and its one-time transfer into leases.

The harness's final workspace gate ran from 18:55:24Z to 18:57:22Z on
20 October 2026. Its 129 recorded runtime inputs stayed unchanged and match all
129 overlapping harness inputs in the final CLI manifest above. Its core suite
passed 131 tests. Executed registry cases cover one charge per generation,
retention after its creator exits, all occupied helper states, saturated-grant
drain, and pending helpers preserving a dependent turn. The retained-floor,
older-compatible-client and cancelled-staging cases passed too. These results
are in
`/private/tmp/ejm-zombie-aware-required-gate-20261020/required-gate.stdout.log`.

The same gate passed 21 package-runtime tests, with two ignored fixtures,
including `coordinated_launch_waits_before_starting_an_interpreter_and_cancels_cleanly`
and `coordinated_hosts_keep_resident_memory_without_holding_execution_slots`.
The CLI's final real-binary suite separately passed Node/Python startup failure,
signal, provider-preflight cancellation and normal-shutdown cleanup through
explicit fixed capacity. This combines public consumer execution with the
harness-owned accounting and cleanup checks.

The harness gate also passed strict workspace/all-target Clippy, native helper
and worker builds, host runtime checks, warnings-denied Rustdoc, Cargo Machete,
Cargo Deny, protocol freshness and ESLint. That attempt stopped at 33 Prettier
files before its SDK phases. Its provenance records that failure; these focused
results do not change it into a complete gate pass.

### Fresh worker and SDK receipts

Both SDKs were checked from 19:05:42Z to 19:06:00Z on 20 October 2026 against a
copy of the worker built during that harness gate. The shared workload lock,
offline/no-sync settings and 10 GiB free-disk floor remained in force. All 129
gate inputs still matched before and after, all 154 scoped runtime/SDK inputs
stayed unchanged, and the original and copied worker hashes stayed unchanged.
TypeScript output and tool caches used temporary directories; no checked-in
generated SDK files changed.

Evidence is in
`/private/tmp/ejm-capacity-consumer-refresh-20261020/sdk-final-gate-worker/`:

- `provenance.json` and `summary.json` record eight successful commands, source
  manifests, the gate match, timestamps, commands, logs and worker receipts.
- The native debug worker is 63,457,792 bytes, SHA-256
  `2fb750b83f57b3ec07c4a98be7dbbac1f395a4a1142014ac49cafbb7401683b7`.
  Its build timestamp is within the recorded gate window. This hash was captured
  after that gate, not in its original provenance record.
- Node 26.12.0 compiled TypeScript into scratch output and passed all 22 SDK
  tests against the pinned worker. Python 3.11.14 passed all 24 SDK tests,
  focused Ruff check/format and strict SDK mypy. No dependency installation,
  environment sync or Python distribution build ran.

Task 1.4 is complete for the verified macOS consumer path. Early public dispatch,
ordinary commands, authenticated bounded connections, charged helper startup and
residency, shared readiness, staged startup/reuse, useful dependent turns,
generation-fenced activation and one-time package startup accounting all have
executed evidence. Cleanup and admission remain in the harness, and the CLI adds
no per-job shared writes. Automatic defaults, startup-peak calibration, broader
scale budgets and unrun Linux/Windows acceptance remain gated. This task/evidence
update adds no public behavior; the public guides already cover the implemented
contracts and their limits. No commit, release, publication or deployment
occurred.

## Shared-fixture Python checks on 20 October 2026

Eight independent Python commands passed from 22:59:19Z to 22:59:20Z. This
records verification only and changes no public contract or claim. Existing
[active-agent fixture options](../../../docs/benchmarks.md#active-agent-terminal-fixture)
remain the public reference. Task 7.4 stays unchecked and progress stays 30/37.

Evidence directory:
`/private/tmp/ejm-cli-capacity-python-checks-20261020/20261020T225919Z-72684ac0/`.
Its `report.json` has SHA-256
`ed16d106442fe1caa56fe46345fc99ab64832555b03410f50f215348990d8545`.
It retains all eight full command arrays, expected and actual return codes,
timestamps, log paths and hashes, interpreter receipt, and source manifests.

All commands ran from the CLI repository root with
`/opt/homebrew/opt/python@3.15/bin/python3.15 -B` and
`PYTHONDONTWRITEBYTECODE=1`. The command names below match the report. Each of
the five rejection commands used `scripts/check_terminal.py --measure-live
/usr/bin/true --helper /usr/bin/true`, a distinct
`--fixture-root <evidence>/rejected-<case>` and
`--output <evidence>/rejected-<case>.json`, followed by its listed invalid value.
Here `<evidence>` denotes the evidence directory above and `<case>` is the
command-name suffix after `parser_rejects_`.

| Receipt command               | Checked arguments or behavior                                                                                | Return code |
| ----------------------------- | ------------------------------------------------------------------------------------------------------------ | ----------- |
| `python_syntax`               | `-c "from pathlib import Path; p=Path('scripts/check_terminal.py'); compile(p.read_text(), str(p), 'exec')"` | 0           |
| `measurement_verdict_tests`   | `-m unittest scripts.tests.test_terminal_measurement`                                                        | 0           |
| `live_parser_help`            | `scripts/check_terminal.py --measure-live --help`                                                            | 0           |
| `parser_rejects_mode`         | `--capacity invalid`                                                                                         | 2           |
| `parser_rejects_models_zero`  | `--model-requests 0`                                                                                         | 2           |
| `parser_rejects_models_large` | `--model-requests 257`                                                                                       | 2           |
| `parser_rejects_jobs_zero`    | `--jobs 0`                                                                                                   | 2           |
| `parser_rejects_jobs_large`   | `--jobs 129`                                                                                                 | 2           |

The measurement-verdict command executed two tests. They verify that active or
drain failures retain their evidence and reject the measurement, while
successful live and saved reports remain usable. All five invalid mode/bounds
cases exited in argument parsing and created neither fixture state nor an
output report. The commands were reviewed before execution: compilation does
not execute the source, the unit tests call only the report writer, and help
and invalid arguments exit before PTY setup or product/helper launch.

The scoped temporary root was empty after the checks. Before and after
manifests were identical for all 416 Git-listed files, symlinks, HEAD, index and
status during this check window. The three prepared CLI fixture file hashes
also stayed unchanged. Both source-manifest files have SHA-256
`1b7b95572740bf6e112c2dbc5dda53226d5f22bc8c8321259d0ada8b866bcd90`.

Rust builds and tests, native helpers, PTY/headless shared-root smoke cases and
the prepared all-stage runner remained on hold for the free-disk gate. That
runner was not imported or executed; its frozen SHA-256 remains
`2742eded1b005552727a76f40937bce1e2d1c27002c7ba470855b07d8f5eb013`.
These Python results do not verify the prepared Rust fixture patch, native
cleanup, shared-host progress, throughput, scale acceptance or automatic
defaults. No install, download, site build, release or publication ran as part
of these checks. This evidence-only addition needs no public docs revision.

## Shared-fixture attempt after cache cleanup

The frozen focused runner began at 23:24:39Z after the harness required gate
passed. It retained the shared workload lock, two Cargo jobs, locked offline
resolution, disabled incremental compilation and the 10 GiB free-space guard.
Ten preliminary phases passed, including scoped Rust formatting and the
independent Python syntax, report-verdict and parser checks.

The release Clippy phase logged only `Blocking waiting for file lock on build
directory`. It reached no Clippy diagnostic before free space fell to
9,895,112,704 bytes. The guard sent SIGTERM to its owned child, PID 56057, and
reaped exit code -15 without signal or wait errors. The report records final
logs, `all_commands_reaped=true` and `cleanup_requires_audit=false`. This was
a disk-guard stop during Cargo contention, not a Clippy finding or pass.

Receipt:
`/private/tmp/ejm-cli-capacity-fixture-checks-20261020/20261020T232439Z-db702278/report.json`,
SHA-256 `279af5f4845cfd5e89f0aced0645ecdd69a0da027d820a19fa3da9a13ab35417`.
CLI and harness before/after source manifests were identical. The frozen
runner and three fixture input hashes still matched their prepared pins.

Helper/libtest builds, the configuration compatibility test and both fixed
and automatic shared-root smoke cases were not reached. No native helper or
fixture launched in this attempt. A later read-only check found neither
Cargo target lock held; it did not identify or stop the other build. Free
space remained below the guard, so no retry ran. Task 7.4 remains open and
automatic defaults remain gated. This updates verification evidence only;
the public fixture reference and performance limits are unchanged.

## Verification-runner cleanup review

The original focused runner and its attempt receipt above remain unchanged.
Review found that its failure cleanup could reap the process-group leader
before escalation. A new scratch runner keeps that PID reserved through
SIGTERM, a bounded grace wait and SIGKILL before reaping. A second review
caught interruption of the grace wait skipping SIGKILL; V3 uses a nested
`finally` to attempt the last signal before waiting. Any interrupted sequence
or wait error requires audit and cannot claim final logs. Command and top-level
interruptions now record failed status before the final receipt save.

The reviewed runner is
`/private/tmp/ejm-cli-capacity-fixture-checks-20261020/run.v3.py`, SHA-256
`cea65782e1b17de984e2013ca9bb21dcffb93329f597f894ee1d9eaad4a5564f`.
It has not run native checks. Original and V2 bytes remain retained. Eight
stdlib mocked cases passed: success, signal permission failure, absent group,
wait timeout and interruptions at TERM, grace, KILL and wait. No child process,
signal or real grace wait ran. The receipt is
`/private/tmp/ejm-cli-capacity-fixture-checks-20261020/runner-v3-cleanup-static-20261020.json`,
SHA-256 `67922428ab893e625eaa87e2bf283bc4d03c9eca8f7e7c0357f7ecf271e8f2cd`.
Independent static review found no remaining concrete issue. Runtime cleanup,
configuration compatibility, shared-root progress and performance gates remain
open. No production source, default, public contract or public docs claim
changed in this verification-only follow-up.

## Coordinated smoke and exploratory matrix on 2026-10-20

The V5 focused runner passed from 2026-10-21 00:41:47Z to 00:43:25Z on this Mac.
All 16 phases passed: versions, scoped Rust formatting, Python syntax,
measurement-verdict and parser checks, strict release CLI Clippy, release helper
and fixture builds, configuration compatibility, complete hard-envelope
equivalence and native observer preparation. Its two five-second headless
shared-root cases passed, fixed then automatic. Each had two overlapping
participants managing 100 agents each, separate session stores and one shared
capacity root. Fixed completed 116/118 useful turns; automatic completed
199/200. Both checked shared ancestor budgets, zero workload/drain failures,
exact client/helper identities, original witness release and reaping of all
commands and observers. Short smoke counts establish no throughput improvement.

Receipt:
`/private/tmp/ejm-cli-capacity-fixture-checks-20261020/20261021T004147Z-70e58374/report.json`,
SHA-256 `478e87ed5c46fb09255031fa16f52d1f27823194ed4da700b51082f29f61e97a`.
The frozen helper is 18,093,312 bytes, SHA-256
`bbdce48e8517f9f55541d52f63495a240712e4b2310c5565a15e0e17e3ccb3a4`;
the fixture is 19,455,136 bytes, SHA-256
`e1638365848ef7ed0eb9fad9003aaa93c127d896d8bc7023d824467a3415df79`.

The subsequent matrix reused those images and planned 24 cases: 100/1000/4096
agents, fixed/automatic, headless/interactive and two alternating repeats.
Each case had two participants, separate session stores, a fresh shared capacity
root and a 30-second work window. Interactive cases used `--no-stop` and
`--no-event-trace`. Each participant had two model requests, one native job,
512 MiB memory, 2000 CPU millis, one heavy job, 32 processes, queue 256,
1 MiB output, 64 MiB artifacts and a 120-second job deadline. The runner held
the shared workload lock and 10 GiB free-space guard; no builds or paid model
calls ran inside the matrix.

The four first-repeat 100-agent cases passed workload and cleanup checks.
Participant completed-turn counts were 900/995 fixed headless, 802/909 fixed
interactive, 738/799 automatic headless and 771/800 automatic interactive.
Controller windows overlapped above 99.9% by start/stop-file observations,
without a shared barrier. Each participant retained 100 pending turns at cutoff,
later cancelled, with zero drain completions and zero recorded workload/drain
failures. Only completed-turn latency samples are available; unfinished turns
remain censored. The fixed interactive pair's apparent 10.38% loss exceeded the
5% target in that pair, with hypothetical cutoff-count bounds of -0.10% to
18.94%. These bounds are not observed throughput or a confidence interval.
One repeat establishes no throughput win, sustained regression or repeatability.

The fifth case, `r1-1000-fixed-headless`, failed with
`fixed JSON did not establish successful useful work`. Participant 1 completed
zero turns and 990 tools in about 30 seconds. The second raw participant report
records 66 completed turns and 1000 tools; the aggregate runner stopped before
adding that report to its completed participant summaries. Both raw reports
record 1000 pending turns at cutoff, all cancelled in drain, and zero workload
or drain failures. Nineteen cases were not run. Useful-turn progress at 1000
agents and scheduling behavior remain under investigation.

Matrix receipt:
`/private/tmp/ejm-cli-capacity-comparison-20261020/20261021T004334Z-eb616d8b/report.json`,
SHA-256 `ba58b988ad9bb803a07d32dba52418707b48227d07da5a2b963dd021d407cfbd`.
It retains `status: failed` and `cleanup_status: failed`. A later read-only
audit at 00:52:48Z verified exact birth exit for both observers, both fixture
clients and the known helper, plus all three original stable, unlocked witness
inodes. It performed no signal, workload, database write or timeout inference.
It leaves the failed matrix and original cleanup error unchanged and does not
prove arbitrary detached descendants ended.

Audit receipt:
`/private/tmp/ejm-cli-capacity-cleanup-audit-20261020/20261021T005248Z-f5c6c1e5/report.json`,
SHA-256 `8ae573c3d5c29f44ae4af8628939865fad95f110640bf66420e9d8b7dae84900`.
The original report remained hash-identical. Runtime source hashes matched the
documented freeze; concurrent site maintenance was explicitly excluded and its
path deltas retained. These results do not close task 7.4, broader scale/static
sweeps, native crash recovery or automatic-default activation. No task, default,
release or publication was completed by the smoke pass or audit.

## Startup lock race and 1000-agent retest

The selected 90-second fixed/headless diagnostic used two participants with
1000 agents each and the earlier two-model/one-job resource envelope. It
completed 2000/1998 turns and 2168/2234 tools. Participant 2 recorded one active
turn failure, `startup.lock already exists`; participant 1 had none and both
had zero drain failures. The attempt failed while its cleanup checks passed.
Both commands and observers were reaped. Receipt:
`/private/tmp/ejm-cli-capacity-comparison-20261020/20261021T005606Z-af14d6cd/report.json`,
SHA-256 `68ae37e465f0faca9a2396a16a0a810e5c2fd08fe9488e21247128e0d3d8a4bb`.

The first dedicated regression attempt failed before contention setup because
`/var is or traverses a symlink / reparse point, which is refused`. Its raw
`regression_reproduced: true` is incorrect. Preserve the original report and
interpret it as a fixture setup failure, not a reproduction:
`/private/tmp/ejm-native-startup-race-check-20261020/20261021T010037Z-2573aabe/report.json`,
SHA-256 `c884e1c098036f6a298828650cad91e8401e99da1577027ef2dd99fc3ed42d0b`.

After canonicalizing the temporary root, the test
`competing_startup_clients_share_one_stable_lock` validly reproduced
`startup.lock already exists` with eight synchronized clients before the
production correction. The retained command exited 101 as expected. Receipt:
`/private/tmp/ejm-native-startup-race-check-20261020/20261021T010124Z-6bd52895/report.json`,
SHA-256 `183a8c0f88d34a6a49e09b7ce1b79932364f5233735958e73758cb480671cb94`.

The correction accepts a competing creator's `AlreadyExists` result and opens
the same permanent witness with the existing single-link check. It does not
replace the inode, retry launches, or hide other I/O errors. The same regression
then passed, checking mutual exclusion across all handles and later reuse:
`/private/tmp/ejm-native-startup-race-check-20261020/20261021T010150Z-2c1680bd/report.json`,
SHA-256 `72f1d78e451df287bcb0c536a3090029d20fb9ea69307443320d51f02025dbff`.

The executor gate passed formatting, 77 tests and strict all-target Clippy,
with three ignored native fixtures. Source stayed unchanged during the gate.
Receipt:
`/private/tmp/ejm-capacity-buffer-gate-20261020/20261021T010214Z-099be9bd/report.json`,
SHA-256 `4bc47217419c44ffef3e768b57a871e28399ba99f420555d825fb23a424ebefa`.
This is a focused executor gate, not the full repository gate. It tests
the competing creator's `AlreadyExists` path. A separate possible
hard-link publication race remains under investigation, as recorded below;
startup and readiness failures are not established as eliminated.

Fresh V5 passed all 16 phases and both five-second shared-root smoke cases.
Fixed and automatic each completed 100/100 useful turns, with zero active/drain
failures and cleanup checks passed. All commands and observers were reaped.
Receipt:
`/private/tmp/ejm-cli-capacity-fixture-checks-20261020/20261021T010248Z-c387420b/report.json`,
SHA-256 `1f48e61f2c8acf823e18c878a5690ea9225e2e3791272801798ad121f1a64aff`.
The release helper with the tested EEXIST correction remains 18,093,312 bytes, SHA-256
`d4f538eb44e2a1c04d96ce2f3674eb8873bb4695993c521d581895a433a0d1e2`;
the fixture remains 19,455,136 bytes, SHA-256
`e3d5cdc6a35fe4dcf378e06a47109ef5cb3c06fab64b078336cd9979acbccab6`.
Earlier helper-memory calibration retains its earlier CLI and worker image
hashes and source checkpoint. It does not become calibration for these images.

The images with the tested EEXIST correction passed the selected 90-second fixed/headless retest at
1000 agents per participant, two model requests and one native job each.
Participant 1 completed 2000 turns and 2929 tools; participant 2 completed 2652
turns and 3000 tools. Both reported zero active/drain failures, 1000 pending
turns at cutoff, all cancelled in drain, and zero drain completions. Initial
completion p95 was 23.323/23.438 seconds; restarted completion p95 was
25.871/44.479 seconds, from 1000/1652 completed restarted turns. Unfinished
turns remain censored. Approximate controller windows overlapped by 99.993%.
The workload and cleanup checks passed, all commands and observers were reaped,
and recorded runtime sources remained stable. One `registry_snapshot`
discovery error recorded process identity unavailable after an owner exited;
the passed workload and cleanup verdicts do not establish that every
observation succeeded. Receipt:
`/private/tmp/ejm-cli-capacity-comparison-20261020/20261021T010820Z-81837ad0/report.json`,
SHA-256 `5157dc40502a49f7560b1f892d4035322b24cacb8009bfe6b0a1a88f1fb25ad5`.

The report keeps `acceptance_status: not_established`. This selected retest
establishes useful progress for its workload on this Mac, not a controlled
throughput improvement, fixed/automatic comparison, terminal-overhead bound,
stable tail, full scale/static-sweep acceptance or native recovery. The earlier
failed reports remain unchanged. No task completion, automatic default,
release or publication is established.

## 4096-agent cutoff and queue diagnosis

The selected `r1-4096-fixed-headless` diagnostic used the helper
and fixture images with the tested EEXIST correction recorded in the preceding follow-up. Two participants had
separate session stores and one shared capacity root. Each managed 4096 agents
for a 90-second active window with four model requests and one native job.
Each also had 512 MiB of memory, 2000 CPU millis, one heavy job, 32 processes,
a 256-job queue, 1 MiB of output, 64 MiB of artifacts and a 120-second job limit.
The model envelope differs from the two-model 1000-agent retest; those cases
do not form a fair speed or scale-cost comparison.

| Participant | Completed turns | Completed tools | Active failures | Drain failures | Cutoff pending | Drain cancelled | Elapsed ns  |
| ----------- | --------------- | --------------- | --------------- | -------------- | -------------- | --------------- | ----------- |
| 1           | 0               | 2960            | 0               | 0              | 4096           | 4096            | 89999377542 |
| 2           | 0               | 3020            | 0               | 0              | 4096           | 4096            | 90001323333 |

Both had zero drain completions. The top-level report failed with
`fixed JSON did not establish successful useful work` and retained
`acceptance_status: not_established`. Cleanup passed, all commands and observers
were reaped, and the recorded runtime sources remained stable. Receipt:
`/private/tmp/ejm-cli-capacity-comparison-20261020/20261021T011019Z-d13020e6/report.json`,
SHA-256 `87282eb18c27500a975bf1ff24ead7657e45b338427150953d27a34253b95394`.
The runner stopped its useful-work check after participant 1 failed it; the
retained participant 2 report supplies that participant's completion metrics:
`/private/tmp/ejm-cli-capacity-comparison-20261020/20261021T011019Z-d13020e6/r1-4096-fixed-headless/participant-2.json`,
SHA-256 `a1bb93421304b070aa8e6fb1e59d6ce0575bb94322cb3376120fa1e9e318cdb5`.

The read-only diagnosis generated at `2026-10-21T01:13:43.480602+00:00`
queried each participant's journal through a read-only SQLite URI, enabled
query-only mode and imposed a 30-second deadline per participant. It observed:

| Journal event             | Participant 1 | Participant 2 |
| ------------------------- | ------------- | ------------- |
| First model step started  | 2964          | 3028          |
| Any second model step     | 0             | 0             |
| Tool finished             | 2960          | 3020          |
| Run finished as cancelled | 4096          | 4096          |

The diagnosis is aggregate fixture evidence from a finite, censored window.
Started first model steps and finished tools are distinct counts. These
observations establish neither scheduling optimality nor indefinite
starvation. They preserve the failed useful-work verdict and the original
benchmark bytes. Diagnosis:
`/private/tmp/ejm-cli-capacity-4096-queue-diagnosis-20261020.json`, SHA-256
`f5611054a385951e2431fdf314be1f8022f8bff1a687faedaeecb2816351f0ff`.
No sustained throughput result, full acceptance, automatic default, release
or publication follows from this diagnostic.

## Automatic 1000-agent diagnostic and rejected longer window

The images with the tested EEXIST correction passed `r1-1000-auto-headless`, a selected 90-second
automatic-mode diagnostic with two participants managing 1000 agents each.
Participants used separate session stores, one shared capacity root and the
same two-model/one-job envelope as the passing fixed 1000-agent retest. Each
also had 512 MiB of memory, 2000 CPU millis, one heavy job, 32 processes,
a 256-job queue, 1 MiB of output, 64 MiB of artifacts and a 120-second job limit.

| Participant | Completed turns | Completed tools | Active failures | Drain failures | Initial completion p95 | Restarted completion p95 |
| ----------- | --------------- | --------------- | --------------- | -------------- | ---------------------- | ------------------------ |
| 1           | 2000            | 2672            | 0               | 0              | 28.196 s               | 38.458 s                 |
| 2           | 2000            | 2488            | 0               | 0              | 28.152 s               | 36.879 s                 |

Both had 1000 pending turns at cutoff, 1000 drain cancellations and no drain
completions. Each latency population contains 1000 completed initial turns
and 1000 completed restarted turns. Unfinished turns remain censored. The
approximate controller windows overlapped by 99.998%; file timestamps are
an approximation without a shared barrier. Workload and cleanup passed,
commands and observers were reaped, and recorded runtime sources remained
stable. Cleanup retained exact sampled client/helper identities and the stable
helper witness. Four `registry_snapshot` discovery errors recorded unavailable
process identities near owner exit. The passed workload and cleanup verdicts
do not establish success for every observation. Receipt:
`/private/tmp/ejm-cli-capacity-comparison-20261020/20261021T011510Z-318c9749/report.json`,
SHA-256 `3f53f9d7874546b31d10252feba82503b99e9d0adc6d580f36c635c19e5f048d`.

The run used helper SHA-256
`d4f538eb44e2a1c04d96ce2f3674eb8873bb4695993c521d581895a433a0d1e2`
and fixture SHA-256
`e3d5cdc6a35fe4dcf378e06a47109ef5cb3c06fab64b078336cd9979acbccab6`.
Its individual result establishes automatic-mode useful progress for this
workload on this Mac. It leaves `acceptance_status: not_established` intact.
The separate fixed/automatic diagnostics do not establish a repeated
throughput comparison, a sustained win or regression, stable tails, a fixed
sweep, full acceptance, native crash recovery or a new automatic default.

The attempted 180-second fixed 4096-agent follow-up was a configuration failure.
The existing terminal observer caps `--seconds` at 120; argument parsing
rejected 180 before CLI/native fixture launch. Both Python observers exited
with code 2, were observed exiting and were reaped. The top-level report and
aggregate cleanup verdict failed because the expected native identities were
absent. No native clients or helpers were recorded, and neither list of
unresolved identities contains an entry. This verdict is not evidence of
surviving native work. No 180-second workload ran. Recorded runtime sources
stayed stable and the original report remains unchanged:
`/private/tmp/ejm-cli-capacity-comparison-20261020/20261021T011653Z-fbca9c8d/report.json`,
SHA-256 `280ee2b26ad80dec9e0c100d8e81e5df5c63f577976818bc84eb7a5fc9219b2f`.

Both retained stderr files are 1432 bytes with SHA-256
`081a82eb8aefa241fbb7558148e191ccf9598695698e42eb1508867531ab61b1`:

- `/private/tmp/ejm-cli-capacity-comparison-20261020/20261021T011653Z-fbca9c8d/r1-4096-fixed-headless/observer-1.stderr`
- `/private/tmp/ejm-cli-capacity-comparison-20261020/20261021T011653Z-fbca9c8d/r1-4096-fixed-headless/observer-2.stderr`

They record `argument --seconds: invalid choice: '180'` and the allowed
5-through-120 range. Preserve this failed setup separately from the earlier
failed 4096-agent 90-second workload and any later diagnostic.

## Automatic 4096-agent diagnostic and remaining lock work

The final selected `r1-4096-auto-headless` diagnostic used the same measured
helper and fixture hashes recorded above. Two participants had separate
session stores, one shared capacity root and 4096 agents each. The active
window was 120 seconds with two model requests and one native job per
participant. Each also had 512 MiB of memory, 2000 CPU millis, one heavy job,
32 processes, a 256-job queue, 1 MiB of output, 64 MiB of artifacts and a
120-second job limit. This differs from the fixed 4096-agent diagnostic's
90-second window and four-model envelope. It is not an equal-envelope
fixed/automatic comparison.

| Participant | Completed turns | Completed tools | Active failures | Drain failures | Cutoff pending | Drain cancelled | Elapsed ns   |
| ----------- | --------------- | --------------- | --------------- | -------------- | -------------- | --------------- | ------------ |
| 1           | 0               | 3457            | 0               | 0              | 4096           | 4096            | 120001320250 |
| 2           | 0               | 3687            | 0               | 0              | 4096           | 4096            | 120000849708 |

Both had zero drain completions. No completed-turn latency population exists
for this case. The top-level report failed with
`auto JSON did not establish successful useful work` and retained
`acceptance_status: not_established`. Cleanup passed, all commands and observers
were reaped, and recorded runtime sources stayed stable. No discovery errors
were recorded in this case. Receipt:
`/private/tmp/ejm-cli-capacity-comparison-20261020/20261021T011853Z-776a6888/report.json`,
SHA-256 `a654d09209b21c6b9486669bebc0bb39112b156752298770a82453e5582a2ee5`.

The runner stopped its useful-work check after participant 1 failed it.
Participant 2's retained sidecar supplies that participant's completion metrics:
`/private/tmp/ejm-cli-capacity-comparison-20261020/20261021T011853Z-776a6888/r1-4096-auto-headless/participant-2.json`,
SHA-256 `194638f1e08c8b02587e8928e9f02fb92f0354733678bdcee58dd155785b82a5`.
The 3457/3687 finished tools establish native tool progress in the selected
fixture. The failed case does not establish completed-turn acceptance at 4096
agents, sustained throughput, a win or regression, a fair fixed/automatic
comparison, stable tails or native crash recovery. Earlier failed reports
and the rejected 180-second configuration attempt remain unchanged.

The tested startup-lock correction covers competing creators reaching
`AlreadyExists`; its before/after regression and focused gate remain valid
within that scope. Source review confirmed a separate publication gap:
`StateDir::write_new` hard-links a temporary inode into place, then unlinks
the temporary name, while `open_read` requires one link. A reader may encounter
the interval with two links, and publisher death can leave both names. Reviewed
`rightkit-fs` 0.4.4 and 0.4.6 public APIs do not supply direct private exclusive
creation through the existing pinned `StateDir`. The
[harness dependency blocker](https://github.com/ejmlabs/ejm-harness/blob/main/docs/host-capacity.md#stable-lock-creation-blocker)
records the alternatives and required API. The later full harness gate
reproduced the concurrent-reader failure in
`competing_startup_clients_share_one_stable_lock`; its unchanged-source failed
report is retained at
`/private/tmp/ejm-capacity-final-required-gate-20261020/20261021T013610Z-1fa89d84/summary.json`.
A publisher-death schedule, correction and verification are pending. The earlier startup
evidence does not establish elimination of startup or readiness failures,
and these exact measured images do not validate a future publication-race
correction. Full acceptance and automatic defaults remain gated. No task
completion, release or publication is established.

## Required-gate follow-up on 2026-10-20

The later literal CLI `./scripts/check` failed at `cargo-deny` on
`RUSTSEC-2026-0381`. Its dependency path is `tokio-tungstenite 0.30.0` through
`webpki-roots 0.26.11` to `webpki-roots 1.0.9`. The advisory reports trust in
distrusted emSign intermediates and identifies `webpki-roots 1.0.10` as the
patched bundle. Before that failure, the gate passed 179 CLI unit tests with
five ignored fixtures, 18 binary integration tests and five relay-web tests,
plus its formatting, strict lint/build, rustdoc and unused-dependency stages.
Those stage results do not make the full gate green. Receipt and logs:
`/private/tmp/ejm-capacity-consumer-required-gates-20261020/20261021T013923Z-f4b7fc60-cli-gate/`.

The same wrapper recorded concurrent changes to CLI `site/package-lock.json`
and harness `openspec/changes/separate-project-scope/.openspec.yaml`.
Its runtime/full source-stability verdicts were false; these unrelated changes
were retained. All direct children were reaped. This failed gate is not a
source-stable integrated acceptance result.

Checksum-verified crate sources and fresh primary metadata confirm that
`0.26.11` only re-exports the 1.x root bundle, and no published patched 0.26
bridge or newer `tokio-tungstenite` release was available in that review.
Updating only `1.0.9` to `1.0.10` removes the actual distrusted root but leaves
the wrapper flagged by the advisory's current version range. No advisory
exception, trust-store change or dependency update was applied. The proposed
explicit Rustls connector repair remains subject to its OpenSpec plan revision
and verification. Review:
`/private/tmp/ejm-cli-webpki-roots-review-20261020/report.json`, SHA-256
`72110796360782a2c2e7e7e033ab73e1b0194eb7922fe0b0afc0780559d01f06`.

The separately serialized CLI site `check` and `budget` commands both passed at
the captured snapshot. CLI and shared-UI sources stayed fully stable during
those commands and every direct child was reaped. Receipt:
`/private/tmp/ejm-capacity-consumer-required-gates-20261020/20261021T014751Z-4c5449eb-cli-site/report.json`.
Later source edits and local browser preview results are separate evidence;
this snapshot does not establish publication or a final current-source gate.

## Approved dependency repairs and focused verification

The owner approved both plan revisions on 2026-10-20. The temporary harness
filesystem patch and explicit CLI Rustls connector are now implemented. The
historical failed reports above remain unchanged.

The CLI now selects the pinned WebSocket crate's public Rustls connector and
uses bundled `webpki-roots 1.0.10` directly. Its locked all-feature graph no
longer contains the vulnerable 0.26 bridge or 1.0.9 bundle. A real local TLS
fixture proves untrusted-chain rejection, wrong-host rejection and a trusted
positive WebSocket/frame exchange. Exactly one successful HTTP upgrade carries
the ticket; the two rejected TLS connections do not reach that boundary.

The focused macOS run passed formatting, architecture, that TLS fixture,
existing remote round trips, strict all-target CLI Clippy, the all-feature
root graph and the advisory/license/source gate. The ordinary remote test
filter passed five tests and ignored two internal fixtures. Recorded CLI
source stayed unchanged and all direct children were reaped. Receipt:
`/private/tmp/ejm-cli-tls-repair-checks-20261020/20261021T022616Z-ad1d36ca-check/report.json`,
SHA-256 `c98e06f0fa71eb12c60813aac5741812d1bd88a4104f1da0991168fc0f61fa20`.
Earlier import and Clippy failures are retained in the sibling run directories;
the passing receipt follows their fixes without a lint allowance.

The harness adopted its exact checksum-pinned `rightkit-fs 0.4.4` patch using
an inherited workspace path dependency, so the CLI also consumes that source
when it is the Cargo root. Direct permanent creation and same-handle privacy
checks replace the hard-link publication path. Its focused macOS tests cover
creator death, competing creators, hard links, private mode and macOS ACLs.
The harness's canonical receipt is
`docs/evidence/native-stable-lock.macos-arm64.json`; adoption record:
`/private/tmp/ejm-capacity-stable-lock-adoption-20261020/adoption-completion.json`,
SHA-256 `8718ce322e16e2b981859885e752778deb795f329f4b1104e1785ba114762fdd`.

These focused checks do not establish native Linux/Windows support, a current
release-image calibration, whole-turn 4096-agent acceptance or a fair repeated
fixed/automatic throughput result. Both required repository gates and the
post-adoption IDE check remain pending at this checkpoint. Automatic defaults
and publishing remain gated.

The owner also approved the shared docs loading repair. Task 8.5 tracks its
implementation and checks in `ejm-ui` and both product sites. The 95 KiB
wire-transfer budget stays unchanged; no estimated byte reduction counts as
acceptance before the rebuilt pages are measured.

## Trace integrity, shared docs and required gates on 2026-10-21 UTC

This checkpoint keeps the change at 32/39 tasks. Task 8.5 was already checked;
no checkbox changed. The harness owns commit timing and package mutation
locking. The CLI consumes public traced subscriptions and owns its test-only
terminal observer. Shared docs loading remains in `ejm-ui`.

### Trace integrity

The harness focused run passed both `traced_selected_` tests with
`test-support` enabled. Its opt-in traced subscription preserves a timestamp
taken immediately after successful commit return and before publication,
including the original pending stamp through lag/reset handling. It adds no
durable event or relay wire field. Receipt:
`/private/tmp/ejm-capacity-core-trace-checks-20261021/20261021T045831Z-a2fce601-core-trace-focused/report.json`,
SHA-256 `2afeb0166dc440e8b7ee7a2357f125c311a13107fb5b2b9f6b276050c618a198`.

The CLI focused run passed four Rust `paint_trace_` tests and three Python
measurement methods covering 16 cases. Missing commit stamps never fall back
to receipt timing. Missing or partial samples and unavailable integrity
accounting remain incomplete, independently of useful-work success. Even a
clean diagnostic retains `selected_event_latency_acceptance: unverified`.
Receipt:
`/private/tmp/ejm-capacity-trace-integrity-checks-20261021/20261021T045902Z-c2a69f39-trace-focused/report.json`,
SHA-256 `ab9752dc51506a18095cab7ebeee9a5d1181616f0a55d983d10964721893c81a`.

These focused checks establish neither a release-profile latency population
nor the 50 ms target. Commit timing excludes precommit journal wait and the
physical display. First-paint correlation, selection coverage and trace
overhead remain unverified. The observer's 120-second cap is unchanged; the
earlier rejected 180-second attempt and its failed evidence remain retained.

Later source review clarified that each journal batch samples traced interest
before commit, so a batch already in progress may still take its timestamp after
the final subscriber drops. Public Rust API comments and the canonical harness
benchmark method now state that boundary. The CLI report's stale receipt-only
limitation now describes both clocks and their first-write/selection limits.
These are wording corrections; runtime timing and workload behavior are unchanged.

### Shared docs checks

Both later product-site `check` and `budget` commands passed. Recorded product
and shared-UI sources stayed fully stable during each run, and all direct
children were reaped. The 95 KiB, 97,280-byte wire-transfer budget is unchanged.
Receipts:

- CLI:
  `/private/tmp/ejm-capacity-consumer-required-gates-20261020/20261021T050029Z-07e291d3-cli-site/report.json`,
  SHA-256 `1c1eac232b4fabcb53d7946a718b75155711f03bd994b8f2ace5724234c5dc1d`.
- Harness:
  `/private/tmp/ejm-capacity-consumer-required-gates-20261020/20261021T050054Z-d78bd218-harness-site/report.json`,
  SHA-256 `c4eee5bff509b8fa2f3a26501ceacb4030648b8add70af41baf80c846135513a`.

Earlier local browser QA retains 16 captures and 32 interaction contexts over
four routes, two viewports, both themes and scripts enabled/disabled. Its
derived capture, behavior, no-tab-chunk, performance, stability and cleanup
gates passed. The raw interaction phase failed; its request observations remain
retained. The derived report classifies explicit scripts-disabled CSP failures
and canceled home requests whose destination rendered, with no unexpected
request or behavior failures. This classification does not prove the internal
cancellation cause. Version 4 also corrects capture failure-count decoding;
it reran no browser work and changed no raw evidence. Receipt:
`/private/tmp/ejm-capacity-site-final-qa-20261020/20261021T035358Z-37217c58/qa-summary.v4.json`,
SHA-256 `b843d9902b6ca53851827ce755cad171c56870e2bbea33c1bf514e50eaa69688`.

That earlier snapshot measured JavaScript wire bytes of 95,141 for the harness
developer guide and 96,861, 96,950 and 96,876 for the CLI developer guide,
reference and benchmarks routes. Its source/dist receipts remain separate
from the later builds. No new browser run or current built-file identity bridge
is established here. Human visual review, deployed Worker headers, external
links, physical devices and screen readers remain unverified. These local
checks establish no publication.

### Required gates and remaining failure

The first CLI gate stopped at the trace test's 102/100-line Clippy failure and
redundant `continue`. That failed report is retained:
`/private/tmp/ejm-capacity-consumer-required-gates-20261020/20261021T050405Z-2ffd143f-cli-gate/report.json`,
SHA-256 `6649debdf5712f38ddb0eab1e852029982549f0183f15fa6f14bb3835b9ffb67`.

After the lint correction, the next CLI `./scripts/check` passed Clippy and
184 unit tests with five ignored fixtures, then failed with 21/22 integration
tests passing. `startup::startup_failures_reap_all_loaded_package_hosts`
failed at `startup.rs:196`: `enable` immediately after successful `install`
reported `another package operation is running`, caused by a lock that would
block. The gate did not complete. Receipt:
`/private/tmp/ejm-capacity-consumer-required-gates-20261020/20261021T050503Z-92ed1162-cli-gate/report.json`,
SHA-256 `e86f2b9280473c518487bc4eb1de4908e4fee37063aa0bd5ffdd8522b24a1c6d`.

The log proves no cause for the retained lock. A descriptor inherited during
concurrent spawn is a supported hypothesis, not a confirmed diagnosis. The
proposed harness repair would explicitly unlock package mutation guards using
`scopeguard`, with a deterministic retained-description regression. It awaits
the owner's OpenSpec plan approval and is not implemented at this checkpoint.
It must not change transferred native lifetime witnesses, whose ownership
depends on retaining the locked open description.

The later full harness `./scripts/check` passed: 396 Rust test executions with
11 ignored, including its package crash/recovery fixtures. Receipt:
`/private/tmp/ejm-capacity-package-crash-checks-20261021/20261021T051634Z-9881afc2-harness-gate/report.json`,
SHA-256 `1b88770ce7e496b5649ddcc3266b3c507cb900d375044232744267e3567c9591`.
That owner gate does not clear the CLI integration failure.

All trace and repository-gate reports above record stable full sources for
their listed roots and reaped direct children. Ignored dependencies and
external directory symlinks are not fully frozen, and direct-child reaping
does not prove arbitrary detached-descendant containment. Report and retained
phase-log hashes were checked before this entry was written.

Full CLI acceptance, release-profile scale and throughput budgets, trace
overhead, other native platforms and representative low-end hardware remain
unverified. Automatic defaults stay gated. No release, deployment or
publication is established. This evidence-only entry changes no public
runtime contract; the affected local docs checks are recorded above.

## Historical release trace comparison on 2026-10-21 UTC

This evidence checkpoint leaves task 7.1 and all remaining acceptance tasks
open. No checkbox changed. The CLI owns this internal terminal fixture and
its docs; the harness owns the public traced subscription and native executor.
No runtime contract changed in this checkpoint.

The fresh release builder passed 16 phases and fixed/automatic compatibility
cases. Its full recorded CLI and harness sources stayed stable, with no listed
exclusions; commands and observers were reaped. Receipt:
`/private/tmp/ejm-cli-capacity-release-build-20261021-trace/20261021T053104Z-0f5729dd/report.json`,
SHA-256 `40aa2811b91b17f33aa174a5bb8267b715076e3205812c356e610e6dd8de8235`.

The comparison retained six 30-second interactive fixed-capacity cases, three
pairs in trace-off/on, on/off and off/on order. Each case used two participants
with separate session stores and a fresh shared capacity root. Each participant
managed 100 agents with two model requests and one native job; Stop was
disabled and each scripted interaction had one sample. Receipt:
`/private/tmp/ejm-cli-capacity-comparison-20261021-trace/20261021T053409Z-dc49b81a/report.json`,
SHA-256 `ade007cefd327919c5869b78ca08543850f876580d32e6edd3fa8d65acdf6d68`.

Both receipts and all 12 retained participant JSON reports were checked against
their recorded SHA-256 values and sizes. The frozen helper was 18,109,872 bytes,
SHA-256 `c386f2c07e9242284b3fff705d7b55a262e03cc6d02d2bf737587fb2fdc11d4a`;
the fixture was 19,637,392 bytes, SHA-256
`2ad9dd2ec4a35f454b201f3c22ea50e119ec39bcf402b136ab28692d7e01073f`.
Their retained bytes were hashed again and matched both receipts. These images
used the earlier separate trace subscription. Later observer revisions require
fresh measurements; this historical evidence stays unchanged.

The aggregate completed-turn rates were 39.695738/36.666799,
36.667732/29.687817 and 44.400595/39.999979 turns/s, without/with tracing.
Observed lower traced rates were 7.6304%, 19.0356% and 9.9112%. Repeat spread,
range divided by median, was 19.4803% without tracing and 28.1240% with tracing.
Each participant had 100 pending turns at cutoff, all cancelled during drain;
none completed during drain. The hypothetical pending-count ranges are not
confidence intervals or observed drain throughput. These varying, censored
windows do not establish causal overhead or the 5% target.

All useful-work and cleanup verdicts passed with zero active/drain failures.
Commands and observers were reaped, recorded sources stayed stable, and
cleanup retained no errors or unresolved client/helper identities. Approximate
controller-window overlap ranged from 99.961% to 100%, without a shared barrier.
Registry sampling recorded 20 discovery errors, with counts 4, 4, 3, 4, 1 and 4
in execution order. Nineteen reported native result 1/errno 3; one reported
result 2/errno 0. Their cause is unproven. Passed cleanup does not establish
error-free observation or arbitrary detached-descendant containment.

All six traced participant reports were `diagnostic_only`. They retained 44
receipt-to-write and 44 commit-to-write samples, with zero lag signals,
closures, observer errors, missing commit stamps or omissions. Each traced
participant recorded two selection changes and three subscriptions. The
separate observer can receive a marker after its first terminal write, and
selection changes lack a snapshot anchor. Commit timing includes publication,
queue and scheduler delay, but excludes precommit journal work and physical
display. First-write correlation, selection coverage, latency acceptance and
trace-overhead acceptance remain unverified. No 1000/4096-agent trace evidence
was added. The observer's 120-second cap is unchanged.

These six historical verdicts came from the frozen reporter before
`generation_resets` and `uncovered_saved_markers` existed. The revised parser
classifies the same raw reports as `incomplete` because that accounting is
absent. Their historical verdicts and missing fields remain unchanged.

The small, allowlisted public record is
`docs/evidence/capacity-trace-overhead.macos-arm64.json`, 21,128 bytes,
SHA-256 `8ef270e232ad6b3671c32a536125da99da07d87f5584c713cc828df1e80b6765`.
It retains selected workload counters, elapsed times, trace samples, image and
report hashes, discovery-error counts and two bounded raw discovery samples.
It excludes launch details, environment maps, source manifests and arbitrary
report content. The benchmark guide links this record, and the public developer
guide links the canonical diagnostic section. No new workload or build ran
while these docs were prepared; formatting and site checks remain for the owner.

The latest full CLI gate remains failed at
`startup::startup_failures_reap_all_loaded_package_hosts`: `enable` immediately
after successful `install` reported that another package operation was running,
with lock acquisition blocked. Its retained receipt SHA-256 was rechecked:
`e86f2b9280473c518487bc4eb1de4908e4fee37063aa0bd5ffdd8522b24a1c6d`.
The logs do not prove why the lock stayed held. The proposed explicit-unlock
repair awaits OpenSpec approval and is not implemented. The passed harness gate
does not clear that CLI failure. Automatic defaults, release and publication
remain gated.

## Selected-view trace and distinct rounds on 2026-10-21 UTC

The internal trace now observes the actual selected-view receive/apply path,
without a second subscription or observer task. It retires pending samples on
view-generation changes, counts saved/recovery markers as uncovered, and takes
the write timestamp immediately after successful terminal drawing. These hooks
remain test-only. The harness-owned offline workload now emits
`response:{prompt}:round-{round}:`; its existing multi-round regression proves
that neither round's marker is a substring match for the other. The CLI's
snapshot-race expectation uses the new marker format.

The guarded focused run passed the benchmark regression, three paint-trace
tests, thirteen selected-view tests, measurement-parser tests, formatting and
strict all-target CLI Clippy. Full recorded CLI, harness and shared-UI sources
stayed stable; all direct children were reaped. Receipt:
`/private/tmp/ejm-capacity-selected-trace-checks-20261021-v2/20261021T060712Z-c14e2d55-selected-trace-focused/report.json`,
SHA-256 `e72351a30362801447b284f176ba038ed625532a3990710ef58e2044d012161f`.

This is functional evidence, not a release-profile latency or trace-overhead
result. Earlier frozen comparisons retain their original observer and markers.
No acceptance checkbox changed. The package mutation-lock repair still awaits
the owner's plan approval; native lifetime witnesses retain their existing
release rules. Automatic defaults and publication remain gated.

## Frozen selected-view release measurements on 2026-10-21 UTC

The current-source release builder passed all sixteen command phases and both
fixed/automatic shared-root smoke cases. Its frozen helper is 18,109,872 bytes,
SHA-256 `ca6dd376f292e9210e1799d961b1b7f9889bdfabb3284fd342673d7a1ec3732f`;
the fixture is 19,620,752 bytes, SHA-256
`a96c083220412e7a800dc4ae24dd249f9820dd10d1cbc1fcea0322e449c99726`.
Receipt:
`/private/tmp/ejm-cli-capacity-release-build-20261021-trace/20261021T060946Z-823afc4c/report.json`,
SHA-256 `5e80308069aaf7a38980b0084ea9ac8087c5458854b74af1b9e0b294eba9c5bc`.

The new diagnostics used those frozen images before the separate package
operation-unlock repair. Two 100-agent trace-off/on pairs ran in alternating
order with 30-second windows. Single traced 1000- and 4096-agent cases ran
90-second windows. Each case had two participants, separate session stores,
one fresh shared capacity root, two model requests and one native job per
participant. Workload and cleanup passed in all six cases; recorded CLI and
harness sources stayed fully stable without exclusions, and commands and
observers were reaped. Receipts:

- 100-agent comparison:
  `/private/tmp/ejm-cli-capacity-comparison-20261021-trace/20261021T061226Z-36f72f4a/report.json`,
  SHA-256 `ecdf33b7329c95a4d316a15739dbf126f919829fffc3e35d77d9d2a91d72466b`.
- 1000-agent diagnostic:
  `/private/tmp/ejm-cli-capacity-comparison-20261021-trace/20261021T061526Z-2e617071/report.json`,
  SHA-256 `007b44fcb0faf06e0af1ea10d0140fb4c7701205d7910b576503144114cd9a86`.
- 4096-agent diagnostic:
  `/private/tmp/ejm-cli-capacity-comparison-20261021-trace/20261021T061731Z-49a5e1f2/report.json`,
  SHA-256 `c4d1882dd01eb05e0630e23f8fa4d367c47836621f498c8fd4a348ac43be21fa`.

All twelve participant report hashes and sizes were verified against the
parent receipts. The allowlisted public record retains raw trace samples,
work counts, bounded integrity counters, action summaries and report/image
hashes, while omitting source manifests, launch environments and private
identity/endpoint data:
`docs/evidence/capacity-selected-path-trace.macos-arm64.json`.

The eight traced participants recorded 108 samples for each clock, with no
missing commit stamps, omissions, uncovered saved markers, lag, closures or
observer errors. Each changed selection twice and subscribed three times.
All eight trace verdicts are `diagnostic_only`, with selected-event latency
acceptance `unverified`. The six cases retained eighteen process-discovery
errors. Recorded client/helper identities ended and their witnesses became
available, but these observations do not prove arbitrary detached-descendant
containment or success for every process sample.

Observed traced throughput changes were -12.8317% and +0.4027%; repeat spread
was 2.3041% without tracing and 11.8168% with tracing. Censored windows, noise
and sparse action samples leave overhead and latency budgets unverified.
Stop was disabled. The larger cases are single diagnostics. Their first-paint
clock includes fresh creation/start of every session, ready-list processing
and terminal observation; it is not the warm-composer p95 boundary. No
acceptance checkbox changed. Public benchmark guidance records these limits.

The owner approved the separate package operation-lock amendment and repair
after these measurements. It will not alter the frozen images or their
historical evidence. Automatic defaults and publication remain gated.

## Package mutation-lock repair on 2026-10-21 UTC

The approved harness repair now uses operation-scoped `scopeguard` cleanup
to attempt explicit package mutation unlock before closing its file. It keeps
the existing acquisition/contention behavior, Windows pinned anchors and
native lifetime witness rules. The direct edge uses already locked
`scopeguard` 1.2.0 with default features disabled; no new package version is
resolved. CLI changes only its lockfile dependency edge for this repair.

The retained-description regression and exact
`startup::startup_failures_reap_all_loaded_package_hosts` test passed:
`/private/tmp/ejm-capacity-package-lock-checks-20261021/20261021T062910Z-6f75d1b6-package-lock-focused/report.json`,
SHA-256 `22612d289f187f707a422ab94551e3f0d88e3d8613a8d5899569ea5f3423cb9a`.
The retained-handle regression protects the concrete operation-lifetime gap.
Its result does not prove the original gate failure's cause. Unlock remains
best effort, and abrupt death can bypass destructor cleanup.

The current full CLI gate passed strict Clippy, 183 unit cases with five
ignored fixtures, all 22 integration cases, the release build, rustdoc,
dependency checks, native relay fixtures and nine browser cases:
`/private/tmp/ejm-capacity-consumer-required-gates-20261020/20261021T063354Z-95d9549c-cli-gate/report.json`,
SHA-256 `740521e487f956dfd3f48d84a41619a6b2bb6f6999be9116d707520969bdc37f`.
This supersedes the earlier failed CLI gate as the current functional result.
The CLI, harness and shared-UI source records stayed fully stable, with no
exclusions or unreaped direct children.

The full harness gate, including current worker and both SDK suites, passed:
`/private/tmp/ejm-capacity-package-crash-checks-20261021/20261021T063039Z-2d22bffe-harness-gate/report.json`,
SHA-256 `dd35fad60a5d9fc06b9f6f197d97c11e2029d1ad88b03e55117b0aa99b0a5814`.
Current IDE compile compatibility passed separately:
`/private/tmp/ejm-capacity-package-lock-ide-check-20261021/20261021T063747Z-39910459-ide-compatibility/report.json`,
SHA-256 `6db355b0ae76c6747354df0e59f3b85884e9aea6aa2d054ccc7fbc6b5b7b42cd`.
The IDE has no package-manager dependency and needs no source or lock change.
All recorded source roots stayed fully stable and direct children were reaped.

The CLI site rebuilt the current benchmark/developer docs and passed `check`
and unchanged `budget`:
`/private/tmp/ejm-capacity-consumer-required-gates-20261020/20261021T064057Z-09d67e98-cli-site/report.json`,
SHA-256 `1525424e59d339269c87205d000078188e332d8fafff4db911f25a9db2b42a2e`.
The harness site passed its matching checks:
`/private/tmp/ejm-capacity-consumer-required-gates-20261020/20261021T063954Z-684e36e8-harness-site/report.json`,
SHA-256 `2dc159ed72720c2a127489fe229ea9875f25e3ae017f94dd9b9751563a6b2480`.
Both source records stayed fully stable and direct children were reaped.

These are macOS arm64 functional/build checks. They close harness task 2.7,
leaving harness progress at 41/46 and CLI progress at 32/39. They do not change
the frozen diagnostic images, establish performance acceptance, prove native
Linux/Windows recovery or activate automatic defaults. No release or site
publication occurred. The paired changes remain open for their full acceptance.


## macOS recovery and helper source bridge on 2026-10-21 UTC

CLI task 1.2 is complete for macOS arm64. The
[paired harness record](../../../../ejm-harness/openspec/changes/adaptive-agent-capacity/verification.md#macos-recovery-and-helper-source-bridge-on-2026-10-21-utc)
records the source bridge, exact historical images, calibration, recovery and
package-language coverage. Earlier open statements retain their checkpoint scope.

Machine-checked receipt:
`/private/tmp/ejm-capacity-critical-recovery-bridge-20261021.json`,
SHA-256 `0b9cd9030b581c8f46466884cf02ba7f0154551a62c73c5075290399955467b8`.
The retained standard-library adapter is the adjacent `.py` file. Its 133
comparisons match the selected helper, core registry, vendor, package-runtime,
worker and CLI paths across their relevant checkpoints. Both CLI entry and
extension composition are unchanged. In-memory removal of only the package
manager's `scopeguard` edge reproduces the exact earlier lockfile hash. Native
lifetime witnesses keep their close-only protocol.

The latest passed harness and CLI gates cover package startup/import/reload,
controlled interpreter cleanup, cancellation and caller/owner death. The
existing native fixture covers paused-owner exclusion, inherited witness
release, child noninheritance and fresh-grant progress. The receipt preserves
their exact identities and original sampling and cleanup limits. These checks
remain harness-owned; the CLI consumes public contracts and its early helper
entry adds no second scheduler or recovery engine.

Detached descendants can survive owner death. Recovery remains monitored;
paused or uninspectable owners retain commitments, possible survivors remain
unverified external load after confirmed scheduling death, and uncertain
launches are never replayed. No full-cleanup or workspace-safety claim follows.
Linux task 3.2 and native recovery gate Linux activation; Windows shared
ownership and native recovery gate Windows activation. The temporary filesystem
patch still blocks publication pending maintained upstream.

Representative performance acceptance and automatic defaults remain pending.
This bridge ran no new build, test, fixture or benchmark and changes no public
runtime contract. Existing public native limits and gated-default guidance
remain accurate, so no public guide or site build changed for this closeout.
No release, publication or other-platform acceptance is claimed.

### Scoped integration correction

The first integration checkpoint passed release builds and all four Stop tests, then failed before workload startup because the scoped branch omitted the terminal observer. Both observer processes were reaped; no client or helper identities were discovered. The relay contract set passed 18 of 19 tests and caught an unintended 64 KiB prompt limit. Restore main's 16 KiB boundary and declare the observer's existing maintained emulator directly. Retest before claiming acceptance.

## Final macOS representative acceptance and default activation, 2026-10-22

The repaired candidate passed the predeclared representative gate. Exact report:
`/private/tmp/ejm-capacity-critical-comparison-20261021-integration/20261022T104153Z-a69f5df4/report.json`.
See `docs/capacity-integration-checkpoint.json` for its hash, host, inputs, paired
results, latency samples and source bridge. Median automatic throughput loss
was -1.1594%; median terminal overhead was 1.8826%. All commands and observers
were reaped. Pressure recovery, learned restart and reservation bounds passed.
The second terminal-overhead pair was 8.29%; the declared median gate passed.
Earlier failed trials remain evidence. Host fixed rates changed between runs,
so these results do not isolate the effect of the controller repair.

The CLI parser and TypeScript/Python SDK tests passed for macOS automatic
omission and explicit fixed mode against the actual release worker. SDK build,
ESLint and Python Ruff checks passed. Version 0.2.0 and docs changes do not alter
the measured explicit-auto path. No second workload matrix was run.

The public filesystem fork and consumer lockfiles satisfy source distribution.
Registry publication remains separate. Linux activation, other platform native
acceptance and broader hardware/idle claims remain open. Required protected CI,
human review, main merge and source-release dispatch are not complete.
