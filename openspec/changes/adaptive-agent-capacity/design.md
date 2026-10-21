# Design

## Context

See `proposal.md` for motivation and the three delta specs for the behavior contract. Implementation is in progress. This plan records the approved target; unchecked tasks and unrun platform or performance gates remain incomplete.

The current CLI fixes its default envelope at 100 managed sessions, 8 model requests, 4 tools, and 512 MiB of admitted tool memory. `main::run_agent` copies those values into `HarnessConfig` and uses `Harness::with_optional_provider`. The shared engine, provider preparation, executor, and execution grants already belong to the public harness crates.

Several foreground paths do work proportional to history or session count:

- `ui::run` subscribes to all harness events before `App::apply` discards other sessions. `recover_events` reads 512-event pages until empty inside one selected event-loop branch. `yield_now` permits another Tokio task to run; it does not return control to terminal input in the same task.
- `remote::backend::load_app` starts from sequence zero and replays every page before returning. Session switching awaits that operation.
- The local authority adapter replays events under its authority mutex to reconstruct run identity and run-transition sequences. Its observed-session map rejects the thousand-and-first distinct session without eviction.
- Opening the picker and finding an ID prefix each call `list_sessions`. The picker renderer prepares every row even though only a small window is visible.
- `main::run` loads `Extensions` before `run_agent` constructs the harness. `RuntimePool::load` can start a lazy Node/Python host and import package code during that earlier phase; callback `PackageContext` receives `harness.governor()` only later. Governing callbacks alone therefore leaves startup and persistent host memory outside the shared budget.
- Transcript blocks and text already have bounds. Change wrapping or introduce a layout cache only if the new workload demonstrates a remaining render bottleneck.

The baseline uses two Tokio worker threads, a small blocking pool, and a 32 ms terminal tick. The saved-session PTY probe recorded 64 to 66 ms picker/search/page response at p95 because data or bare Escape needs a second cycle. Input and bounded work now use a 16 ms cycle while painting keeps its separate 32 ms cadence. Measure the latency and 60-second idle CPU trade-off before claiming those budgets pass. Keep the runtime and blocking pools unchanged.

## Goals / Non-Goals

**Goals:**

- Consume automatic capacity as a harness policy while keeping the CLI's authority to parsing, composition, host adapters, and terminal presentation.
- Preserve the distinction between all managed agents and the subset that currently consumes model, CPU, tool-memory, or terminal resources.
- Bound each foreground step by count, bytes, and elapsed time, with a captured recovery endpoint and ordered state replacement.
- Preserve remote mutation ordering, local Stop, terminal restoration, provider selection, grants, and drafts while allowing old inactive views to leave memory.
- Define reproducible acceptance targets and measure the CLI's contribution separately from host capacity and provider limits.

**Non-Goals:**

- A second scheduler, resource probe, learning store, provider quota tracker, or host registry in the CLI.
- An installed service, distributed host scheduler, per-job lifecycle log, new relay protocol, or remote authority to change host capacity.
- A shared-widget migration, new visual system, speculative renderer rewrite, or tuning of unrelated packages.
- Declaring real hosted-model capacity, the four-core/8 GB target, other OS coverage, or published documentation from a local fixture.

## Decisions

### 1. Configure the public harness policy at process composition

`args` adds `--capacity auto|fixed`. Automatic becomes the default only after the platform recovery and measurement gates pass. Until then, omission keeps legacy process-local fixed behavior and both explicit modes opt into coordination. Existing `--max-sessions`, `--model-concurrency`, `--tool-concurrency`, and `--tool-memory-mib` stay accepted. Parse their explicit presence separately from their resolved values so omitted old defaults do not accidentally cap automatic mode.

Automatic mode forwards optional explicit ceilings to the harness. Fixed mode resolves omitted values to the current 100/8/4/512 MiB envelope, then opts into the same shared host policy with fixed process targets. Fixed mode remains subject to shared host capacity and provider backoff. Neither mode changes write, shell, package, workspace, or remote grants.

