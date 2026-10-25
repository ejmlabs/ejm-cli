# Registry proxy guard verification

## Scope and ownership

This 2026-10-20 follow-up starts at
`7df81ba5610ae4168bf2c917531fbe054565d841`. The harness pin at verification time
was `4a751c6a3f24d7d143140f79ff53517fb93e3531`. After merging main, the pin is
`42e4f9cf33bd3525aec344cfdffabc251271f782`. Both lack loopback `no_proxy()` on
`RegistryClient::new`, so the CLI proxy guard remains required.

The pin lacks the direct HTTP loopback transport now present on the harness
registry branch. `Args::registry_choice` adds only a temporary configuration
refusal. It checks at most six environment variables for a validated HTTP
origin. Any nonempty value, including non-UTF8 bytes, rejects the choice.
`NO_PROXY` does not override the refusal. The error contains the variable name
and no value. No dependency, HTTP client, persistent state, or background task
was added.

`open_registry` checks the choice before `RegistryClient::new` and before
dispatch can load a saved token. Doctor uses the same choice. The pure origin
resolver, HTTPS selection, model commands, and local discard retain their
existing paths.

The harness owns transport and credential storage. The CLI consumes its public
packages. This adapter may be removed only after the CLI pins a compatible,
reviewed full commit from protected harness main and passes direct-route and
credential-storage checks. Keep the normal human admission and CI gates.

## Checks on Darwin arm64

- `rustfmt --edition 2024 crates/ejm-cli/src/args.rs crates/ejm-cli/tests/cli.rs`:
  passed, including a later check after the help edit.
- `git diff --check`: passed.
- `openspec validate move-registry-to-ejmlabs-domain --strict`: passed.
- The compiled Args driver rendered `--help`, `package --help`, and
  `doctor --help` successfully. Each names the proxy restriction and `NO_PROXY`
  rule. This is generated help from the real argument source, not a run of the
  complete CLI.
- The isolated actual Args-module check passed: nine existing Args unit tests
  and seven behavior checks, including 24 flag/environment/proxy/bypass
  combinations. Both small verification binaries compiled with `-D warnings`.
  The behavior checks also cover absent and empty values, whitespace and
  non-UTF8 values, HTTPS selection, flag precedence, and hidden proxy values.

The temporary driver includes the actual `args.rs`, copies the real
`PackageCommand` enum for Clap parsing, and stubs only the terminal appearance
enum. It links existing cached Anyhow, Clap, URL, harness, and execution
libraries without rebuilding them. Those cached harness libraries are not
proven to match the immutable pin. This check verifies the edited configuration
module only, not command dispatch or transport. It ran after the coordinated
measurement hold ended and held the shared Cargo coordination lock.

```sh
python3 /private/tmp/ejm-registry-args-check-20261020/build_and_check.py
```

The temporary script compiles the driver and its tests with `rustc`, runs the
nine actual Args tests, then uses Python's standard `unittest` library to check
isolated subprocess choices. It installs no package and makes no network call.
The five cached library inputs were:

- `libanyhow-80b7e18987d053dc.rlib`
- `libclap-83bc732a192936c2.rlib`
- `liburl-85dacb887322667e.rlib`
- `libejm_harness-28b0c984e91941b5.rlib`
- `libejm_harness_exec-6f482fa1cbe34bf6.rlib`

The new real-binary fixtures in `tests/cli.rs` later passed twice against the
exact pinned harness. They cover each
of six proxy variable forms, both registry-selection sources, a covering
`NO_PROXY`, preservation of a synthetic session, no connection to passive
registry/proxy listeners, no credential or proxy-value output, doctor reporting,
empty proxy values, HTTPS, replay, and local discard.

Their variable-name assertion ignores ASCII case because Windows environment
variable names are case-insensitive. This source check does not claim a Windows
run.

The exact old harness source was exported from the local Git object database
after disk space recovered. All 239 CLI and 258 pinned-harness exported blobs
matched their Git objects. The real proxy fixtures passed before and after the
assertion-only lint repair. A tiny Args-module check does not establish those
contracts by itself.

## Full-gate portability follow-up

