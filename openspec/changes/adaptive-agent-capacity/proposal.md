# Proposal

## Why

The CLI exposes fixed agent, model, and tool limits and leaves users to tune them. At larger agent counts, global event filtering, complete-history recovery, and complete session lists also add foreground work that automatic harness tuning cannot remove.

## What Changes

- After the native crash-recovery gate passes, select the harness's automatic capacity policy by default. Keep explicit ceilings, a reproducible fixed mode, and the existing execution grants.
- Join the harness's shared local host budget across participating CLI and worker processes, including processes using different session-state directories.
- Prepare shared capacity before package loading and reuse that public context for language hosts and final harness construction.
- Dispatch the public harness native supervisor before ordinary CLI setup. Shared lifetime links keep controlled commands and resident interpreters tied to their owner; waiting agents create no helper or child process per agent.
- Use OS ownership release for shared-owner takeover, preserve uncertain-effect handling, and report measured native cleanup limits. The CLI adds no per-job lifecycle log or replay of uncertain launches.
- Show managed agents, active and queued model/tool work, operating targets, hard ceilings, and the reason work waits through public harness capacity status.
- Consume session-filtered events, bounded presentation tails, and paged session search. Return to input and painting between recovery batches.
- Bound authority view caches without losing run identity, remote mutation ordering, share revocation, or drafts.
- Gate activation on one representative 1000-agent, two-process offline workload that measures useful work, responsiveness and shared resource bounds. Reuse existing safety and boundary fixtures; broader scale, hardware and provider matrices are follow-up coverage.
- Update help, canonical references, public guides, and evidence with implemented behavior and measured limits in the same runtime change.

## Capabilities

### New Capabilities

- `automatic-capacity`: Automatic harness configuration, explicit ceilings and fixed mode, truthful capacity status, and local-profile reset.
- `scalable-session-view`: Bounded foreground event work, incremental recovery, paged session navigation, and safe authority view eviction.

### Modified Capabilities

- `interactive-commands`: Extend `/status` to explain automatic or fixed mode, effective targets, ceilings, queue reasons, and observation availability while preserving existing command behavior.

## Impact

- Owner: `ejm-cli` owns argument resolution, terminal presentation, bounded view state, and host adapters. Admission, probes, provider quotas, learning, persistence, and durable session state remain harness-owned.
- Public dependency: the sibling `ejm-harness` change with the same name must supply a prepared capacity context, governed package setup, public native helper entry, lifetime-linked process composition, and capability/recovery status, capacity policy/status, filtered subscriptions, paged session search, and bounded presentation snapshots. Keep `ejm-cli -> ejm-harness`; do not copy shared logic or import private source.
- Affected areas: `args`, `main`, `extensions`, `ui`, `app`, `render`, `remote/backend`, CLI fixtures, docs, and authored site guides. Existing model-picker, platform, auth, and relay work must be preserved.
- Shared docs owner: `ejm-ui` defers browser tab controls inside the existing docs components while preserving complete server-rendered content and public APIs. The CLI and harness sites consume the change through their declared packages and retain their existing transfer budgets; check both rebuilt consumers and shared tab/hydration behavior.
- Automatic defaults remain gated on relevant native recovery evidence and the representative workload for the platform being activated. Reuse evidence when the exercised runtime and dependency paths remain unchanged; rerun only uncovered or affected risks. Platforms activate independently, so pending Linux or Windows work does not block a verified macOS activation. Capacity omission retains legacy process-local behavior until activation. Existing numeric flags remain accepted as ceilings; explicit fixed mode retains the former envelope while joining shared admission. Remote controllers receive no authority to change capacity or grants.
- Native supervision and lifetime links are the approved execution design. Host-wide coordination is confirmed scope. The CLI adds no installed service, per-job execution ledger, supervisor controller, or tuning engine. Status distinguishes participating reservations, supervised children, sampled external load, and unavailable or monitored limits.
- Status: implementation in progress. The owner selected native supervision and lifetime links with measured recovery limits. macOS arbitrary detached descendants remain a monitored limitation. Local fixtures cannot establish provider capacity, low-end hardware acceptance, or another OS's behavior. Publication remains separate.