`main` resolves `CapacityPolicy` and prepares an additive public harness capacity context before package loading or expensive startup allocation. Final harness composition consumes that same context through an additive constructor that also supports an optional provider and hooks. Existing `HarnessConfig` and `RuntimeConfig` struct literals, constructors, ordinary status structures, and durable wire events keep their compatibility contract. The CLI must not encode absent caps as zero or edit harness private fields. The sibling harness plan owns the exact exported context, constructor signatures, and validation ranges.

Worker requests and SDK options that omit the new optional capacity field keep fixed legacy semantics. Callers can explicitly select the automatic policy. Any future SDK default change shares the same activation gate and must be negotiated without reinterpreting old worker requests. The CLI opts in through the new public contract; it must not infer an automatic policy from a missing field before activation.

The harness owns its common private host registry, probes, quota domains, learning, leases, and failure behavior. Session `--state-dir` remains independent of the common host scope. Coordination covers participating processes in the same supported OS-user/resource domain. Other users, older clients, and unrelated applications are sampled external load. The CLI displays this boundary and any unavailable coordination. It does not access the registry directly or claim it can reserve resources for all applications.

The automatic CLI default has a platform-specific native harness gate. On macOS, kill idle and busy participants and controlled helpers, then measure return to useful work, cancellation, package startup/import/reload, and resident-child cleanup. Record unsupported or monitored descendant cases rather than claiming universal containment. Other platforms need their own evidence before their defaults or support claims change. Do not delete host coordination state or replay uncertain effects to make recovery appear successful.

The owner approved native supervision and lifetime links. The harness owns child admission, helper protocol, parent-loss cleanup, native capability reporting, and shared-owner recovery. The CLI adds a narrow early entry to its current executable that delegates private helper mode to the public library before argument parsing, ordinary Tokio setup, auth, or terminal setup. The helper is lazy and shared by owned native work, with charged startup/residency; waiting agents do not create one process each.

**Alternatives considered:** Keeping numeric defaults in automatic mode preserves the current ceiling. CLI host probes duplicate the harness. Container engines and virtual machines add setup, footprint, and launch costs that do not fit the approved scope. A per-job lifecycle journal adds writes and recovery states to every command. Native lifetime links and OS ownership locks keep the steady path small while native limits remain explicit and measured.

### 1.1. Prepare shared capacity before package runtime work

Use the additive public capacity context as the single owner of the process's coordinated policy, governor, host reservation handle, and lifetime accounting. The CLI prepares this handle after bounded argument/configuration parsing and before `Extensions::load`, language-host launch, package import, or other expensive preparation. Establish top-level cancellable shutdown handling before the first package load and propagate cancellation through each startup phase. The bounded control-plane setup needed to join the host budget is distinct from admission of package or agent work.

Pass the same handle into the additive public `RuntimePool` setup and retain it through `Extensions`. After packages expose their tools/hooks, compose the final `Harness` and `Executor` from that context, with or without a provider. Later callback `PackageContext` values must refer to the same governor. Do not create a placeholder harness to obtain a governor and then replace it, create a second capacity controller for package startup, or access harness private modules. A harness constructor receiving the prepared context must reuse its controller and reservations rather than silently registering another participant.

The harness and `ejm-harness-packages` own the admission and process mechanics. The CLI passes the shared context and cancellation/lifecycle signals. A lazy Node or Python host reserves startup work and its initial resident memory before launch/import and expensive associated allocation. An unused language host is neither started nor charged as resident. Once started, its resident memory has one balanced lifetime charge, represented within the context's shared reservation and retained while the host remains alive between callbacks. Release a startup/import execution slot after that phase; do not hold a running-tool slot merely because the idle language host still occupies memory. A callback admits its incremental work without charging the same resident host memory again.

Import and reload must use this context as well. Account for temporary allocation or an overlapping host generation before it begins. Retain each still-live generation's memory charge until verified exit; a failed import or a cancelled callback does not prove that the resident process ended. The public package runtime contract owns the split between resident memory, incremental memory, and transient runnable work, including measured estimates and conservative fallback. The CLI must not supply a second estimate or admission engine.