The first exact-pin `./scripts/check` run stopped at strict Clippy because a
new equality assertion used `assert!`. Its repair reads the saved bytes into a
named buffer and calls `.eq(&original)`, retaining a failure message that does
not print either buffer.

The next run passed strict Clippy, all 62 unit tests, and 42 of 43 integration
tests. The existing other-loopback fixture failed before starting the CLI:
this Mac cannot bind the unconfigured `127.0.0.2` alias. The fixture now uses
`http://127.0.0.2:0` and keeps the nonzero exit, origin announcement, and precise
`registry requires HTTPS` checks. On the immutable pin that error is raised
before HTTP client construction, so this contract needs no listener. Port zero
cannot name a listening service. No host alias is added and no test is skipped.

Both repairs affect tests only. The public README, CLI guide, help and site
still describe the same product behavior and need no further edit for these
fixture repairs.

## Exact-pin full verification

The final run used CLI base `1b198c3b278d47b62fcc70ad951f56e30dab42ca`
plus the two test-only repairs above. The final `tests/cli.rs` SHA-256 is
`d3063dc7f277c7d8926c80dda2d4362ff5f885472f14846eaf6f0bc5da548de1`.
Every product source file stayed at that base. The sibling harness was exported
from immutable commit `4a751c6a3f24d7d143140f79ff53517fb93e3531`, tree
`e51a5a478c9a7cc1f93a3fb47aa865fee86f8ba2`. The exported
`.github/harness-revision` and both Cargo lockfiles stayed unchanged.

The sibling source layout was
`/private/tmp/ejm-audit-verification-20261020/cli-pinned/`, with `ejm-cli`
beside `ejm-harness`. Builds used Rust 1.99.1, Cargo jobs two,
`CARGO_INCREMENTAL=0`, `CARGO_NET_OFFLINE=true`, and the separate
`CARGO_TARGET_DIR=/private/tmp/ejm-audit-verification-20261020/target`.
Existing debug and release profiles were preserved. Every native run held
`fcntl` `LOCK_EX` on `/tmp/ejm-capacity-cargo.lock`. Credential variables were
omitted; tests used synthetic sessions and replay. No primary target was
cleaned or copied.

From that exported CLI directory, the commands and results were:

```sh
cargo test --offline --locked -p ejm-cli --test cli proxy -- --test-threads=1
cargo test --offline --locked -p ejm-cli --test cli \
  pinned_registry_client_still_rejects_other_loopback_addresses \
  -- --exact --test-threads=1
./scripts/check
```

- Proxy fixtures after the assertion repair: three passed, zero failed,
  40 filtered, 1.18 seconds. The final full gate reran them after the separate
  portability repair.
- Portable pinned-origin fixture on the final test delta: one passed,
  zero failed, 42 filtered, 1.02 seconds.
- Final `./scripts/check`: exit zero on Darwin arm64. Architecture, formatting,
  strict Clippy, all 62 unit tests and 43 integration tests, the configured
  release build, rustdoc with warnings denied, cargo-machete, cargo-deny,
  and real PTY interaction/restoration passed. No test was ignored.
- The final gate held the shared lock from `2026-10-20T15:58:20Z` through
  `2026-10-20T16:00:39Z`. It ended with about 27.6 GiB free, above the
  10 GiB stop threshold.

Raw logs, source archives, commit/tree provenance, archive hashes, exact fixture
patches and command events are under
`/private/tmp/ejm-audit-verification-20261020/logs-registry/`. The final logs are
`cli-pinned-proxy-patched.log`, `cli-pinned-other-loopback-portable.log`, and
`cli-pinned-full-gate-final.log`. The exported source and owned fixture matched
before the final checks. Evidence and task updates after the gate do not alter
that tested source.

## Remaining gates and documentation

Task 5.4 is complete for this Mac. Linux and Windows acceptance, compatible
harness adoption, human admission, spec archival, and release work remain open.
The fresh review clone has no installed Git hooks; no hook configuration or
bypass flag was used.

The README, CLI reference, help source, and changelog describe the temporary
restriction and recovery. The static public page has no registry origin or
proxy setting, so it needs no content change for this follow-up. Publication
was not verified. No pin, lockfile, workflow, tag, release, service, or deployment
was changed. The OpenSpec change remains active for full verification and the
separate pin-adoption work.
