# Tasks

Checked tasks have local evidence in `verification.md`. The owner narrowed the
remaining verification to the critical path below. No deferred measurement is
marked passed. Explicit capacity modes are implemented; automatic defaults remain
gated per platform.

## Critical path

1. Reuse relevant safety evidence and close only uncovered native recovery or
   shared-budget risks for the platform being activated (task 1.2).
2. Use one representative 1000-agent, two-process workload for the remaining
   measurement tasks 7.1 through 7.5. Share its inputs, samples and cleanup proof.
3. Activate the verified platform's automatic default through task 2.2, update
   affected docs and run the required checks for the actual code changes.

Full scale, hardware, provider and package-language matrices, precision idle/I/O
profiling and all-eight production-helper saturation are follow-up work unless a
changed path or concrete failure makes one necessary. Existing focused 4096-run
admission/cancellation coverage remains relevant. Native Linux/Windows checks
gate their own activation, not another platform's. See design section 7 for the
bounded comparison and stopping rule.

## 1. Consume the shared capacity contracts

- [x] 1.1 Coordinate with the sibling harness change and confirm additive prepared capacity context, public native helper entry/lifetime-linked execution and capability/recovery status, and governed `RuntimePool` setup before package load/heavy allocation, final composition with optional provider/hooks using that same governor/grant, separate capacity status, local reset, filtered events with continuity metadata, paged session search, and bounded authoritative snapshots; verify CLI/worker/SDK and local-authority contracts without private sibling imports.
- [x] 1.2 Close the platform's uncovered native recovery risks before automatic-default activation. Reuse existing relevant idle/busy client/helper death, package startup/import/reload, resident-child cleanup, cancellation, OS ownership release and paused-owner evidence when the exercised paths remain unchanged. Verify safe useful progress, retained charges for unproven survivors and no heartbeat-based takeover or uncertain launch retry. Record detached-descendant limits and make no other-platform claim without its native evidence; only changed or uncovered paths need a new fixture run.
- [x] 1.3 Consume the verified harness revision through the existing public path packages and refresh `Cargo.lock` if required; verify `cargo check --locked --all-targets -p ejm-cli` with Cargo jobs at most two and preserve other consumers' old constructor/status/wire contracts.
- [x] 1.4 Add the narrow early current-executable dispatch to the public native helper entry before Clap, ordinary Tokio, auth, or terminal setup. Verify ordinary commands keep their behavior, private mode uses bounded authenticated private local lifetime/protocol connections, helper startup/residency is charged, waiting agents create no process each, and cleanup/controller logic remains in the harness. Consume staged public admission for startup and reuse; verify cancellable waiting preserves queue order without a lease or unused helper hold, pending helpers preserve a useful dependent turn, generation-fenced activation precedes effects, and the complete package startup peak transfers once without per-job shared writes.
- [x] 1.5 Present public native capabilities and recovery state in doctor/status and matching help/docs; verify enforced, monitored, unsupported, stale-contact, and owner-recovery states stay distinct. Verify the CLI adds no container/setup requirement, per-job lifecycle log, private witness parser, launch replay, or heartbeat-based owner takeover.

## 2. Add automatic defaults, explicit ceilings, and local reset

- [x] 2.1 Add `--capacity auto|fixed` in `args`, preserve explicit presence of numeric ceiling options, and resolve fixed omissions to 100 sessions/8 model requests/4 tools/512 MiB; verify focused parsing tests cover omitted, partial, explicit, invalid, and fixed settings without encoding absent caps as zero.
- [ ] 2.2 Prepare the public capacity context in `main` before `Extensions::load` or heavy startup work, pass it through extension/runtime setup and final harness/executor composition, and activate automatic defaults only after task 1.2 passes; verify offline startup and final tool work compete for the same budget while optional-provider startup, hooks, selection, explicit ceilings, and grants remain intact.
- [x] 2.3 Consume additive public package-runtime admission and native lifetime links for lazy Node/Python launch, imports, resident memory, and reload overlap through the prepared context; verify unused hosts stay absent, launch waits for admission, one resident charge survives between callbacks, transient execution slots release, and incremental resources avoid double-counting memory.
- [x] 2.4 Install startup cancellation before package load and retain the prepared context through failed load/import/reload/final composition and normal shutdown; verify cleanup attempts every initialized owner, waits for controlled-child exit before releasing protected resources, reports native uncertainty without replaying work, preserves useful errors, and does not retire shared ownership early.
- [x] 2.5 Add the local `capacity reset` subcommand through the harness reset API; verify an isolated profile reset preserves active reservations, provider rate accounting, sessions, credentials, explicit ceilings, and grants, and verify the remote command catalog gains no reset or policy-mutation authority.
- [x] 2.6 Update `--help`, `README.md`, `docs/ejm-cli.md`, `CHANGELOG.md`, and affected public user/developer guides; verify offline examples, gated defaults, native supervision and measured limits, host/reset scope, package startup/residency, import/reload waits and cleanup, owner recovery, and existing grant caveats against the parser and local fixtures.