The top-level prepared context outlives both the final harness and extensions. On failure, cancellation, or shutdown, stop new work, cancel pending load/import/reload/callback work, quiesce the harness and package runtime in their documented order, and await verified language-host/descendant cleanup before releasing protected memory. Attempt cleanup of all initialized owners even when another cleanup returns an error, and preserve the useful original error. Shutdown of one handle must not retire shared accounting while another initialized owner still holds live resources. Uncertain descendants stay charged through the harness's crash-recovery contract. This also covers package loading succeeding before final harness or provider composition fails.

The worker and SDK startup paths must follow the same public prepare/load/compose ordering. Coordinate this contract with the sibling harness change, including fixed semantics when legacy worker or SDK capacity settings are omitted and explicit opt-in while automatic defaults remain gated. Old constructors/config literals remain valid; coordinated entry points use the additive context API. Read-only CLI commands such as session listing must not start language hosts or native helpers just to construct an empty package pool.

### 1.2. Consume the public native supervision contract

The public executable-entry adapter recognizes only the harness private helper invocation. The harness authenticates bounded private local lifetime/protocol connections. Ordinary commands return to normal CLI startup. The library owns bounded framing, validation, command launch, process groups or supported OS job ownership, and helper shutdown. The CLI must not parse supervisor frames, record child lifecycle state, manufacture cleanup evidence, or copy a native controller. Preserve the existing public constructors, provider behavior, execution grants, package protocol, and relay wire.

Lifetime links tie controlled helpers and resident Node/Python hosts to the composing client. A helper observes owner loss through its retained OS link and cleans up children it owns. Callback cancellation, import/reload failure, final composition failure, and normal shutdown retain resource charges until the shared runtime completes cleanup. Unused language hosts and waiting agents create no extra processes. The core and package runtime choose their native admission estimates and monitored bounds; the CLI supplies no second budget or process estimate.

Consume the harness's staged admission contract for helper startup and reuse. A bounded local queue records demand without granting a lease, preserves its ordering and cancellation, and reports aggregate demand through existing coarse renewals. An unused helper readiness hold must end whenever resource activation would wait. The shared runtime checks the installed grant generation while activating the queued reservation; it may retry only before a launch effect. Package startup includes resident memory and import scratch in one complete peak and transfers those amounts once into their actual leases. Pending helper plans must leave a useful dependent turn schedulable. The CLI owns none of these queue, feasibility or registry mechanisms.

Shared-owner takeover requires release of the prior owner's OS ownership lock. Missed heartbeats, CPU stalls, suspension, or an elapsed TTL alone must not authorize a second owner while that lock remains held. The harness reports stale contact and holds affected new admission when it cannot establish current ownership. Its owner replacement and live-child accounting remain shared public behavior, not CLI file handling.

Native supervision does not prove portable containment of arbitrary detached descendants. On macOS, ordinary controlled-child and interpreter cleanup is measured, while a granted arbitrary command that detaches beyond the supported lifetime links remains a monitored limitation. Doctor and docs distinguish admission estimates, monitored process-tree samples, enforced OS restrictions, and unsupported observations. Do not promise that a missing parent PID, a dead helper, or an empty sampled tree proves every arbitrary descendant ended.

No new per-command durable lifecycle journal, launch intent log, or witness/replay queue is introduced. The existing effect journal still records uncertain effects as interrupted and never automatically repeats them. Coarse shared host grants and learning persistence keep their existing owners. Local input and Stop remain available while supervision or owner recovery reports a degraded state.

Measure helper cold start, idle exit, native launch, resident interpreter idle cost, callback cost, parent/helper death, and owner takeover separately. The required macOS fixtures cover idle/busy client death, controlled command and package-host loss, failed composition, OS-lock release, a paused owner that still holds its lock, and uncertain-effect recovery. They record detached-descendant limits and useful progress. Evidence from macOS must not imply Linux or Windows recovery coverage.

