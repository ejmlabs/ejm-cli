# Native remote source and review inputs

Native [CLI PR #21](https://github.com/ejmlabs/ejm-cli/pull/21) reviewed head
`6cbc5b7d1f8cd70109dd72ae464bef29f8483f11` merged through protection on
2026-10-20 at 23:19:40 UTC as `71d0f47f8b05c6314eca4dc0750ffaad833d6afb`,
tree `3adf3701aec18883b2aabd710e72fa3c513c7fdf`. Its sole parent is protected
CLI PR #20 merge `9e3f8ef11dc083c31b651b9dae15a98e086e3359`.
[Run 39511620937](https://github.com/ejmlabs/ejm-cli/actions/runs/39511620937)
passed Linux/macOS quality with 133 unit and six integration tests per platform,
plus PTY checks. Both raw exact-head reviews were complete with zero findings;
exact approval comment `6303964435` preceded the normal protected merge.
Controls stayed unchanged. This accepts the native source, not a live relay,
binary distribution or published docs site.

PR #20's [historical UI prerequisite](ci-bootstrap.md#ui-source-review-prerequisite)
selected UI merge `24935aa85355ea99f49bdbecc4186a5e95049393`, tree
`2d8bde67c8b5fe851b70ae90a3a2787091397780`, with complete install review data.
The [following inert prerequisite](ci-bootstrap.md#current-inert-shared-ui-and-relay-inputs)
advances that UI selection and prepares Relay install review data on native 71d0.
It retains the thirteen-file install boundary and adds a separate complete
changed-owner source supplement after PR #22's two incomplete reviews. That
successor still needs its own gates and protected adoption. This native layer includes
no Worker or browser UI. The retained sections below describe earlier cutoffs;
their then-pending native checks are superseded only by the exact receipt above.

Earlier native preparation started from `06154b18137eb376bef986a520904381b8744b9d`
on protected PR #9 merge `c8e2239392f71df82cbef6166b1827c8e5f5f1c5`,
preserving 40d caller context, Linux setup, owner work and PR #6 consolidation/deletion.
The 061 normal gate passed 129 unit tests, 6 integration tests and all 14 private
terminal cases; its five old-behavior controls failed as intended. Its admitted
[run](https://github.com/ejmlabs/ejm-cli/actions/runs/39350796485) bound the exact
264-file CLI and 358-file harness source and supplied both as plaintext to 40d
reviewers. Linux quality passed; macOS tests passed before the job was cancelled.
The first complete review was clear, but the second found snapshot authority and
expiry defects. [Admission was revoked](https://github.com/ejmlabs/ejm-cli/pull/10#issuecomment-6280986645).
On October 20, corrected staged tree
`d8dc3fd3e1f1e24deb04c75f70e0f0562f7145d1` passed the normal macOS native gate:
131 unit tests, six integration tests, strict formatting and Clippy, the locked
release build, Rust documentation, dependency checks and PTY interaction and
restoration. One private terminal fixture remains ignored in the unit runner
because the PTY supervisor drives it. The source includes a test-only injected
clock with a conditional, reasoned lint expectation for the production helper.
That local result applies only to d8. The later exact-source native gate and
hosted acceptance are recorded above; the earlier admission remains revoked.

## Account persistence correction

Native PR #21 head `45a8d3da23f8f2a17096f2de1f626c7d5c822760` passed its
normal local gate, but its second complete hosted review found linked JSON
writes and split credential/metadata publication. The correction stores both
in the existing public vault, preserves newer logins during logout and retains
bounded legacy input only for controlled migration. Fourteen vault and four
account HTTP/TLS cases passed locally. Four valid negative controls caught missing
index publication, stale metadata, stale tokens and missing migration index repair.
Those focused results preceded the corrected normal gate, fresh admission and
protected acceptance recorded above; they do not accept the following inputs.

## Earlier review corrections

The f66 normal macOS gate and [hosted Linux/macOS quality](https://github.com/ejmlabs/ejm-cli/actions/runs/39344263873)
passed 127 unit and 6 integration tests per platform. Both complete AI reviews
blocked adoption on four distinct defects; [admission was revoked](https://github.com/ejmlabs/ejm-cli/pull/10#issuecomment-6280152832).
At that cutoff, the successor required regressions, the normal gate, fresh hosted
reviews, approval and protected adoption. PR #21 later supplied its own accepted
receipt above. All eleven deployment tasks stay open.
No live relay, binary or docs release is claimed.

## Retained four review corrections

- Draw each new fingerprint confirmation, then purge buffered input before
  accepting a fresh approval key. Remove confirmations whose receiver has closed.
- Pass operation cancellation to public vault locks; cancel their blocking
  worker on caller drop. An OS key-store call already in progress is not interrupted.
- Recover an authoritative snapshot after a delivered server event gap.
  Missing output never establishes Idle or permits a new mutation.
- Decode invitations until the eight-hour share deadline; retain the host's
  five-minute first-enrollment deadline and both endpoints' pinned-key checks.

## Snapshot and expiry corrections

Refresh ordered authority after selecting a snapshot page so its active-run
state covers every event through the returned cursor, at the cost of one extra
ordered journal pass per snapshot. Recheck fresh platform
milliseconds after authority contention and journal refresh, immediately before
direct or queued mutation admission. Receipt time cannot extend an expired share. New deterministic
contracts cover completion during snapshot construction, a following ordinary
Submit and expiry at both wait boundaries. The local source result above does
not establish final-source or hosted acceptance.

The protected docs/UI prerequisite kept C8's existing runtime, harness pin and
local gates. Native PR #21 later completed its own protected adoption. Main-spec
release verification remains separate. Preparing a tree or splitting review
scope establishes no acceptance.

## Included source

The accepted native source contains account/device HTTP and TLS rejection coverage, endpoint
identity and vault migration, invitations, Snow Noise, bounded version-1 messages,
exact-session/run authority, host lifecycle and the native controller.
`LocalBackend`, readiness and `RemoteBadge` share existing App/UI paths.
[Setup, controls, trust and recovery](remote-control.md) remain the user guide.
Shared integration preserves provider/login/package and noninteractive behavior.
`remote accounts` and rejected non-terminal `remote connect` bypass provider and
journal setup. Open panels keep Enter with a `/remote` draft; the catalog lists
implemented commands only, and slash status adds metadata only.

## Dependency integration

The current native lock SHA-256 is
`02ba0f0097754444a40e106d683ce530886ec7c31d30009f0c5cc43750d6118b`.
The historical d8 lock was
`5143432fc0201bedeef680d18ae1c4681e2215fddd6e2fb45cd4cbf0149f7d1b`.
On October 20, the normal hook stopped at Deny's TLS advisory check after
131 unit and six integration tests, release/Rustdoc and Machete passed; no
commit was made. The corrected graph and extended WSS rejection fixture then
passed the fresh full native gate and hosted review recorded above. No earlier
pass supplied that acceptance; preserve the tested lock in the inert prerequisite.
Versions and package evidence are in [remote dependencies](remote-dependencies.md).
The pin selects protected harness merge
`42e4f9cf33bd3525aec344cfdffabc251271f782`, tree
`826e02310f1097e13806ca5ca1bf5cc00d71e966`: 358 tracked source files,
all ten Cargo manifests exact, without linked source, caches or sibling Git state.

The CLI consumes only these public harness contracts:

- `SecretId`, `SecretString`, `SecretVault`, `VaultKeyStore` and transaction
  `get`, `set`, `delete`, `migrated`, `mark_migrated` and `commit`, then reload.
  Tests use public memory/injectable key stores; no plaintext fallback is permitted.
- `Harness::steer_run`, `Harness::cancel_run` and
  `Harness::subscribe_run(session_id, run_id).await -> Option<RunHandle>`.
  Handles clone; detailed `RunCompletion` retains `queued_successor` while
  summary-only wait stays compatible.
- Session/event/replay, package, process and terminal-prerequisite APIs;
  [registry setup](ejm-cli.md#registry-and-publishing) covers logout and migration.

Admitted context must bind `pin_source: head`, product head, harness commit/tree,
archive digest and inventory, and prove ancestry to protected main in the fixed
harness repository. Relative paths prove no identity. Preserve checksum/Deny
policy; private checkout needs dedicated, expiring repository Contents: read
access. Public source needs no private credential.

Linux persistence needs libdbus development files to build, plus a session
D-Bus and unlocked Secret Service at runtime. f66 quality covers its Linux/macOS
fixtures, not every native key service or Windows terminal. Memory-only storage
is an explicit choice, never an automatic fallback.

## Prepared completion and prompt corrections

Queued prompts stay revocable through exact completion and bounded observers.
After local input, the host polls its one retained handle and rechecks durable
state, share, expiry, target and current run. An atomic local-successor handoff
discards the chain even after that successor finishes; Idle, missing handles or
finished events cannot authorize it. Local stop cancels session cleanup and
successors; remote cancellation stays run-bound. Text mutations share a 16 KiB UTF-8 bound.
Display catch-up suppresses live events through acknowledged journal pages and
their highest missing sequence, including short/stale snapshots. Control replies
continue; mutations never retry. No new dependency, queue engine or event buffer
is added. The [remote guide](remote-control.md) gives the full recovery contract.

## Historical local pass

Git history and frozen receipts retain these earlier results.

| Source and check | Log SHA-256 |
| --- | --- |
| 395524dd local gate, then harness 220305fe | `3e49bc00bb6bcb8f96f055f20afccefd49af517dacec54a7e6156cad67f819a8` |
| Old 2c real-entry SIGTERM regression, intended failure | `913f3c5b244e2d7ba8e8dc6d7549032915e4e8e30b4f8f9a976863c0da51969a` |
| 0a7a/826e twelve backend contracts | `5506447deee7629e5ce05412f26a572d4ae97598ec8bd0c544dce07a7350f165` |
| Two ordered display-recovery contracts | `000388f9fc871acbd7f210f63c16dafa5bc12c1eebe0b4ba15cefdd521997759` |
| e740 normal native gate | `3577699c6de8720a80d78696fe0d1072880c44c91c197cfdfe8ec725404df8d2` |
| f66 normal native gate | `bc2e884624a6d270e03f1a0be99e0366c7c8849d4eaa7f9eda1109cefafd0b75` |
| 061 normal native gate | `5313d5ed8f1a065e69825c13b467edb4c3b7ce08066f9429fbc2e6673619ff15` |

Hosted f66/42e bytes, modes and Git blobs are bound by receipt SHA-256
`4f40faadb2b3ad8c969b23f4f9b2c8379ab56d20db1ba8648a8e79e9c4ea372b`.
Both reviews still rejected f66; local results prove no live latency.

## Verification for each following source

The native receipt above completes these checks for PR #21's exact source.
The inert prerequisite and later Worker source each require a fresh normal gate,
exact-head admission, two complete paid reviews, separate final approval and
protected merge. Retain the accepted native baseline and its contracts below.

Run unchanged `./scripts/check` with at most two Cargo jobs and offline fixtures:
architecture, format, strict Clippy, native tests, release/Rustdoc, Machete,
licenses/advisories and PTY. Retain authority, stale cursor, deduplication,
uncertain result, handshake, origin/body/TLS, vault preparation/missing-key,
host ownership and private-input cleanup contracts.

PTY selection uses compiler-artifact JSON from that current-checkout invocation,
including cache hits: `cargo test --workspace --all-targets --locked
--message-format=json-render-diagnostics`. Missing/ambiguous artifacts fail;
mtime and a separate build never select the test binary.

```sh
python3 scripts/check_terminal.py --remote-only CARGO_TEST_MESSAGES_JSONL
python3 scripts/check_terminal.py --entry-signal-only PATH_TO_EJM_CLI
python3 scripts/check_terminal.py --native-fixture CARGO_TEST_MESSAGES_JSONL
```

Retain all eight private host fixture cases, ordinary interaction/restoration,
memory-only startup and stop/switch/quit/signal during blocked observers.
The real-entry SIGTERM case reaches protected migration confirmation without
credentials or network. New regressions must fail the old pairing input,
vault contention, server-gap and expired-enrollment decode paths.
The selector-only command prints the exact executable without opening a PTY.
Run the full gate on Linux/macOS; Windows needs separate terminal evidence.
Each following source requires fresh source-bound quality, two complete reviews,
explicit approval and protected adoption. Separately built/published docs and
live acceptance remain pending.

## Following Worker/browser layer

The original native slice needs a separately configured compatible relay and
included no Worker, WASM, npm lock, Cloudflare config, commercial policy,
dashboard or browser assets. The following inert prerequisite adds final-path
`relay/package-lock.json`, a short README and the complete 51-line manifest as
a content-addressed original prepared candidate snapshot outside the runnable
package, with provenance. It adds no runnable `relay/package.json`, so the
Relay setup/install step remains skipped. Its
[protected UI target and thirteen-file boundary](ci-bootstrap.md#current-inert-shared-ui-and-relay-inputs)
remain distinct from the complete changed-owner source supplement and later
full target archive/install proof. The supplied 932-file M archive plus 28
complete current entries exposes the 941-file target view. The older archive
could not supply changed current TSX/CSS; the supplement supplies those complete files.
Review metadata and supplement
fixtures are never imported, installed, executed or packaged. Production still
uses public owner exports. The centralized prerequisite record binds source
identities, byte counts and the revoked old-head review; no quality pass is
claimed for its cancelled platform jobs. The successor needs new exact-head
human readiness and two complete paid reviews after its own normal full gate,
then separate final approval and protected merge.
Keep the accepted native lock, 42e harness and 40d callers unchanged; add no
Worker/config, root gate, Cargo, harness or workflow change in that inert slice.

After its protected merge, the full Worker/browser PR must inherit the new UI
pin from protected BASE, verify the exact full UI archive and lifecycle-disabled
installation, and pass the full native/WASM/Worker/browser gate before closed
deployment. Both slices retain the 200-path, 12,000-line and 1,000,000-byte caps.
Add `transport_fixture.rs`, `native_roundtrip.rs`, `worker_fixture` and their
Workerd/native/browser supervisors with that layer, never unconditional gates here.
The main spec's browser contract does not implement or verify it. Its
`cloudflare-relay` main-spec archive prerequisite also arrives with that layer.
Preserve concurrent site ownership and verify live publication separately.
