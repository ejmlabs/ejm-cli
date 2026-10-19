# Native remote publication candidate

Prepared: **2026-10-19 UTC**. This source-only successor starts from frozen
integrated native tree `b7569e88633687e5a22e89f8a90d57504137c609` on actual
protected CLI PR #9 merge `c8e2239392f71df82cbef6166b1827c8e5f5f1c5`, tree
`d49abd3f2881f8a2328063bc45d9ded4a078e00f`, identical to reviewed head
`7b5dc84632e79498e6cabc45a425595b9ac867a3`. It preserves the frozen native corrections, tests, gates, lock, exact b2/40d
context, Linux setup and owner work, including PR #6's guide consolidation and
deletion. The reviewed registry consumer handoff adds the public vault deletion
and controlled migration command described in the [registry guide](ejm-cli.md#registry-and-publishing). It selects the qualifying protected completion
API merge 42e below. Earlier native 61aa-based results remain historical. The
combined normal gate, hosted review and native adoption remain pending. Recheck
the protected source after the separate public cutover before admission.

## Included source

The native module contains account/device HTTP, a certificate-rejection TLS
fixture, endpoint identity, controlled legacy migration into the public shared
vault, invitation encoding, Snow Noise channels, bounded version-1 messages,
exact-session/run mutation authority, host lifecycle and native controller.
Local and remote prompt actions pass through `LocalBackend`; controller
readiness and `RemoteBadge` state use the existing App and renderer. The command
catalog advertises only this layer's implemented local and native commands.

Shared `main`, `args`, App, renderer and UI changes are selective. The base
provider constructors, login flows, package-provider selection and noninteractive
execution remain in place. `remote accounts` and a rejected non-terminal
`remote connect` bypass provider and journal setup; the accompanying CLI contract
uses a missing replay input to catch accidental provider initialization.
Open panels retain Enter handling even when the composer contains a `/remote`
draft. Local slash status supplies metadata without adding a provider subsystem.

The authored cut spans published-prelude tree
`cc2bc0f0330fb8464cca4060886fc2f1e4be9297` to native tree
`ecb6135bfeea6d3b41cb17c213a611da5e8a544a`. Its 49 paths include selective
shared changes; runtime, tests and build inputs retain those exact Git blobs.
This restack applies only the 51-path delta from approved selector head
`dd0b1fb72e399623458debef8bc28554407c789a` to local native commit
`395524dd45a1096e4949d0eba742db17d1aaa940`. The baseline retained all 40 native code, test,
lock, gate, spec and landing-page inputs byte-for-byte. The prepared corrections
below change only their named source and public contract inputs; the earlier
identity proof and runtime pass remain historical.
The preparation receipt binds those facts and the preserved parent paths.
This layer has no dependency on later provider onboarding, model selection or
site framework work. It retains the base authored landing page and updates its
native source status. A later docs-site migration must receive the content
through its own sources and build. No live publication is claimed.

## Dependency integration

`crates/ejm-cli/Cargo.toml` requests exact native versions documented in
[remote dependencies](remote-dependencies.md). `Cargo.lock` is the exact resolved
native lock, SHA-256
`5143432fc0201bedeef680d18ae1c4681e2215fddd6e2fb45cd4cbf0149f7d1b`.
This restack does not regenerate it. `.github/harness-revision` selects actual
protected API merge `42e4f9cf33bd3525aec344cfdffabc251271f782`, tree
`826e02310f1097e13806ca5ca1bf5cc00d71e966`, including exact-run completion.
The physical sibling is its tracked-source export: 358 verified files, with no
Git state, caches or linked source. All ten harness Cargo manifests remain exact.
Root verified the ordinary protected merge, matching reviewed tree and unchanged
checks at 22:44 UTC, then freshly read public CLI main C8, harness main 42e, restored
Actions/callers and unchanged CLI protection at 22:46 UTC. Those receipts qualify
the selected source, not native acceptance. Fresh locked resolution and the
normal full gate on this final pair remain required before admission.

The reviewed harness revision must export these public Rust contracts. The
completion correction below consumes the additive API at the selected protected pin:

- `ejm_harness_auth::{SecretId, SecretString, SecretVault, VaultKeyStore}` for
  namespaced, transactionally committed secrets. Vault transactions expose
  `get`, `set`, `delete`, `migrated`, `mark_migrated` and `commit`; reloading must
  verify durable values. Tests use the public memory and injectable key-store
  constructors, not private vault internals.
- `Harness::steer_run` and `Harness::cancel_run` target an exact current run,
  with the ordinary session/event/replay/run-handle APIs already used by the CLI.
- Existing package, process and terminal-prerequisite public APIs from the
  earlier protected harness publication remain required.

This preparation uses actual protected C8 and qualifying protected harness 42e.
Review the docs/publication boundary before native admission.
A fresh hosted run through installed 40d must prove that the new pinned
API commit equals
or precedes protected main in the fixed harness repository. A disconnected reviewed head
after a squash merge is insufficient even when its tree matches. Preserve
checksum/source policy and recheck the exact `deny.toml` exceptions. Relative
sibling paths do not establish source identity. Public source needs no private
checkout credential; any private checkout still requires dedicated, expiring
Contents: read access scoped to that repository.

Linux native persistence needs distribution libdbus development files to build,
plus a session D-Bus and unlocked Secret Service at runtime. The earlier local
pass covers macOS compilation. Linux and Windows compilation and real native
key-service operation still need separate evidence. Memory-only operation is
an explicit user choice, not an automatic storage fallback.

## Prepared completion and prompt corrections

Review of native head `2c20695ed87a88675357eca60845f353158fb92a` found that
`RunFinished` is durable before bounded run-end observers and active-run cleanup.
Starting a queued prompt on that event can report busy and discard accepted
successors. The wire validator also allowed 64 KiB although the selected harness
accepts only 16 KiB.

The protected harness-owner change supplies
`Harness::subscribe_run(session_id, run_id).await -> Option<RunHandle>` and
clonable handles for the exact current run, including internal queued successors.
Detailed `RunCompletion` retains the driver's atomic `queued_successor` decision;
legacy summary-only wait remains compatible.
The prepared native pair consumes that public API at actual protected merge 42e,
whose tree equals the reviewed 358-file source. The pin is updated and Cargo lock
remains exact; this consumer's matching full/hosted gates are still pending.
The completion change adds no package dependency or background task.

`LocalBackend` retains one handle for its current revocable queue target. It
waits outside host authority, refreshes durable state and rechecks the same share,
expiry, target and current run before popping a prompt. It discards the dependent
chain when detailed completion reports a queued local successor, even if that
successor finished before replay. Revocation or a
replacement share wins while observers are blocked. A missing handle, session
Idle or finished event alone never authorizes admission. Every text mutation
uses one 16 KiB UTF-8 byte bound. The existing encrypted transport fixture now
uses a valid bounded prompt and keeps its ciphertext assertions.

The new contracts exercise a real internal queued target, two remote successors,
stop and replacement during the wait, a fast local successor overtaking the host,
and 16,384 versus 16,385 multibyte UTF-8
bytes without losing valid pending work. On 2026-10-19, the retained focused
0a7a/826e run passed all 12 backend contracts. Event-only completion and a 64 KiB
validator each failed their intended regression. The corrected log SHA-256 is
`5506447deee7629e5ce05412f26a572d4ae97598ec8bd0c544dce07a7350f165`.
This prepared successor preserves those test/runtime inputs; it has no combined
full-gate pass. Hosted source binding, two complete reviews, explicit approval,
protected adoption and live relay/docs acceptance remain separate.

The display-recovery focused run separately passed both corrected contracts;
removing suppression failed both, exposing out-of-order 18 versus 16 and 1040
versus 528 sequence values. Corrected log SHA-256:
`000388f9fc871acbd7f210f63c16dafa5bc12c1eebe0b4ba15cefdd521997759`.
These local fixture results do not prove production latency or live recovery.

## Historical local pass

Root's normal local commit `395524dd45a1096e4949d0eba742db17d1aaa940`,
tree `5e2e3c61ff6150353bbddda656266c7cd5ab39f8`, passed on macOS on
2026-10-19 against then-selected protected harness `220305fe...`: 116 unit tests, 6
integration tests, formatting, strict Clippy, release build, Rustdoc, Machete,
Deny and the ordinary/private PTY gate. The log is
`normal-commit-20261019T202331.021400Z.log`, SHA-256
`3e49bc00bb6bcb8f96f055f20afccefd49af517dacec54a7e6156cad67f819a8`.
Its parent was the then-prospective `dd0b1fb72e399623458debef8bc28554407c789a`.
The protected CLI merge has the same tree as `dd0b1fb...`, but this
status-refined revision still requires its own normal local gate and exact-head
hosted acceptance.

## Verification still required

This preparation checks source identity only and runs no Cargo or hooks.
Run the normal `./scripts/check` after the final protected-parent restack,
including locked formatting,
strict Clippy, all native tests, release/doc builds, unused-dependency and
license/advisory checks, and the existing real PTY gate. Keep Cargo jobs at most
two and use local replay/HTTP fixtures rather than paid provider calls.

The native test source covers role and mutation authority, stale cursors,
deduplication, uncertain results, reconnect, handshake tampering, account origin
and body bounds, TLS rejection, fresh preparation and missing-key recovery,
host task ownership and private input cleanup. These are prepared contracts,
not a passing count for this candidate. `./scripts/check` now drives the ignored
`remote::host_ui::tests::remote_terminal_interaction_fixture` through its explicit
PTY supervisor. The gate captures compiler-artifact JSON from its existing locked
Cargo test run and passes the exact native unit-test executable to the supervisor.
It rejects missing or ambiguous artifacts instead of scanning a shared build
directory by modification time. It then checks host flow, complete and partial
queued paste, cancellation,
interrupt, error, signal and panic. The ordinary interaction and restoration
checks remain. A focused rerun is `python3 scripts/check_terminal.py --remote-only
CARGO_TEST_MESSAGES_JSONL`, using stdout captured from the current checkout's
`cargo test --workspace --all-targets --locked --message-format=json-render-diagnostics`.

The admitted review of head `2c20695...` found a separate SIGTERM gap in the
real remote command entry. The correction reuses the existing UI signal helper.
An added supervisor case runs the built CLI's `remote migrate-credentials` with
an offline relay origin, reaches its protected confirmation, then sends SIGTERM
before confirmation or credential access. It checks normal cancellation,
private-input secrecy and terminal restoration. Run just this regression with
`python3 scripts/check_terminal.py --entry-signal-only PATH_TO_EJM_CLI`.
The retained old 2c regression failed as intended, exit 1, log SHA-256
`913f3c5b244e2d7ba8e8dc6d7549032915e4e8e30b4f8f9a976863c0da51969a`.
The corrected case and combined full gate remain unrun in this preparation.

The selector uses Cargo's manifest path, binary target and test profile, including
fresh cache hits. It never builds another test binary or uses a relay. Run that gate
on Linux and macOS and retain separate Windows terminal evidence. Verify that
the admitted context binds the completion API's actual protected revision and
tree, `pin_source: head`, the exact native head and protected-main authority,
plus the actual harness archive digest and full source inventory. The old base pin or a green wrapper without
that binding is insufficient. Run public
docs/site checks and verify publication separately.

`python3 scripts/check_terminal.py --native-fixture CARGO_TEST_MESSAGES_JSONL`
prints only the validated executable path for other native fixture supervisors.
It uses the same selector and does not enter a PTY. The JSONL must come from the
current checkout and invocation; never reuse a capture from another build.

## Following Worker/browser layer

No Worker package, WASM crate, npm lock, production Cloudflare configuration,
commercial policy, account dashboard or browser assets are included. Native
forwarding requires an independently configured compatible relay service.
`transport_fixture.rs`, `native_roundtrip.rs` and the `worker_fixture` module
declaration are deliberately excluded. Add those files and their native/Workerd
and real-browser supervisors together with the Worker/browser layer, then run
its complete gates. Do not add unconditional relay build calls to this earlier
slice.

The existing `private-remote-control` main spec accompanies the native source.
It also records the browser contract supplied by the following layer; its
presence is not an implementation or verification claim for that layer. The
inherited deployment delta targets `cloudflare-relay`, whose main spec must
arrive with the Worker layer before that change can be archived. OpenSpec
validation reports that archive prerequisite separately.

Docs PR #6 is merged in the candidate parent. This preparation preserves its
consolidation and prelude-guide deletion; still-needed native status now lives
in the consolidated and native guides. It does not overwrite another owner's
site migration or claim that these docs are published.

The deployment change remains open. Source preparation does not satisfy
protected admission, review, required hosted checks, release adoption, real
OAuth or Cloudflare deployment. Its OpenSpec task checkboxes remain unchecked.