**Alternatives considered:** Passing a governor only in callback context is too late for host spawn and imports. Loading packages first and measuring their memory afterward cannot prevent startup overcommit. Releasing host memory when a callback completes understates real resident use. Giving every package a fresh governor duplicates shared capacity and can deadlock or over-admit. Creating an early incomplete harness couples package discovery to a replaceable controller and risks losing its reservations during final composition.

### 2. Read a separate capacity snapshot and keep local control explicit

Use the additive harness `CapacitySnapshot` beside the existing status interface. It carries mode, scope, observation availability and age, managed count, active/queued work, targets, ceilings, limiting reasons, and cold or learned state. `render` receives snapshots; drawing does not trigger probes or learning. Refresh or subscribe through the harness's bounded status path and reuse the last snapshot while a newer one is pending. Mark stale data instead of blocking a frame on collection.

The compact status line keeps separate agent, model, and tool counts. `/status` provides the full explanation, including a target below an explicit ceiling, provider waiting, host pressure, coordination failure, and missing observations. Use existing plain-text and terminal-safe rendering. Do not add capacity fields to the relay wire or present unsampled resource percentages.

Add `ejm-cli capacity reset` as a local subcommand backed by the harness's reset API. The harness resolves the applicable local profile scope and reports the reset outcome. Reset does not clear sessions, provider credentials, grants, configured caps, or active reservations. Remote session commands do not gain capacity mutation or reset authority. Update the single command catalog and help where relevant; this command need not become a slash command.

Learned capacity profiles and provider rate accounting are separate state. Learning reset does not replenish provider rate budgets. Host reboot can invalidate prior native process ownership but does not erase provider quota consumption or an unexpired provider reset/backoff condition. The harness preserves that accounting until the provider reset rules allow reuse; the CLI only presents the resulting wait reason.

**Alternatives considered:** Extending an existing public status struct can break downstream exhaustive construction and serialization. Computing derived targets from flags in the renderer risks reporting limits that the harness is not enforcing. A local reset implementation would duplicate shared persistence rules.

### 3. Treat input and painting as runnable foreground work

Use one bounded selected-session subscription through the public harness API. Background totals come from aggregate status, not a global event stream that the CLI drains and filters. Change the selected subscription and its selection generation together. The selected conversation, not every managed agent, owns transcript and input state.

Introduce explicit pending view work in the existing terminal event loop: loading, recovering to a captured cursor, or current. Fetch at most one bounded page at a time. Complete the fetch in a cancellable task, then apply only a bounded batch on the UI cycle. Initial limits are 128 events, 64 KiB, or 2 ms of synchronous application per cycle, whichever is reached first; single larger permitted events are handled with bounded text slices or an authoritative replacement rather than bypassing the byte budget. Input and a due frame run between batches. Keep pending queues bounded and return to snapshot recovery when they overflow.

Use separate existing Tokio intervals for 16 ms input/work polling and 32 ms painting, both skipping missed ticks. This prevents a two-cycle lookup from imposing a 64 ms response while preserving the paint cap. Do not add a thread, poller, or renderer for each session. Record idle CPU with the measured response timings because more frequent input polls have a resource cost.

Each request carries the selected session ID, a generation, and its cursor. A completion for an old generation cannot change the current conversation, cursor, overlay, or draft. Session switching cancels pending work and ends the prior share before accepting mutations for the new selection. The new view can show a loading state immediately while preserving the intended session's composer.

Stop takes the direct local cancellation path and acknowledges acceptance without waiting for child cleanup, a backlog drain, a provider response, or capacity learning. Report cleanup separately. The backend must obtain compact authority state and must not restore all history just to validate cancellation.

**Alternatives considered:** `yield_now` inside a full replay loop leaves input starved in that task. Increasing event-channel capacity only delays overflow and consumes memory. One receiver, renderer, or polling task per managed agent makes parked agents expensive.

### 4. Recover to a fixed authoritative cursor without races

The sibling harness must provide a bounded, consistent view snapshot containing session metadata, authoritative active run identity, last run-state transition sequence, durable high-water cursor, selected-session continuity anchor, and a limited transcript tail. The same public contract serves display loading and the local authority adapter. A transcript tail alone cannot safely replace the adapter's full authority replay.

