# Native remote publication candidate

Prepared: **2026-10-19 UTC**. This native layer starts at actual protected
CLI merge `61aa4b4b137180ba6d76210b2cb5321cdc59658c`, tree
`504892783c5b2b49db490582f4a11a782b8542df`. CLI PR #7 protected-squash
merged the approved `dd0b1fb72e399623458debef8bc28554407c789a` selector
head with that exact tree and unchanged protection. Its B2 callers, source
context, owner standards and PR #6's guide consolidation and deletion remain
intact. The [selector receipt](ci-bootstrap.md#cli-pr-7-selector-adoption-receipt)
is prerequisite acceptance, not native source acceptance or service release.

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
`395524dd45a1096e4949d0eba742db17d1aaa940`. All 40 native code, test,
lock, gate, spec and landing-page inputs remain byte-identical. The only later
changes record the actual protected parent and historical local pass.
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
This restack does not regenerate it. The dependency pin is actual protected
harness API merge `220305fe4be7fecd33c3cf04b24f7a1b4b50503d`, tree
`518bd4ecfc28b31832465df083fe53df9cd053dd`. The physical sibling contains only
that tracked source. The source tree equals the earlier provisional
`8056a85e1b814fc4e09455a1156cdeed137b80c4` input, but the pin now names the
actual protected merge. All ten harness Cargo manifests match those native
integration inputs. This source identity is not a new locked build result;
fresh locked resolution and compilation remain required.

The reviewed harness revision must export these public Rust contracts:

- `ejm_harness_auth::{SecretId, SecretString, SecretVault, VaultKeyStore}` for
  namespaced, transactionally committed secrets. Vault transactions expose
  `get`, `set`, `delete`, `migrated`, `mark_migrated` and `commit`; reloading must
  verify durable values. Tests use the public memory and injectable key-store
  constructors, not private vault internals.
- `Harness::steer_run` and `Harness::cancel_run` target an exact current run,
  with the ordinary session/event/replay/run-handle APIs already used by the CLI.
- Existing package, process and terminal-prerequisite public APIs from the
  earlier protected harness publication remain required.

This preparation is based on actual protected CLI main. Before native
admission, root must review the docs/publication source boundary. A fresh
hosted run through installed B2 must prove that the pinned API commit equals
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

## Historical local pass

Root's normal local commit `395524dd45a1096e4949d0eba742db17d1aaa940`,
tree `5e2e3c61ff6150353bbddda656266c7cd5ab39f8`, passed on macOS on
2026-10-19 against the same protected harness input: 116 unit tests, 6
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
Run the normal `./scripts/check` on this exact protected-parent stack,
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
The selector uses Cargo's manifest path, binary target and test profile, including
fresh cache hits. It never builds another test binary or uses a relay. Run that gate
on Linux and macOS and retain separate Windows terminal evidence. Verify that
the admitted B2 context binds harness revision `220305fe4be7fecd33c3cf04b24f7a1b4b50503d`,
tree `518bd4ecfc28b31832465df083fe53df9cd053dd`, `pin_source: head`, the
exact native head and protected-main authority, plus the actual harness archive
digest and its 352 source files. The old base pin or a green wrapper without
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
