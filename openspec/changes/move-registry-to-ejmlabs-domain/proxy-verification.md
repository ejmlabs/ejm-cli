# Registry proxy guard verification

## Scope and ownership

This 2026-10-20 follow-up starts at
`7df81ba5610ae4168bf2c917531fbe054565d841`. The immutable harness pin remains
`4a751c6a3f24d7d143140f79ff53517fb93e3531`.

That pin lacks the direct HTTP loopback transport now present on the harness
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

The new real-binary fixtures in `tests/cli.rs` are not yet run. They cover each
of six proxy variable forms, both registry-selection sources, a covering
`NO_PROXY`, preservation of a synthetic session, no connection to passive
registry/proxy listeners, no credential or proxy-value output, doctor reporting,
empty proxy values, HTTPS, replay, and local discard.

Their variable-name assertion ignores ASCII case because Windows environment
variable names are case-insensitive. This source check does not claim a Windows
run.

The exact old harness source exists in the local Git object database, but an
old-pin CLI rebuild was deferred to preserve constrained disk space. A tiny
Args-module check cannot establish full CLI dispatch, credential, socket,
platform, or immutable-pin compatibility.

## Unrun gates and documentation

`cargo test -p ejm-cli`, the new real-binary fixtures, `./scripts/check`, full
Clippy, release/PTY/documentation builds, and Linux/Windows acceptance remain
unrun for this follow-up. Keep task 5.4 open until the exact pinned build passes
the required checks. The fresh review clone has no installed Git hooks; no
hook configuration or bypass flag was used.

The README, CLI reference, help source, and changelog describe the temporary
restriction and recovery. The static public page has no registry origin or
proxy setting, so it needs no content change for this follow-up. Publication
was not verified. No pin, lockfile, workflow, tag, release, service, or deployment
was changed. The OpenSpec change remains active for full verification and the
separate pin-adoption work.