Establish the filtered subscription and snapshot with the harness's documented race-free handshake. Capture the snapshot's high-water mark once. Apply its state atomically for the matching selection generation. Discard buffered duplicates at or below that cursor; replay and live events above it are applied in durable order. Verify selected-stream continuity against the snapshot anchor using the public predecessor/continuity token or an explicit channel loss signal. A token mismatch or reported loss requires another bounded authoritative snapshot before accepting an untrusted continuation.

Global durable sequence numbers interleave across sessions. Selected events with sequences 10 and 14 can be consecutive selected-session events when 11 through 13 belong elsewhere. Do not test continuity with `sequence == previous + 1`, treat a numeric jump as loss, or warn about omitted output on that evidence alone. Keep the cursor for ordering and duplicate suppression; use the separate continuity contract to prove that selected events were not lost.

Recovery pages stop at the captured high-water mark. Continued output does not move that endpoint indefinitely. Once caught up, accept ordered later events within the same budgets. If later traffic already overflowed, start a new bounded recovery rather than retaining every missed event. Durable history remains harness-owned and readable; truncating the presentation is not deleting history.

Keep `SessionBackend` a narrow CLI adapter. The local implementation consumes public bounded snapshots and continuity metadata. The remote implementation retains all existing wire forms, its bounded snapshot/replay protocol, and documented explicit gap behavior, with incremental scheduling and generation checks in its controller loop. Do not add continuity fields to the relay wire or reinterpret numeric jumps as remote gaps. Its existing omission semantics must stay explicit, and incomplete remote presentation cannot establish a new local mutation authority.

**Alternatives considered:** Subscribe after loading can lose events in the interval. Accepting live events before their required selected-stream predecessor can hide actual loss. Inferring loss from global numeric jumps causes needless recovery under ordinary interleaving. Replaying until the store is empty may never finish under sustained output. Storing every missing event in a UI queue trades latency for unbounded memory.

### 5. Page navigation and bound authority caches by safe use

Use the additive harness session-page/search API with a stable ordering and opaque continuation. The picker keeps one 50-entry page and at most 32 prior-page cursors. Query and navigation generations reject stale completions. Render only the visible window. Prefix lookup asks the harness for up to two matches across the full result set, enough to distinguish absent, unique, and ambiguous IDs without listing all sessions.

The noninteractive `sessions` command can consume pages while preserving its existing complete output and JSON-array format. Stream the array incrementally if memory measurements show that materializing the full result remains relevant. Keep errors and broken-pipe behavior consistent; an interactive optimization does not justify a changed output schema.

Replace the permanent thousand-session cutoff in the local authority adapter with bounded eviction of inactive, unshared, unselected entries. Retain at most 1000 entries. Active runs, the selected or shared session, in-flight mutations, and queued remote prompt chains are pinned. If all entries are protected, reject a new view with a recoverable explanation instead of dropping authority state. Existing Stop remains available.

Rebuild an evicted entry from the harness's compact authority snapshot before a mutation. Recheck run identity and sequence under the existing local mutation ordering point; a page read alone does not authorize a later action. Share epoch, revocation state, mutation ledger, and queued chain state stay protected until the share or operation ends. Do not infer revocation from a missing cache entry. These rules apply while removing whole-history waits from `refresh` and `settle`.

**Alternatives considered:** Raising 1000 to a larger permanent cutoff only postpones rejection. Generic eviction without pins can reopen stale actions or detach queued prompts from the run that authorized them. Whole-list prefix filtering is simple at small scale but creates avoidable I/O and allocation at the requested scale.

### 6. Keep the dependency and operating-system lessons narrow

Reuse Clap, Tokio, existing collections, the existing terminal renderer, and public harness types. Capacity work adds no CLI runtime package family. The scoped TLS repair below makes existing transport dependencies direct and pins the patched bundled roots. The harness owns package evaluation for host sampling and reuses its existing SQLite capability for host coordination. If an implementation needs a new package, compare existing dependencies and maintained alternatives against the stated budgets before adding it.

