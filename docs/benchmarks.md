# Performance benchmarks

ejm-cli's acceptance target is 100 managed hosted-model agents on four CPU
cores, 8 GB RAM, and an SSD. Managed agents share model and local command pools.
The CLI should paint a warm composer in under 300 ms at p95, retain bounded
scrollback, and acknowledge Stop promptly while cleanup proceeds separately.

The sibling ejm-harness repository owns the reproducible benchmark runner and
the raw report under `docs/evidence/benchmark.json`:

```sh
cargo build --release --locked
(cd ../ejm-harness && cargo build --release --locked -p ejm-harness-bench)
(cd ../ejm-harness && python3 scripts/benchmark.py --agents 100 --iterations 20 --output docs/evidence/benchmark.json)
```

An earlier local release run on 2026-10-18 measured first composer paint at
24.197 ms with fresh state and 41.312 ms at warm p95 across 20 samples. It used
a Mac16,8 with 12 logical CPUs, 24 GiB RAM, macOS 26.5.2, and Rust 1.99.1.
The [raw report](https://github.com/ejmlabs/ejm-harness/blob/main/docs/evidence/benchmark.json) includes the
CLI binary hash and every sample. No hosted API calls were made.

That historical runner measured process creation to the first composer paint in a real
100x30 PTY, with a test response provider and reused state directory. The shipped
replay flags have since been removed. The current runner opens the ordinary
unconfigured TUI without submitting a model turn. First paint
includes process creation and local initialization. It does not include remote
model latency. Its engine workloads record separately sampled RSS and actual
governor concurrency.

`./scripts/check` also runs a real PTY check for input, offline output, signal
handling, and terminal restoration. The refresh interval and transcript bounds
are implementation facts, not timing evidence. Input-to-stop and delta-to-paint
timings need separate trace instrumentation.

A result from a modern machine does not establish the old-laptop target, and no
controlled speed or coding-quality comparison with another agent has run.

## Budgets for future changes

The [platform support table](cross-platform.md) lists native test coverage.
Composer latency, input, cancellation, process-tree memory and idle workloads
need separate measurements. Windows, Linux and Intel Mac runtime benchmarks
have not run yet.

The [final local macOS arm64 release snapshot](evidence/cross-platform-current-macos-arm64.json)
from 2026-10-19 UTC records a 16,243,744-byte binary with SHA-256
`43e08112234ffe4440fb4da76d2bbd4ab6f88918fbd55634e91adb3f10cc7076`.
Twenty fresh-process plain replay turns with reused isolated state measured
11.459125 ms p50 and 12.546583 ms p95 on the Mac16,8 described above. These
measurements cover process creation through exit. The report keeps every
sample, exact arguments, toolchain and observed checkout revisions. No CLI
Cargo build ran during these samples; other machine work was uncontrolled.

The [source input report](evidence/cross-platform-release-inputs.json) records
hashed build inputs for this local development build. It is not an immutable
release snapshot, and the earlier binary has unknown source provenance and
workload. These results therefore cannot establish a controlled speed or size
change. Input/cancel timing, process-tree memory, startup/idle CPU and I/O,
100-session acceptance and the other native platforms still need measurement.
The [archive report](evidence/cross-platform-archive-macos-arm64.json) covers
packaging and extracted offline checks separately.

Before changing a path, set budgets for its startup and terminal update latency, peak process-tree memory, CPU, binary size, I/O and dependency cost. Compare before and after with the same workload, compiler profile and machine. Measure relevant package alternatives and transitive features against those budgets. Record the results, explain speed and footprint trade-offs, and justify any regression.

## Adaptive capacity and large session collections

The adaptive-capacity change adds explicit automatic/shared fixed modes and bounded selected-view work. Automatic defaults remain gated on native crash-recovery and performance evidence. The partial local results below do not establish general thousand-agent acceptance.

The planned release workload covers 100, 1000, and 4096 managed agents, a fixed selected tail, executing work, background event bursts, large saved histories, session searches/switches, slow readers, and cancellation. Targets are p95 input/Stop and selected-event paint within 50 ms, warm startup within 300 ms, additional parked-agent memory within 32 KiB each, the existing 100-agent process tree within 256 MiB, and settled idle terminal CPU within 1% of one logical core over 60 seconds. Active terminal interaction targets at most a 5% useful-throughput cost against the same host workload without active terminal interaction. Saved-session results below cover a subset of these targets; managed-agent acceptance remains incomplete.

Evidence must include release bytes, exact build inputs, package features, process-tree memory, CPU/I/O, monotonic latency samples, and no-package/first Node or Python host/import/warm callback/reload cases. Compare automatic capacity to a same-host safe fixed sweep and exercise several participating processes with separate session directories. Do not infer native crash recovery or four-core/8 GB acceptance from a development-machine replay fixture.

### Saved-session terminal measurements

The existing Unix PTY check has an optional `--measure-saved` mode. Use a new
retained journal from the public harness benchmark, then pass its
`first_session_id` to the CLI measurement:

```sh
# Run in ejm-harness. Use new, absolute paths without symlink components.
target/release/ejm-harness-bench --agents 1000 --rounds 0 --deltas 128 --park-ms 0 --capacity fixed --capacity-root /private/tmp/ejm-capacity-fixture --state-dir /private/tmp/ejm-saved-fixture

# Run in ejm-cli after building its release binary and installing the observer with `npm ci --prefix scripts`.
python3 scripts/check_terminal.py --measure-saved target/release/ejm-cli --state-dir /private/tmp/ejm-saved-fixture --session SESSION_ID_FROM_REPORT --samples 20 --idle-seconds 60 --output docs/evidence/capacity-terminal-1000.json
```

These examples use macOS's real `/private/tmp` directory. On Linux, choose a
real directory with an existing parent. macOS's `/tmp` and `/var` aliases are
symlinks and are rejected for protected capacity state.

Repeat with 100 and 4096 saved sessions and fresh fixture paths. The observer
uses the installed `@ejm/shots` package's maintained xterm.js emulator to check
the visible terminal grid after differential draws. This development check
needs Node and the site's dependencies; ordinary CLI use does not.

The JSON report keeps bounded monotonic samples for draft paint, picker data,
page changes, search, and session switches, plus a delayed-reader case. It also
records the binary hash and size, process-tree RSS snapshots, and idle CPU over
the requested interval. Its first-paint sample covers process creation to the
composer. The process snapshots do not establish peak memory or include children
that exit between samples.

This is a saved-session workload. It does not measure concurrent live agents,
an unselected-agent event flood, executing work, Stop, or I/O. The delayed-reader
sample includes the requested reader delay. External observer overhead and
uncontrolled host load are reported limits. Use the harness workload and native
lifecycle fixtures for the remaining acceptance gates.

### Saved-session results from 2026-10-19

The retained reports used an interim dirty-source release binary before removal
of its replay flags. Their raw commands and hashes remain unchanged as historical
evidence. The binary was 17,826,288 bytes. Each case used 20 interaction samples
and a separate 60-second idle interval. These are saved sessions with no active
agent workload or package hosts.

| Saved sessions                               | Draft p95 | Picker open/page/search/close, worst p95 | Switch p95 | First paint, one sample | Idle CPU, one core |
| -------------------------------------------- | --------- | ---------------------------------------- | ---------- | ----------------------- | ------------------ |
| [100](evidence/capacity-terminal-100.json)   | 33.37 ms  | 35.38 ms                                 | 33.48 ms   | 16.59 ms                | 0.53%              |
| [1000](evidence/capacity-terminal-1000.json) | 34.28 ms  | 35.42 ms                                 | 33.55 ms   | 49.43 ms                | 0.58%              |
| [4096](evidence/capacity-terminal-4096.json) | 32.43 ms  | 35.38 ms                                 | 33.20 ms   | 176.98 ms               | 0.58%              |

The 100-session [earlier probe](evidence/capacity-terminal-100.baseline.json)
recorded picker p95 values near 65 to 66 ms. It exposed a 32 ms input cycle that
needed a second cycle for fetched data or bare Escape. Input and bounded work
now use a 16 ms cycle with a separate 32 ms paint interval. These observations
support that local scheduling change. They do not establish a controlled
throughput comparison, peak RSS, parked-agent cost, or other-platform acceptance.

### Active-agent terminal fixture

The optional `--measure-live` mode uses the public unpublished
`ejm-harness-bench` workload and real native command executor. Build the release
test executable and pass the path Cargo prints:

```sh
cargo test --release --locked --no-run -p ejm-cli --jobs 2
python3 scripts/check_terminal.py --measure-live PATH_TO_RELEASE_TEST_EXECUTABLE --helper target/release/ejm-cli --fixture-root /private/tmp/ejm-live-100 --agents 100 --seconds 30 --samples 20 --output docs/evidence/capacity-live-100.json
```

Repeat with 1000 and 4096 agents using new fixture roots. Successful runs restart
within the same managed session; stopped runs stay stopped. By default, the
workload uses fixed capacity with ceilings of eight model requests and four
native tool jobs, 128 deltas per response and one fixed read-only command per
turn. `--capacity auto|fixed`, `--model-requests 1..256`, and `--jobs 1..128`
select the policy and ceilings for each case. Both modes use the same remaining
hard ceilings from the public harness resource defaults: 512 MiB memory,
2000 CPU millis, one heavy job, 32 process slots, and 1 MiB output. Queue,
artifact, and job-duration ceilings retain the same harness defaults in both modes.
The fixed development command has a ten-second timeout, including queue time.
The harness may reduce CPU and memory ceilings to fit the host. A configured
ceiling does not promise that host admission will allow that much concurrent work.
The helper path selects the production executable's public harness helper
adapter. Test-only configuration never reaches ordinary CLI startup.

Each case keeps its own new `--fixture-root`, including its session state.
Capacity defaults to that root's `capacity` directory. To compare several
participating processes, give every process the same explicit
`--capacity-root /private/tmp/ejm-live-shared-capacity` and separate fixture roots.
Use a private root with no symlink components and retain it across the cases
whose shared learning you intend to measure. The harness validates and owns
this fixture budget through its public API; it is separate from the ordinary
host registry. Reports retain the requested policy and capacity root, plus
`fixture.resolved_capacity_limits` with the resolved session/model ceilings and
complete resource limits. Check those values match before comparing modes under
the same hard envelope. Older reports without resolved limits do not establish
that equivalence. Selecting these controls does not establish a multi-process or
tuning result.

The observer supplies an explicit 2 GiB journal limit to the internal fixture.
The fixture reports physical main/WAL bytes and retains at most eight failure
samples, each with at most 512 characters of error text. The observer writes
the report before rejecting any active or drain failure, independently of the
libtest process's exit status. The public journal default stays at 512 MiB.

For a throughput pair, pass `--no-stop --no-event-trace` to the interactive case
and `--headless` to the equivalent case without the UI. Compare useful completed
work per elapsed second, completion latency, and the same source/binary inputs.
Use an additional `--no-stop` interactive case with event tracing to measure its
overhead. Run pairs in alternating order while the host and source are quiet.

The bounded event probe matches a response-only marker in the completed terminal
buffer. It retains the existing selected-subscription receipt-to-write samples
and adds `selected_event_commit_to_terminal_write_ns`, with at most 1000 samples.
The added clock starts immediately after a successful harness journal commit and
ends after terminal writing returns. It includes publication, queue and scheduler
delay, but excludes precommit journal work and the terminal emulator's later
presentation. Both clocks end at the same write callback. External PTY traces
measure observed input and Stop acknowledgement separately. The fixture records
Stop intent so a completion racing cancellation does not keep restarting that
session.

Completion counts and latency end when the fixture controller observes a result.
`initial_completion_ns` starts at the workload gate; `completion_ns` starts before
submitting each later turn. Keep those distributions separate. `drain_completed`
counts results observed only after the cutoff, so it bounds possible throughput
undercount and is not included in the reported rate. Both executables' hashes and
sizes are retained.

The internal fixture traces the actual selected-view subscription, receive and
apply path. It creates no second subscription or observer task. A marker enters
the bounded pending set only after the view accepts its session, sequence,
predecessor and byte bounds, immediately before application. Each pending entry
records its session, view generation, event sequence and marker. The end clock
is captured immediately after a successful terminal draw, before scanning the
completed buffer for matching markers.

The harness-owned workload prefixes each response with
`response:{prompt}:round-{round}:`, using at most 128 prompt characters. Fixture
prompts distinguish turns, and the round comes from that turn's tool results.
This prevents a later model round from satisfying an earlier round's paint
match. It does not establish timing for markers loaded from saved history.

`paint_trace_integrity` records successful subscriptions, selection changes,
same-session `generation_resets`, lag and unexpected closure signals, bounded
error samples, `missing_commit_stamps` and `uncovered_saved_markers`. Missing
commit stamps never fall back to receipt time. Accepted saved-tail and recovery
markers have no monotonic origin and increment `uncovered_saved_markers`.
Changing selection or generation retires pending samples as omissions. The
actual UI records lag before replacing its receiver through bounded recovery;
the public lag argument is not an exact lost-event count. Selected-view load
and task failures remain in `result.json`. The pending set stays capped at 32
entries, each sample array at 1000 and error samples at eight 512-character
strings. Zero errors or omissions does not prove complete latency coverage.

The PTY report keeps `workload_status` separate from `event_trace_status`:

| Trace status      | Meaning                                                                                                                                   |
| ----------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| `disabled`        | Event tracing was not requested.                                                                                                          |
| `incomplete`      | Integrity or commit evidence is missing, sample counts differ, an observation failed, saved markers lack timing, or samples were omitted. |
| `no_samples`      | Tracing was enabled without a detected error, but no response paint sample was retained.                                                  |
| `diagnostic_only` | Samples were retained without a detected observation error or omission.                                                                   |

`event_trace_reasons` explains the trace verdict. Successful useful work can keep
`workload_status: passed` with an incomplete trace. All verdicts and raw samples
are saved before a workload failure rejects the measurement. Older traced
reports without integrity accounting are incomplete under this check.
`selected_event_latency_acceptance` remains `unverified` for every trace status.
The live-event hook excludes snapshot duplicates and follows the current view
generation. Saved and recovery events remain untimed. Complete first-write and
selection coverage, a representative latency population and trace overhead
still need release-profile evidence. Commit samples remain diagnostic and do
not establish either latency or overhead acceptance. Earlier receipt-only
reports do not measure commit-to-write delay.
The observer retains its 120-second cap.

The retained `idle_start`, `idle_end`, and `idle_percent_one_core` field names
come from the saved-session probe. In live reports they bracket active workload,
so their CPU values do not establish settled idle terminal cost. Changing child
membership can make CPU accounting unavailable. Memory snapshots cover observed
live descendants and do not establish a peak or the complete cost of exited
children.

### Active-agent results from 2026-10-19

The [historical derived record](evidence/capacity-live-summary.json) links the
first 12 raw reports. A separate
[replacement record](evidence/capacity-live-4096-journal2g-summary.json) links the
two explicit-envelope reports. Both record counters, separate initial/restarted
latency distributions, memory snapshots, source hashes and limits. These local
release fixtures used an Apple
M4 Pro with 12 CPUs, 24 GiB RAM, Darwin 25.5.0 arm64 and Homebrew Rust 1.99.1.
No team build or other team timing workload ran during each pair; other host
load was uncontrolled. There were no remote API calls or code packages.

The first 12 reports used a production helper of 17,892,512 bytes, SHA-256
`e29555413618192a8de447a6fcdd0c0a4dc707d080fa46b662b39504586c902e`.
The release test executable was 19,072,480 bytes, SHA-256
`29ad4f6236d641fc83ab858dac160c322a4c50fd2dcc387fef86055acaec6f43`.
Both came from a dirty checkout through
`cargo test --release --offline --locked --no-run -p ejm-cli`, with two Cargo
jobs and incremental compilation disabled. The selected instrumentation hashes
are not a complete immutable build manifest.

Each 30-second Stop case used 20 interaction samples:

|                                Managed agents | Input p95 | Picker, worst p95 | Switch p95 | Stop acknowledgement p95 |
| --------------------------------------------: | --------: | ----------------: | ---------: | -----------------------: |
|   [100](evidence/capacity-live-100.stop.json) | 33.269 ms |         34.429 ms |  33.630 ms |                33.845 ms |
| [1000](evidence/capacity-live-1000.stop.json) | 33.531 ms |         34.365 ms |  33.858 ms |                33.325 ms |
| [4096](evidence/capacity-live-4096.stop.json) | 33.499 ms |         34.540 ms |  33.536 ms |                33.442 ms |

All three recorded 20 distinct stopped sessions, 20 cancellations and no failed
runs. Response subscription receipt to terminal writing had 21, 3 and 2 samples,
with p95 values of 31.100, 30.511 and 29.589 ms. The larger cases have too few
samples to establish general response-paint p95 acceptance. Fixture preparation
plus first paint took 111.942, 363.464 and 1483.172 ms, one sample each; these
are not ordinary warm-startup measurements.

Untraced 30-second pairs disabled Stop and counted useful completed turns:

| Managed agents |                                    Interactive turns/s |                                    Headless turns/s | Interactive loss |
| -------------: | -----------------------------------------------------: | --------------------------------------------------: | ---------------: |
|            100 |  [73.287](evidence/capacity-live-100.interactive.json) |  [73.807](evidence/capacity-live-100.headless.json) |           0.705% |
|           1000 | [66.668](evidence/capacity-live-1000.interactive.json) | [66.650](evidence/capacity-live-1000.headless.json) |          -0.028% |
|           4096 | [22.694](evidence/capacity-live-4096.interactive.json) | [32.591](evidence/capacity-live-4096.headless.json) |      **30.368%** |

The 4096 case missed the 5% throughput target in this window. It counted only
initial completions: 681 interactive and 978 headless, beginning at 25.819 and
24.748 seconds. There were no restarted-turn samples at that scale. All six
cases recorded no failures or completions observed only after cutoff. One pair
per setting does not establish repeatability or measurement noise.

The additional [1000-agent traced case](evidence/capacity-live-1000.trace.json)
and its untraced counterpart each counted 2000 completions, about 66.67 turns/s.
Their initial and restarted latency distributions changed in opposite
directions. These runs do not establish negligible trace overhead.

A separate 120-second 4096 pair ran headless first. Both counted 8192 successful
turns, about 68.27 turns/s, with 4096 initial and 4096 restarted samples each.
The [interactive case](evidence/capacity-live-4096.long-interactive.json) also
recorded **4096 failures**, while [headless](evidence/capacity-live-4096.long-headless.json)
recorded none. This pair does not pass throughput acceptance.

Retained state strongly supports exhaustion of the inherited 512 MiB journal
limit. The interactive SQLite file reached exactly 536,870,912 bytes, with
131,072 pages of 4096 bytes and no free pages. It retained 12,288 run starts,
8192 completed terminal events, and 4096 still-running authority records without
terminal outcomes. The headless journal was 506,413,056 bytes. The fixture did
not retain individual error summaries, so the literal SQLite error is
unavailable. The failed case stays in the evidence. A test-only follow-up gives
the fixture an explicit 2 GiB journal envelope, records physical journal bytes
and bounded failure summaries, and rejects failed workloads independently of
the libtest exit code. The public journal default is unchanged.

#### Replacement pair with an explicit journal limit

The replacement 120-second pair ran headless first, then interactive, with Stop
and event tracing disabled. Both workload verdicts passed independently of their
zero process exit codes:

| Case                                                                  | Completed turns | Turns/s | Initial/restarted samples | Failed/drain failed | Pending at cutoff/drain cancelled | Journal main/WAL bytes |
| --------------------------------------------------------------------- | --------------: | ------: | ------------------------- | ------------------- | --------------------------------- | ---------------------- |
| [Headless](evidence/capacity-live-4096.journal2g-headless.json)       |            8085 |  67.376 | 4096 / 3989               | 0 / 0               | 4096 / 4096                       | 460,468,224 / 362,592  |
| [Interactive](evidence/capacity-live-4096.journal2g-interactive.json) |            8192 |  68.268 | 4096 / 4096               | 0 / 0               | 4096 / 4096                       | 504,303,616 / 354,352  |

Neither case recorded active cancellations, drain completions, operator stops
or failure samples. Both stayed below the 2,147,483,648-byte journal limit.
Headless initial/restarted completion p95 was 57.824 / 65.697 seconds;
interactive was 51.708 / 59.716 seconds. These long queued-turn latencies are
separate from terminal input latency. Interactive input p95 was 33.497 ms,
picker worst p95 was 33.997 ms, and switch p95 was 33.537 ms, with 20 samples
each. There were no selected-response trace samples in this pair.

Interactive recorded 107 more completed turns, an apparent loss of -1.324%.
This pair did not miss the 5% target, but one pair and different restarted-turn
populations do not establish repeatability or a general overhead bound. It
does not replace the earlier 30-second miss or the failed 512 MiB case.

The replacement helper was 17,892,512 bytes, SHA-256
`2347ddd37cf1efc8e19ea0aff05e2a5982d46291a5baf979653003125976c150`.
Its test executable was 19,072,496 bytes, SHA-256
`1d349e5a9ef06ec250a27ecc7a7f86925351c4457c0c0d9797e348a8d3cd89e0`.
The source changed for the fixture diagnostics and the harness's prepared
composition guard. This is a separate build and series, so it is not a
controlled comparison with the historical reports. The replacement record
keeps the selected instrumentation hashes and raw report hashes.

These results do not establish peak process-tree memory, parked-agent memory,
settled idle CPU, I/O, shared-host coordination, automatic-versus-fixed tuning,
learning, package-host lifecycle cost, native recovery, live-provider behavior,
other platforms, or four-core/8 GB acceptance. The early
[diagnostic probe](evidence/capacity-live-probe.json) is excluded because its
picker-close observer could pass on a clipped hint. No automatic default,
release or public-site publication was activated by these measurements.

### Coordinated CLI exploration on 2026-10-20

A later release checkpoint passed the V5 focused checks and two five-second
shared-root smoke cases, fixed then automatic. Each case used two overlapping
participants with separate session stores, each managing 100 agents. The runner
checked equal resolved ceilings and shared ancestor accounting, useful completed
turns, zero workload/drain failures, exact identity and witness cleanup, and
reaping of every command and observer. This is a coordinated smoke result.

The release helper was 18,093,312 bytes, SHA-256
`bbdce48e8517f9f55541d52f63495a240712e4b2310c5565a15e0e17e3ccb3a4`;
the fixture was 19,455,136 bytes, SHA-256
`e1638365848ef7ed0eb9fad9003aaa93c127d896d8bc7023d824467a3415df79`.
The longer attempt reused those exact images. Each participant's envelope was
100, 1000 or 4096 managed sessions as requested, two model requests, one native
job, 512 MiB memory, 2000 CPU millis, one heavy job, 32 processes, queue 256,
1 MiB output, 64 MiB artifacts and a 120-second job deadline. No package hosts
or paid model calls were used.

The exploratory plan had 24 cases: two repeats at each agent count, fixed and
automatic, with and without terminal interaction. Work windows were 30 seconds.
Interactive cases disabled Stop and event tracing. Each case had two
participants sharing a fresh capacity root. The first four 100-agent cases
passed, with approximate controller-window overlap above 99.9%:

| First-repeat case      | Participant 1 completed turns | Participant 2 completed turns | Sum of participant turns/s |
| ---------------------- | ----------------------------- | ----------------------------- | -------------------------- |
| Fixed, headless        | 900                           | 995                           | 63.170                     |
| Fixed, interactive     | 802                           | 909                           | 56.613                     |
| Automatic, headless    | 738                           | 799                           | 51.234                     |
| Automatic, interactive | 771                           | 800                           | 52.423                     |

Each participant still had 100 pending turns at cutoff, later cancelled, with
no drain completions or failures. These are observed completion rates from one
repeat, with unfinished turns censored. The fixed interactive pair's apparent
loss was 10.38%, above the 5% target in that pair. Its hypothetical cutoff-count
bounds ranged from -0.10% to 18.94%; those bounds are not observed throughput or
a confidence interval. The series establishes neither a throughput win nor a
sustained regression, repeatability or the best safe fixed concurrency.

The fifth case, fixed/headless with 1000 agents per participant, failed the
useful-work check: participant 1 completed zero turns despite 990 completed
tools; participant 2 completed 66 turns and 1000 tools. Both had 1000 pending
turns at cutoff, all cancelled during drain, with zero recorded workload/drain
failures. The runner stopped and retained 19 unrun cases. Tool completions do
not substitute for completed turns; scheduling and the short window still need
investigation.

That attempt also retained a failed cleanup verdict. A separate read-only audit
later proved that the five retained observer/client/helper birth identities had
ended and all three original witness inodes were stable and unlocked. It did
not change the failed report or establish cleanup of arbitrary detached
descendants. Neither a timeout nor a newly created lock path was exit proof.

The exact receipts and SHA-256 values are retained in the
[CLI verification record](https://github.com/ejmlabs/ejm-cli/blob/main/openspec/changes/adaptive-agent-capacity/verification.md#coordinated-smoke-and-exploratory-matrix-on-2026-10-20).
Runtime sources matched the recorded freeze, excluding the listed concurrent
site-maintenance paths. These partial local results do not close shared-host
scale, tuning, sustained CPU/memory, native recovery, other-platform or
four-core/8 GB acceptance. Automatic defaults remain gated. No release or site
publication is established.

### Startup lock race and 1000-agent retest

A later 90-second fixed/headless diagnostic used the earlier release images,
two participants and the same two-model/one-job envelope at 1000 agents each.
It completed 2000 and 1998 turns but failed because one active turn reported
`startup.lock already exists`. Cleanup passed. This remains a failed run.

The focused executor reproduction synchronized eight clients creating the same
permanent startup lock. It failed before the correction and passed afterward,
checking shared lock exclusion and later reuse. The correction accepts only a
competing creator's `AlreadyExists` result, then opens the original lock with
the existing safety checks. Other errors still propagate. An earlier test
attempt failed during setup because `/var` traversed a symlink; that attempt's
raw reproduction flag is wrong and does not establish the race.

After the correction, executor formatting, 77 tests and strict all-target
Clippy passed, with three native fixtures ignored. Fresh V5 CLI checks and both
five-second shared-root smoke cases also passed. These are focused checks,
not a fresh full repository gate or broad crash-recovery acceptance.
That checkpoint covers `AlreadyExists`; source review confirmed a separate
hard-link publication gap, recorded below. The approved repair and fresh
calibration followed those measurements.

The images with the tested EEXIST correction then passed the 90-second fixed/headless 1000-agent retest:

| Participant | Completed turns | Completed tools | Initial completion p95 | Restarted completion p95 |
| ----------- | --------------- | --------------- | ---------------------- | ------------------------ |
| 1           | 2000            | 2929            | 23.323 s               | 25.871 s                 |
| 2           | 2652            | 3000            | 23.438 s               | 44.479 s                 |

Both reported zero active/drain failures, 1000 pending turns at cutoff and
1000 drain cancellations, with no drain completions. Workload and cleanup
checks passed; all commands and observers were reaped. One registry snapshot
recorded process identity unavailable after an owner exited, so the run
does not establish success for every observation. Latency includes only
completed turns. This establishes useful progress in the selected workload on
this Mac, with no fixed/automatic or terminal-overhead comparison, stable tail
claim or broad scale acceptance.

The helper and fixture with the tested EEXIST correction have new hashes recorded in the
[verification follow-up](https://github.com/ejmlabs/ejm-cli/blob/main/openspec/changes/adaptive-agent-capacity/verification.md#startup-lock-race-and-1000-agent-retest).
Earlier startup and saturated-memory calibration retains its earlier image
hashes; it was not rerun for those images with the tested EEXIST correction.
The later [current-image calibration](https://github.com/ejmlabs/ejm-harness/blob/main/docs/benchmarks.md#real-embedding-helper-memory)
records separate CLI and worker hashes, three startup and three saturated
control-state trials per image, and a distinct eight-helper benchmark pool.
Those results do not calibrate the older EEXIST checkpoint. The original failed
30-second matrix, its cleanup audit and the failed 90-second case remain
unchanged. Automatic defaults and the full acceptance matrix remain gated.

### 4096-agent cutoff and queue diagnosis

The images with the tested EEXIST correction then ran a selected 90-second fixed/headless diagnostic
with two participants, separate session stores and one shared capacity root.
Each participant managed 4096 agents with four model requests and one native
job. The remaining per-participant envelope was 512 MiB of memory, 2000 CPU
millis, one heavy job, 32 processes, a queue of 256, 1 MiB of output, 64 MiB
of artifacts and a 120-second job limit. Its four-model envelope differs from
the earlier two-model 1000-agent retest, so these cases do not support a fair
speed or scale-cost comparison.

| Participant | Completed turns | Completed tools | Active failures | Drain failures | Cutoff pending | Drain cancelled |
| ----------- | --------------- | --------------- | --------------- | -------------- | -------------- | --------------- |
| 1           | 0               | 2960            | 0               | 0              | 4096           | 4096            |
| 2           | 0               | 3020            | 0               | 0              | 4096           | 4096            |

Neither participant completed a turn during drain. The case failed its
useful-work check with `fixed JSON did not establish successful useful work`.
Cleanup passed, commands and observers were reaped, and recorded runtime
sources stayed stable. The useful-work verdict remains failed.

A later read-only journal query found 2964/3028 first model steps, zero second
model steps, the same 2960/3020 finished tools and 4096 cancelled runs per
participant. It used a read-only SQLite URI, query-only mode and a 30-second
deadline per participant. The original benchmark bytes stayed unchanged.
This locates the observed work within the finite window; it does not establish
scheduling optimality, indefinite starvation or a throughput improvement.

Exact report and diagnosis hashes are recorded in the
[verification follow-up](https://github.com/ejmlabs/ejm-cli/blob/main/openspec/changes/adaptive-agent-capacity/verification.md#4096-agent-cutoff-and-queue-diagnosis).
The full acceptance matrix and automatic defaults remain gated.

### Automatic 1000-agent diagnostic and rejected longer window

The images with the tested EEXIST correction passed a selected 90-second automatic/headless diagnostic
with two participants managing 1000 agents each. They used separate session
stores, one shared capacity root and the same two-model/one-job envelope as the
passing fixed 1000-agent retest.

| Participant | Completed turns | Completed tools | Initial completion p95 | Restarted completion p95 |
| ----------- | --------------- | --------------- | ---------------------- | ------------------------ |
| 1           | 2000            | 2672            | 28.196 s               | 38.458 s                 |
| 2           | 2000            | 2488            | 28.152 s               | 36.879 s                 |

Both had zero active/drain failures, 1000 pending turns at cutoff, 1000 drain
cancellations and no drain completions. Latencies cover 1000 completed initial
and 1000 completed restarted turns per participant; unfinished turns remain
censored. Workload and cleanup passed, commands and observers were reaped, and
recorded runtime sources stayed stable. Four registry snapshot discovery errors
recorded unavailable process identities near owner exit. These passed workload
and cleanup checks do not mean every observation succeeded.

This establishes automatic-mode progress in the selected workload on this Mac.
The separate fixed and automatic diagnostics do not establish a repeated
throughput comparison, a sustained win or regression, or full acceptance.

An attempted 180-second fixed 4096-agent follow-up failed configuration
validation. The terminal observer accepts only 5 through 120 seconds and
rejected `--seconds 180` before launching the CLI or native fixture. Both Python
observers exited with code 2 and were reaped. The aggregate cleanup verdict
failed because expected native identities were absent. It does not establish
surviving native work. No 180-second workload ran. The original report and both
stderr files remain unchanged.

Exact hashes and the separate configuration-failure interpretation are recorded
in the
[verification follow-up](https://github.com/ejmlabs/ejm-cli/blob/main/openspec/changes/adaptive-agent-capacity/verification.md#automatic-1000-agent-diagnostic-and-rejected-longer-window).
Automatic defaults and full acceptance remain gated.

### Automatic 4096-agent diagnostic and remaining lock work

An earlier selected diagnostic used automatic capacity, two participants with
4096 agents each, separate session stores and one shared capacity root. Its
120-second active window and two-model/one-job envelope differ from the earlier
90-second fixed 4096-agent case's four-model envelope. The cases do not support
an equal-envelope fixed/automatic comparison.

| Participant | Completed turns | Completed tools | Active failures | Drain failures | Cutoff pending | Drain cancelled |
| ----------- | --------------- | --------------- | --------------- | -------------- | -------------- | --------------- |
| 1           | 0               | 3457            | 0               | 0              | 4096           | 4096            |
| 2           | 0               | 3687            | 0               | 0              | 4096           | 4096            |

Both had zero drain completions. The case failed with
`auto JSON did not establish successful useful work`. Cleanup passed, commands
and observers were reaped, and recorded runtime sources stayed stable. Finished
tools establish tool progress during this workload. This failed case does not
establish completed-turn acceptance at 4096 agents or a throughput result.

The earlier startup-lock reproduction and correction tested the competing
creator's `AlreadyExists` path. Source review confirmed a separate
publication race: `StateDir::write_new` creates a hard link before unlinking
the temporary name, while `open_read` requires a single link. That separate
path can reject a concurrent reader, and publisher death can leave both names.
The reviewed maintained APIs do not preserve all required directory, permission
and permanent-lock guarantees when avoiding that gap. See the harness's
[stable-lock repair](https://github.com/ejmlabs/ejm-harness/blob/main/docs/host-capacity.md#stable-lock-creation-repair).
The subsequent full harness gate reproduced the concurrent-reader failure in
`competing_startup_clients_share_one_stable_lock`. A later approved temporary
dependency patch passed focused macOS creator-death, concurrent-access and
privacy checks, followed by a passed full harness gate. The latest full CLI
gate still fails in `startup::startup_failures_reap_all_loaded_package_hosts`:
`enable` immediately after successful `install` cannot acquire the package
mutation lock. The proposed explicit-unlock repair awaits OpenSpec approval
and is not implemented. Package
publishing waits for equivalent upstream APIs. These checks do not establish
all-platform startup, crash recovery or broad scale acceptance.

The
[verification follow-up](https://github.com/ejmlabs/ejm-cli/blob/main/openspec/changes/adaptive-agent-capacity/verification.md#automatic-4096-agent-diagnostic-and-remaining-lock-work)
pins that report and participant sidecar. Earlier failures remain
unchanged. Full acceptance and automatic defaults remain gated.

A later [current-image pair](evidence/native-coordinated-current-4096.macos-arm64.json)
passed separate fixed and automatic headless diagnostics. Each used two
processes with 4096 agents each, the same two-model/two-job ceiling and a
120-second window. Fixed completed 3514 whole turns; automatic completed 3740.
Each mode completed 8192 tools, with zero active or drain failures. All pending
turns were accounted for and cancelled during drain; cleanup and source checks
passed. Neither mode completed repeat turns. One sequential case per mode does
not establish steady state, a full latency distribution, tuning quality, UI
overhead, a causal speed improvement or throughput acceptance. The linked
index retains exact envelopes, images, per-participant results and raw evidence.

### Historical trace comparison from 2026-10-21 UTC

The [retained diagnostic](evidence/capacity-trace-overhead.macos-arm64.json)
covers six 30-second release-profile cases, paired in trace-off/on, on/off and
off/on order. Each used fixed capacity, two interactive processes, separate
session stores and a fresh shared host root. Each process managed 100 agents
with two model requests and one native job. Stop was disabled; each scripted
interaction had one sample.

The frozen helper was 18,109,872 bytes, SHA-256
`c386f2c07e9242284b3fff705d7b55a262e03cc6d02d2bf737587fb2fdc11d4a`;
the fixture was 19,637,392 bytes, SHA-256
`2ad9dd2ec4a35f454b201f3c22ea50e119ec39bcf402b136ab28692d7e01073f`.
These images used the earlier separate trace subscription. Later observer
revisions require fresh measurements.

| Pair | Trace off, turns/s | Trace on, turns/s | Observed lower traced rate |
| ---- | ------------------ | ----------------- | -------------------------- |
| 1    | 39.695738          | 36.666799         | 7.6304%                    |
| 2    | 36.667732          | 29.687817         | 19.0356%                   |
| 3    | 44.400595          | 39.999979         | 9.9112%                    |

Each rate sums the participants' completed turns per controller second.
Repeat spread, calculated as range divided by median, was 19.4803% without
tracing and 28.1240% with tracing. Every participant had 100 pending turns at
cutoff, all cancelled during drain, with no drain completions. The hypothetical
cutoff ranges in the record are not confidence intervals or observed work.
These varying, censored windows do not establish causal tracing overhead or
the 5% target.

Useful-work and cleanup checks passed with zero active or drain failures.
Commands and observers were reaped, and recorded sources stayed stable.
Approximate controller-window overlap ranged from 99.961% to 100%, without a
shared barrier. Registry sampling retained 20 process-discovery errors: 4, 4,
3, 4, 1 and 4 in execution order. Their cause is unproven; passed cleanup does
not establish success for every observation.

The six traced participant reports were `diagnostic_only`, retaining 44
receipt-to-write and 44 commit-to-write samples. They recorded zero lag signals,
closures, observer errors, missing commit stamps or omissions. Each recorded
two selection changes and three subscriptions. The separate observer and
unanchored selection changes still leave first-write correlation and selection
coverage unverified. These results establish neither selected-event latency
acceptance nor 1000/4096-agent trace acceptance. The observer's 120-second cap,
open acceptance tasks and automatic-default gate remain unchanged. No release
or public-site publication is established.

Those six verdicts came from the frozen earlier reporter, before
`generation_resets` and `uncovered_saved_markers` existed. The revised reporter
classifies those raw reports as `incomplete` because that accounting is absent.
The historical verdicts and missing fields remain unchanged.

## Selected-view release diagnostics on 2026-10-21

[The retained record](evidence/capacity-selected-path-trace.macos-arm64.json)
covers the revised selected-view trace with distinct model-round markers. The
frozen release images were built before the separate package-operation unlock
repair. Each case used two participants with separate session stores and one
fresh shared capacity root, two model requests and one native job per
participant. These offline macOS arm64 results do not establish another host,
platform or hosted provider's capacity.

| Agents per participant | Cases | Work seconds per case | Timed live markers |
| ---------------------- | ----- | --------------------- | ------------------ |
| 100                    | 4     | 30                    | 90                 |
| 1000                   | 1     | 90                    | 14                 |
| 4096                   | 1     | 90                    | 4                  |

The 100-agent cases alternate trace off/on and on/off. Their aggregate
completed-turn rates were 76.135549/66.366068 and 74.401269/74.700852 turns/s,
without/with tracing. The observed traced changes were -12.8317% and +0.4027%.
Repeat spread, range divided by median, was 2.3041% without tracing and 11.8168%
with tracing. These short, censored windows do not establish causal overhead
or the 5% target. The larger cases are single traced diagnostics, with no
matching overhead or automatic-capacity comparison.

All six useful-work and cleanup verdicts passed, with recorded sources stable
and direct commands and observers reaped. The eight traced participants
retained 108 receipt-to-write and 108 commit-to-write samples. They reported no
lag, closures, observer errors, missing commit stamps, uncovered saved markers
or omitted samples. Each changed selection twice and subscribed three times.
Their trace verdict remains `diagnostic_only`; complete first-write coverage,
selection continuity, trace cost and selected-event latency acceptance remain
unverified. The six cases retained 18 registry process-discovery errors.
Passed cleanup does not establish error-free observation or containment of
arbitrary detached descendants.

Each scripted input, picker and switch action has only one sample per
participant. Stop was disabled for these throughput windows. Fixture setup
plus first paint took 376.330/381.314 ms at 1000 agents and
1498.602/1496.322 ms at 4096. That clock includes creating and starting every
fixture session, writing and parsing the ready list, and terminal observation.
It does not measure reused-state warm-composer startup or its p95 target.
Open acceptance tasks, automatic defaults and publication remain gated.

## macOS representative acceptance, 2026-10-22

- Exact public filesystem fork: `ad6faff2442ffe6abccd1856f4c0df9a3e472035`; source builds need no crates.io login.
- Representative acceptance passed: 1,000 total agents, two processes, separate stores, three fixed sweep points and two alternating paired trials.
- Median automatic throughput loss: -1.1594% (slightly faster than paired fixed). Median active-terminal overhead: 1.8826%. Individual second-trial overhead was 8.29%; the predeclared aggregate gate passed.
- Required retained-sample input, Stop and selected event-to-paint p95 boundaries stayed below 50 ms. Pressure, learned restart, reservations and cleanup passed. All commands and observers were reaped.
- Strict CLI Clippy, release helper/fixture builds, four Stop regressions and shared-root smoke passed. The earlier relay contract run passed 19 tests; controller regression run passed 18 tests.

See [the checkpoint](capacity-integration-checkpoint.json) for exact local receipts, hashes and limits. Earlier failed trials are retained; changing host fixed rates prevent attributing the improvement solely to the tuner repair.
