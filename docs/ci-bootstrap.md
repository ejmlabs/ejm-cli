# Relay publication prelude and CI setup

## Published prelude and prepared caller adoption

The combined CI bootstrap, terminal input boundary and command foundation
merged through CLI PR #2 as protected main commit
`8a603af45504579ed1234bfe92a34b33367e44bd`. Its tree
`cc2bc0f0330fb8464cca4060886fc2f1e4be9297` equals the reviewed prelude head
`c3fddcb5bfd2b6b9485e91d4eaa2ec682ade37ee`. It includes the local setup action
needed by the next central hook. This is not an available relay or a claim of
measured CI savings.

This bounded prerequisite prepares both canonical caller `uses` pins and the
release `standards-ref` together for reviewed central PR #11 implementation
`ae903129564926a14287f75d2012610c59cccb65`. It preserves protected central
main `d8b31f1debf095b104f6de76193bf531ec4ddd2c`, including its concurrent
docs change. It keeps the CLI hook and adds fixed Linux D-Bus build setup for
the harness quality profile. The earlier `5309137` and `0adffdf5` targets are
obsolete history.

Central PR #11 passed its required checks and exact-head human approval, then
merged through protected `main` as
`8c304c8cdfc8b6fe50f3e442507215e615944f2b`. The merge and reviewed head share
tree `91b7f5c0990e27f2e0fdfcce26aac18afdcf6b81`. The exact reviewed
implementation is retained at
`refs/heads/workflow-pins/ae903129564926a14287f75d2012610c59cccb65` under
active update/delete rule `25369476`, with no bypass actors. Protected main
rules are unchanged. This completes the central prerequisite, not CLI caller
adoption or its hosted checks. The previous caller revision is `5fae851502c2e3c31d4df9a44f8d56c2cb49e098`. Keep the
working harness pin unchanged at
`4a751c6a3f24d7d143140f79ff53517fb93e3531`.

The [deployment change](../openspec/changes/deploy-private-remote-relay/proposal.md)
tracks the full release. All its tasks remain open in this source preparation.
The caller update adds no product code or new harness API consumer. The action,
locks, provider composition, account storage and CLI arguments match protected
main. See the [prelude scope](command-foundation-publication.md) and
[terminal boundary](terminal-input.md) for the earlier source layers.

## What the action prepares

The composite action at `.github/actions/setup-ci/action.yml` uses the pinned
`actions/setup-node` revision and installs Node 26.12.0, npm 11.20.0, the Rust
1.99.1 WebAssembly target and wasm-bindgen CLI 0.2.129. Cargo installation uses
two jobs. Linux setup installs `pkg-config` and `libdbus-1-dev` for the later
protected native credential service.

The admitted workflow exports the CLI source under `product/`. When
`product/relay/package.json` exists, the action runs `npm ci --ignore-scripts`
and the locked Playwright command `npm exec --no -- playwright install
--with-deps chrome` from the relay directory. Before relay source lands, that
one step is skipped. A relay manifest without its lockfile fails installation.
Tool installation needs network access and the hosted runner's package manager.

The prior `5fae851502c2e3c31d4df9a44f8d56c2cb49e098` caller does not invoke
this action. The retained `ae903129564926a14287f75d2012610c59cccb65` caller
adds its exact local-action step only for the admitted `ejm-cli` profile in the
credential-free quality job, after Rust setup and before `./scripts/check`.
The source release continues to use the same central release workflow, with
its `uses` and `standards-ref` pointing to the same revision. Product permissions,
triggers, secret forwarding and human controls are unchanged.

The CLI gate at this cutoff uses Rust and Python and keeps the working harness
pin. It does not yet contain the native API consumers or Worker/browser source.
The hook installs Linux D-Bus build prerequisites before the later API pin
adoption; the browser install step skips only when the relay manifest is absent.
A manifest without its lock still fails.

`./scripts/check` remains the product quality gate. The later relay source change
must add its locked dependencies, tests and gate integration together. Run and
review a fresh clean-checkout gate for each proposed source snapshot.

## Protected adoption order

1. Confirm the reviewed local action is on trusted CLI `main`. The prelude
   receipt above records its protected merge; retain its exact action, working
   harness pin and lock through this prerequisite.
2. Verify the central receipt above against the reviewed implementation at
   all three pin sites. Its protected merge and immutable retention are
   complete; keep the exact retained `ae903129` implementation. If a later
   reviewed head changes, refresh all three CLI pin sites together and repeat
   the required checks and retention review.
3. Publish, admit, review and merge the paired PR/release caller update, with
   the release `standards-ref` at the same revision and the working harness pin
   unchanged. Obtain exact-source admitted Linux and macOS quality results
   before claiming the setup hook works for the CLI.
4. Complete protected review of the required new private harness APIs. After the
   hook provides their Linux native prerequisites, adopt the full reviewed API
   commit in `.github/harness-revision` through its own protected change. Retain
   the private credential scope and expiry requirements below.
5. Publish native remote API consumers with a lockfile resolved for the new
   trusted harness pin, then the bounded Worker/browser source. Run hosted Linux
   and macOS quality checks for each exact admitted head. Retain the protected
   merge and release checks.

Readiness may be posted under the maintainer's explicit delegation. Approval
of each exact head requires its separate explicit human answer. Admission is
tied to a non-draft PR targeting `main`; each new head needs fresh decisions. The quality workflow reads
`.github/harness-revision` from trusted base `main`, not from the proposed PR.
A passing pin-only PR therefore does not prove the new API consumer pair. Its
later API-consuming source must pass a fresh locked gate with that trusted pin.

The prepare job exports `product.tar` under `product/` and the selected private
harness under `ejm-harness/`, preserving the workspace's `../ejm-harness` paths.
The quality job receives source archives and has no provider credentials or
write token. A private dependency checkout needs a dedicated, expiring Contents:
read credential for `ejmlabs/ejm-harness` in `EJM_HARNESS_TOKEN`. Successful source
preparation does not verify the credential's scope or expiry. Validate those
separately through protected secret handling. Keep broad workstation credentials
local.

The current central review bounds are 200 changed files, 12,000 added plus
deleted lines, and 1,000,000 diff bytes per PR. Measure this caller prerequisite
against protected main and remeasure every later committed diff.
Keep generated build output, dependency caches, private environment files and
the separate site migration out of these source changes.

## Verification and publication record

The initial caller preparation checked canonical pins, the exact retained
central validator, actionlint, OpenSpec, documentation links and whitespace.
The normal commit hook then passed for CLI head
`92e423b67243b15e97a43c5ba4776057172d242c`. Its two admitted AI reports
explicitly said review was incomplete because the old and new external workflow
implementations were absent. Successful wrapper jobs and zero findings did not
complete that review or establish acceptance.

The [shared workflow source evidence](ci/shared-workflow-review/README.md)
supplies deduplicated exact plaintext copies of both implementations and their
readable source, with full revisions, original paths, Git blob IDs and SHA-256
receipts. These files are review data and add no execution authority. The
caller pins, runtime, tools, locks and working harness pin stay unchanged.

This correction requires the normal hook and fresh admitted Linux/macOS quality
checks and complete reviews for its new head. It establishes no new full-gate,
PTY, hosted acceptance or public-site result by itself. Exact-head human approval
and protected adoption remain required. This contributor CI guidance changes no
product behavior; the relay source layer owns its public setup and recovery
docs and their separate site publication.