The useful operating-system ideas are runnable versus waiting state, a small hot working set, admission before expensive work, bounded scheduling quanta, and fairness that lets foreground control make progress. Host and provider controls remain distinct because network waiting is not CPU execution. The harness owns admission and tuning; the CLI applies these lessons to its view loop and working set. Do not attempt to replace the OS scheduler or derive a universal concurrency count from CPU count.

**Alternatives considered:** A custom priority thread pool duplicates Tokio and the OS while adding more queues. A new caching framework or UI scheduler is unnecessary unless the measured workload demonstrates that the current primitives cannot meet the contract.

### 7. Close the critical path with one representative workload

The owner narrowed remaining verification on 2026-10-21. The activation sequence is native safety, one representative useful-work check, then the platform's CLI/SDK automatic default. Preserve host-budget accounting, cleanup ownership, cancellation, fixed compatibility and truthful capabilities. Reuse existing focused evidence whenever the exercised runtime and dependency paths remain unchanged. Record the source comparison and receipt rather than rebuilding or rerunning for unrelated linked-code changes. A failed, noisy or incomplete diagnostic does not become a pass through this rule.

Use the existing harness-owned benchmark and CLI PTY/selected-path instrumentation. Run 1000 total managed agents across two participating processes with separate state directories, a bounded selected tail, fixed executing work and background events. Keep useful-work definitions, workload, host constraints and enabled packages the same across capacity modes. Include one bounded pressure step and one learned restart; avoid a separate matrix for each. Record startup separately from steady work and retain monotonic input, Stop and selected-event-to-paint samples, process-tree resource observations and cleanup proof.

The blocking checks are:

| Decision            | Required evidence                                                                                                                                                                                                                                              |
| ------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Safe host use       | Shared reservations stay within effective bounds; controlled hosts remain charged through cleanup; client/helper death resumes useful work safely and a paused owner is not replaced. Reuse relevant package startup/import/reload and resident-host evidence. |
| Useful tuning       | A three-point safe fixed sweep identifies a reference operating point. Automatic capacity loses at most 10% useful throughput against the best safe fixed point under the same constraints.                                                                    |
| Responsive terminal | Input/Stop acknowledgement and selected-event-to-paint p95 are at or below 50 ms. Active interaction loses at most 5% useful throughput against the equivalent workload without active terminal interaction.                                                   |

Use two alternating paired trials for the comparisons. Add at most one pair if noise prevents a verdict. A remaining miss or inconclusive result requires investigation of its concrete cause, not an ever larger benchmark matrix. Sampling must cover the claimed clock and include failures or missing observations; do not infer warm-composer latency from fresh creation of every session. Keep tracing bounded and label its observed cost.

Reuse helper startup and retained-state calibration if a recorded source comparison establishes that native-helper execution and dependency paths are unchanged, even if unrelated linked code changes the executable hash. Keep historical samples attached to their original images. Recalibrate only affected paths, retain conservative accounting, and do not require all eight production helpers under saturation unless changed accounting or concrete evidence identifies a risk. The harness owns this decision and all estimates; the CLI consumes the public contract.

Existing focused 4096-run admission/cancellation, saved-history, navigation and recovery fixtures protect their distinct contracts. The full 100/1000/4096 matrix, package-language/cold-warm matrix, low-resource hardware, hosted providers and precision idle/I/O profiling are follow-up coverage. Their targets remain: warm composer below 300 ms, 100-agent process tree below 256 MiB, incremental parked-agent memory at most 32 KiB and settled idle CPU at most 1% of one logical core over 60 seconds. Retain release-size and dependency receipts where relevant. None of these unmeasured targets is claimed passed or blocks platform activation without a concrete safety or representative performance failure.

Native ownership and recovery remain platform-specific. The real Linux cgroup move and Windows shared-owner mechanism gate their own activation; they do not block a verified macOS default. Reuse relevant evidence rather than requiring the complete scale workload on every hardware/provider combination. The original three-platform support goal remains open until its native dependencies pass. The exact-commit maintained filesystem fork permits source and native builds. Cargo registry publication remains gated on a verified registry version.