## 3. Present current capacity without blocking a frame

- [x] 3.1 Read the public separate capacity snapshot into bounded presentation state and reuse the last value while refresh is pending; verify repeated redraw/status reads do not collect host probes, advance learning, or wait on new resource collection.
- [x] 3.2 Extend the compact terminal counts and `/status` explanation with mode, scope, managed versus active/queued work, targets, ceilings, limiting reasons, observation age/availability, and cold/learned state; verify focused render/command cases cover a target below a ceiling, missing observations, provider waiting, and coordination loss without invented values.
- [x] 3.3 Keep old command validation, failed drafts, free-form tails, terminal-safe output, and command discovery intact; verify the existing command tests and focused status cases pass without asserting incidental wording.
- [x] 3.4 Update canonical status documentation and the affected public user/developer guide sections with queue reasons, shared reservations versus sampled external load, and observation limits; verify examples and links against offline fixture output and the site source checks.

## 4. Bound selected-session loading and recovery

- [x] 4.1 Replace the global TUI event subscription with the public selected-session subscription and aggregate status; verify an unselected-session flood does not deliver all background events through the selected presentation path.
- [x] 4.2 Introduce generation-bound loading/recovery state and cancellable bounded page fetches, returning to input and due painting between count/byte/time-limited application batches; verify draft editing and Stop make progress during continuous backlog and a large selected event cannot bypass the byte bound.
- [x] 4.3 Load the bounded consistent session/authority snapshot with its continuity anchor and implement the documented handshake, captured high-water endpoint, ordered continuation, duplicate suppression, and recovery from a predecessor/token discontinuity or explicit channel loss; verify snapshot races and sustained appends lose no accepted event, and globally interleaved sequences cause no false recovery or omitted-output warning.
- [x] 4.4 Move startup and session switching off complete-history replay, reject stale generation completions, and preserve composer/session/share transitions; verify a long saved history and a switch during pending recovery retain the intended draft and cannot repaint the prior session.
- [x] 4.5 Adapt the existing narrow `SessionBackend` and remote controller loading loop to incremental bounded work while preserving all current relay wire forms and documented snapshot/replay/gap semantics; verify existing remote view and mutation fixtures pass without numeric-jump gap inference, new wire fields, or an authority decision from incomplete display state.
- [x] 4.6 Update `docs/ejm-cli.md`, affected public guide history/recovery sections, and `docs/remote-control.md` where controller behavior changes; verify docs distinguish bounded display from durable history and describe loading, lag, and recovery without promising complete remote output.

## 5. Page session navigation

- [x] 5.1 Consume public stable session-page/search APIs in the picker with bounded retained pages and query/navigation generations; verify thousands of saved sessions, page boundaries, and late query completions preserve selection and draft state without complete-list materialization.
- [x] 5.2 Resolve an ID or prefix through bounded full-set search and render only visible picker rows; verify absent, unique, and ambiguous prefixes including matches outside the visible page, plus keyboard selection across page boundaries.
- [x] 5.3 Preserve the noninteractive `sessions` command's complete text and JSON-array contracts while consuming pages; verify existing output parsing and errors, and stream array entries only if the measured list-all path requires it.
- [x] 5.4 Update session navigation and prefix-recovery examples in `docs/ejm-cli.md` and the public user guide; verify the documented keys and commands with offline saved-session fixtures and check affected anchors.

## 6. Bound local authority views safely

- [x] 6.1 Replace full-history authority refresh with the public compact run identity, transition-sequence, and cursor snapshot, keeping the existing local ordering point and run-bound mutation revalidation; verify local admission, steer, Stop, and remote mutation races retain current authority semantics.
- [x] 6.2 Replace the permanent thousand-session cutoff with bounded inactive eviction and explicit pins for selected/live/shared/in-flight and queued-chain entries; verify visiting more than one thousand inactive sessions succeeds, evicted entries rebuild correctly, and all-protected admission returns a recoverable error within its configured bound.
- [x] 6.3 Preserve share epochs, revocation, mutation-ledger ordering, and queued prompts across refresh and eviction; verify fixtures cover a revocation or local action racing a queued remote mutation and confirm Stop does not await backlog reconstruction or cache eviction.
- [x] 6.4 Update canonical local/remote authority limits and recovery guidance in `docs/ejm-cli.md`, `docs/remote-control.md`, and affected public guide links; verify docs match the protected-entry failure behavior and do not describe cache loss as revoked authority.

