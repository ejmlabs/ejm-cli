# Relay publication prelude and CI setup

## Prepared source and current status

This source combines the reviewed CI bootstrap, terminal input boundary and
command foundation into one bounded publication prelude. Combining them reduces
the number of source layers to review and admit. It does not establish a lower
measured CI cost, a passing hosted run or an available relay.

The source starts at the reviewed CLI standards tree
`35dda43fcf94881971f7bddf3a0174ee7f18962a`. Standards PR #1 merged as
`4b263d4cb2a50a3641d566d6c18fdf7a0551b759` with the same tree. The final
publication must use that merged base and record its exact source receipt. The
canonical PR and release callers retain their immutable central pin
`5fae851502c2e3c31d4df9a44f8d56c2cb49e098`. The harness pin remains the current
reviewed source, `4a751c6a3f24d7d143140f79ff53517fb93e3531`.

The [deployment change](../openspec/changes/deploy-private-remote-relay/proposal.md)
tracks the full release. All its tasks remain open in this source preparation.
The prelude adds no remote client, Worker, browser controller or new harness API
consumer. Provider composition, account storage and CLI arguments retain their
baseline behavior. See the [source scope](command-foundation-publication.md) and
[terminal boundary](terminal-input.md) for their contracts and open checks.

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

The retained central caller does not invoke this action. The prelude's current
gate uses Rust and Python and keeps the reviewed harness pin, so its terminal
and command changes do not need the later harness's Linux D-Bus dependency or
the browser tools. The prepared action supports the later source layers once
the central hook is reviewed and adopted.

`./scripts/check` remains the product quality gate. The later relay source change
must add its locked dependencies, tests and gate integration together. Run and
review a fresh clean-checkout gate for each proposed source snapshot.

## Protected adoption order

1. Admit, review and merge this combined prelude through the restored current
   caller, retaining the current reviewed harness pin and both canonical caller
   files. Resolve only the terminal dependency additions, review that lockfile
   and pass the full gate for the exact prelude source before publication.
2. Once the reviewed local action is on trusted CLI `main`, complete protected
   review and adoption of the central setup hook while retaining that working
   harness dependency. Update the canonical PR and release callers together to
   the reviewed immutable central revision through a separate admitted change.
3. Complete protected review of the required new private harness APIs. After the
   hook provides their Linux native prerequisites, adopt the full reviewed API
   commit in `.github/harness-revision` through its own protected change. Retain
   the private credential scope and expiry requirements below.
4. Publish native remote API consumers with a lockfile resolved for the new
   trusted harness pin, then the bounded Worker/browser source. Run hosted Linux
   and macOS quality checks for each exact admitted head. Retain the protected
   merge and release checks.

A human maintainer must provide the existing readiness and approval decisions
for each final head. Admission is tied to a non-draft PR targeting `main`; each
new head needs fresh decisions. The quality workflow reads
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
deleted lines, and 1,000,000 diff bytes per PR. Measure this prelude again after
its terminal-only lockfile is resolved, and measure every later committed diff.
Keep generated build output, dependency caches, private environment files and
the separate site migration out of these source changes.

## Verification and publication record

The prelude needs standards and action review, OpenSpec validation, documentation
checks, the complete unchanged product gate and exact-source hosted checks. No
Cargo, PTY, hosted or public-site result is established by this source
preparation. Earlier checks of larger development trees do not verify it. The
relay source change owns its public setup and recovery docs, which must be
built and published through the existing site process.