Update public benchmark and user guidance when activation changes runtime behavior. This planning-only revision changes no public interface or current default, so it requires no public docs rebuild or Rust suite. Later code changes retain the normal repository and affected-owner gates; do not repeat already passed unrelated checks for planning edits. Keep the runtime worker count and refresh cadence unless a measured failure justifies a change.

### 8. Repair the relay certificate bundle without changing trust policy

The required CLI gate found `RUSTSEC-2026-0381` through `tokio-tungstenite 0.30.0`, `webpki-roots 0.26.11` and the distrusted bundle `1.0.9`. The published 0.26 bridge only re-exports 1.x roots, but the advisory also flags that wrapper. A lock-only bundle update therefore does not clear the required gate. The checked upstream release metadata contains no patched bridge or newer WebSocket release.

The CLI remote host transport owns this repair. Keep the pinned WebSocket package and use its public `Connector::Rustls` and `connect_async_tls_with_config` APIs. Promote the existing `tokio-rustls 0.26.6` dependency to the normal graph and use its public Rustls re-export with the existing crypto provider and protocol configuration. Pin `webpki-roots 1.0.10` directly and construct the same bundled-root store for each connection. Do not merge host roots, bypass certificate or hostname checks, add client authentication, or change the relay wire, ticket acquisition, connection timeout, TCP behavior, bounds or secret-free errors.

Select the pinned crate's declared `__rustls-tls` feature without its vulnerable bundled-root feature. This feature gates the public connector but is not advertised among the README's end-user choices. Record that narrow version-specific compatibility seam in the manifest and dependency review. Check it on upgrades and remove it when a maintained advertised upstream feature accepts the patched bundled roots without the old bridge. The advertised native-roots alternative enables an unused OS-store fallback; changing issuer trust or adding store access is outside this repair. No new package family or resource improvement is claimed.

Add local WebSocket TLS fixtures using the existing maintained Rustls and WebSocket packages. The production connector must reject an untrusted certificate before HTTP upgrade headers, including its bearer ticket, reach the peer. A test-only trusted positive control must complete the TLS and WebSocket handshakes and exchange a bounded frame, so an unusable fixture cannot count as rejection. Preserve existing local remote round trips. Inspect the locked all-feature graph to prove `0.26.11` and `1.0.9` are absent, run the advisory/license/source gate, and run the required CLI gate. Update the canonical dependency review and affected public TLS guidance. Native evidence remains limited to the tested host.

### 9. Load shared docs tab controls only when needed

The CLI docs wire-transfer gate still exceeds its unchanged 95 KiB budget on pages without code tabs. Metadata trims do not remove enough bytes. `ejm-ui` owns the shared docs loading change; the CLI continues to consume its public `DocsPage` and `DocCodeBlock` interfaces through the existing declared packages. Keep dependency direction `ejm-cli site -> ejm-ui`, preserve public props and exports, and reuse the existing tab frame, reducer, preferences and renderer.

Replace the static tab-frame import inside `DocCodeBlock` with an eager awaited server import and a lazy browser import. Use a local Suspense boundary around the existing tab branch in both builds. Server module initialization must finish before the current `renderToString` call so every tab panel remains in the generated HTML. Read an optional SSR build flag safely and support server module environments where that flag is absent. Do not add a renderer, Vite dependency, public export or metadata workaround.

Verify cold server rendering includes every panel without a fallback/error template, delayed browser loading preserves content and hydrates without warnings, and keyboard navigation, copy and shared tab selection retain their behavior. A page without tabs must neither statically reach nor request the tab chunk. Update the shared docs guidance, rebuild the affected CLI and harness sites, and measure actual wire bytes against the unchanged budget. Byte estimates alone are not acceptance, and local checks do not establish publication.

## Risks / Trade-offs