## 7. Measure the scale budgets and useful-work cost

- [ ] 7.1 Reuse the existing harness benchmark and CLI PTY/selected-path traces for the representative workload. Verify monotonic input/Stop/event/paint boundaries and bounded samples, retain missing-sample and tracing-cost limits, and collect process-tree resource observations from the same run. Add instrumentation only for a concrete missing verdict; no paid providers, second benchmark engine or precision I/O campaign.
- [ ] 7.2 Run one representative 1000-total-agent workload across two participating processes with separate state directories, a bounded selected tail, fixed executing work and background events. Verify p95 input/Stop and selected event-to-paint at or below 50 ms, useful cancellation and cleanup. Include one bounded pressure step and one learned restart in this fixture. Reuse focused history/navigation/recovery and 4096-run boundary coverage; warm startup and broader scale/package matrices remain unclaimed follow-up targets.
- [ ] 7.3 Verify shared reservations stay within the representative workload's effective budget and resource observations include controlled helpers, resident hosts and known descendants, with unknown observations labelled. Reuse helper startup/retained-state calibration when the exercised native-helper and dependency paths are unchanged, documenting the source bridge even if unrelated linked code changes the binary hash. Recalibrate affected paths only. Preserve conservative charges and report memory/CPU samples without claiming the separate 256 MiB, 32 KiB, 1% idle or low-resource-hardware targets passed.
- [ ] 7.4 Use the representative workload for a three-point safe fixed sweep and compare automatic capacity with its best useful-work result under the same host constraints. Run two alternating paired trials and distinguish active-terminal cost using the equivalent workload without active interaction. Require no more than 10% useful-throughput loss for automatic capacity versus the best safe fixed point and no more than 5% for active-terminal overhead, retaining p50/p95 completion latency and noise. Add at most one pair if noise prevents a verdict; a remaining miss or inconclusive result needs investigation of its concrete cause, not a larger matrix.
- [ ] 7.5 Publish one compact local evidence record for tasks 7.1 through 7.4 with exact inputs, host/OS, commands, raw results, source/calibration bridges, observed resources, cleanup and verdicts. Update affected benchmark/user guidance at activation with measured scope, misses and deferred coverage; retain existing release-byte/dependency receipts where relevant. Check links and examples without reopening unrelated release, hardware, provider or profiling work.

## 8. Verify the integrated change

- [x] 8.1 Run focused parser, view recovery, navigation, authority, and offline binary integration checks, formatting, warnings-denied Clippy, and `./scripts/check` with Cargo jobs at most two; verify the full gate passes or report each concurrent/unrelated failure to its owner without weakening gates or reverting other work.
- [x] 8.2 Run `npm run check` in `site/`, review the affected local docs pages and navigation, and validate `adaptive-agent-capacity` with `openspec validate --strict`; verify all authored docs match implemented behavior and report local validation separately from publication.
- [x] 8.3 Review changed-owner harness/CLI contracts and downstream evidence for unchanged relay wire, legacy constructors/config literals, fixed semantics for omitted worker/SDK capacity, gated automatic defaults and shared prepare/load/compose ordering, public native ownership/lifetime links, and provider charges surviving reboot until provider reset; verify no copied controller, CLI scheduler/probe/persistence copy, per-job execution ledger, uncertain-launch replay, missed-heartbeat stealing, private-source import, shared UI duplication, or unmeasured claim entered the change.

- [x] 8.4 Repair `RUSTSEC-2026-0381` with the existing public Rustls WebSocket connector and direct pinned `webpki-roots 1.0.10`, removing the vulnerable 0.26 bridge while preserving bundled trust, crypto/protocol policy, timeout, framing, ticket and relay contracts. Document the pinned feature seam and removal condition; verify real WebSocket TLS acceptance and rejection before credential-bearing upgrade headers, existing remote round trips, the locked all-feature graph, advisory/license/source checks and the required CLI gate. Update canonical/public dependency guidance and record measured scope without a size or speed claim.

- [x] 8.5 Defer shared docs tab controls in the browser through an internal `ejm-ui` loading change while preserving complete server-rendered tab panels, public APIs, existing tab behavior and the unchanged 95 KiB wire-transfer budget. Verify cold server rendering, delayed hydration, keyboard navigation, copy, shared selection, and no tab-chunk load on a plain docs page; update shared guidance and run the affected owner and rebuilt CLI/harness site checks with measured wire bytes.

## Workflow follow-up

- Sync verified completed requirements with `openspec-sync-specs`, then archive through `openspec-archive-change` after required review and acceptance.
- Keep publication and release actions separate from local verification; publish only when the user requests it and verify the live docs before reporting them updated.