- **A new automatic default changes effective concurrency.** Keep explicit ceilings and fixed mode, document their difference, and reuse relevant provider configuration evidence and check changed default resolution.
- **Host coordination cannot reserve resources for unrelated apps or older clients.** Show coordinated scope and sampled external pressure separately; describe unavailable observations and degraded admission honestly.
- **A smaller UI working set omits old display content.** Keep bounded-history notices and durable harness history available; never use an omitted transcript as authority state.
- **Protected authority entries can fill their bound.** Preserve live/share/in-flight entries, reserve a small control allowance, and return a recoverable capacity error for new views. Benchmark this path so Stop does not depend on eviction.
- **Asynchronous loading complicates ordering.** Use session and query generations, a captured high-water cursor, duplicate suppression, explicit continuity tokens/loss signals, and run-bound mutation revalidation.
- **Stalled shared ownership can hold admission.** Surface the harness reason and recovery path. A missed heartbeat does not release the prior OS owner lock, and the CLI cannot take over or override a reservation to make status look healthy.
- **Native supervision has platform limits.** Measure controlled-child cleanup and recovery on macOS, report arbitrary detached descendants as monitored, and leave other-platform claims unverified until their fixtures run.
- **Helper and resident-host cost can defeat resource targets.** Reuse relevant helper and resident-host evidence; investigate a concrete miss in the critical resource or throughput checks before activation. Keep broader profiling targets unclaimed until measured.
- **Package startup can escape callback-only admission.** Prepare one public shared context before loading extensions, keep resident memory charged between callbacks, and verify failure/reload cleanup and final composition use the same governor.
- **Extra measurements can perturb the workload.** Keep instrumentation bounded and optional in normal operation, record its cost, and compare active-terminal and host-only fixture runs on the same system.
- **Existing concurrent model-picker, platform, auth, and relay work may alter these seams.** Coordinate affected files during apply, preserve their contracts, and run focused cross-contract checks.

## Migration Plan

1. Implement and verify the sibling harness's additive prepared capacity context, public native helper entry, lifetime-linked execution and capability/recovery contracts, package-runtime setup, policy, separate capacity snapshot, bounded authoritative view with continuity anchor, filtered events, and page/search APIs first. Keep old constructors/config literals, omitted-capacity worker/SDK requests, and status/wire forms compatible for other consumers, including the IDE. Explicit automatic selection uses the same prepare/load/compose contract.
2. Pass the relevant macOS idle/busy client/helper-death, controlled-child cleanup, OS-owner-release, package startup/import/reload, and measured-overhead gates before activating its automatic default. Report native detached-descendant limits. In the CLI apply phase, add the early public helper dispatch, prepare capacity before extension loading, retain it through final harness/executor composition and cleanup, and consume native capability status. Then finish argument resolution/local reset, selected loading/recovery, picker paging, and authority refresh through public interfaces. Preserve other platforms' current behavior and avoid private-source adapters.
3. Update `README.md`, `docs/ejm-cli.md`, `docs/benchmarks.md`, `CHANGELOG.md`, `--help`, and affected authored `site/content/` guides in the same implementation change. Cover gated automatic/fixed behavior, native supervision and measured limits, ceilings, host scope, learning/reset, package-host startup and resident memory, import/reload waits and cleanup, queue reasons, observation gaps, owner recovery, grants, and measured limits. Review docs navigation and links, then run the normal site gate and local page review.

4. Run focused offline contract and PTY checks, the representative workload, and `./scripts/check` for code changes with Cargo jobs at most two. Record unrun OS/hardware/hosted-provider checks and distinguish local docs validation from publication.
5. Sync and archive only verified completed runtime work through the matching OpenSpec skills. Publishing remains a separate explicit request.

For rollback during development or release recovery, select fixed mode to restore the former process envelope while retaining host coordination. A full code rollback preserves durable session and relay schemas; a newer learned profile must be ignored safely by older harness versions. Do not delete session state, credentials, or shared host reservations to roll back presentation changes.

## Scoped release integration

The static-site release branch installs the optional PTY observer through a private scripts/package.json with exactly pinned @xterm/headless 6.0.0. This reuses the maintained emulator without pulling in the separate site redesign or changing runtime dependencies. The relay retains protected main's 16 KiB prompt limit, matching the harness admission boundary. Verify the retained relay contracts and exact integration build before workload acceptance.
