# Contributing to ejm-cli

ejm-cli is a separate Rust project that consumes the public ejm-harness crates. Check out `ejm-cli` and `ejm-harness` as siblings. The CLI must build using only its declared public dependencies. Keep unrelated proprietary products out of its source and examples.

## Tools and the required gate

The versions in `tools.toml` are required. Use Rust 1.99.1 with Clippy and rustfmt, Python 3.11 or later, cargo-deny 0.20.2 and cargo-machete 0.9.2.

```sh
rustup toolchain install 1.99.1 --profile minimal --component clippy,rustfmt
cargo install --locked --version 0.20.2 cargo-deny
cargo install --locked --version 0.9.2 cargo-machete
./scripts/install-hooks
./scripts/check
```

The repository's `rust-toolchain.toml` selects Rust 1.99.1 when using rustup, without changing your global default. A Homebrew installation of Rust 1.99.1 also works. The gate checks the compiler's reported version. Tool installation and advisory database updates need network access. `Cargo.lock` is committed and every build in the gate uses `--locked`.

The complete gate validates architecture and skill routing, checks format, runs strict Clippy, runs deterministic tests, builds release binaries, checks Rust documentation, finds unused dependencies, checks licenses and advisories, and runs real PTY interaction/restoration tests. Run it before reporting a change complete or preparing a release; admitted CI also runs it. The faster pre-commit hook checks staged whitespace and working-tree architecture and Rust formatting, and does not replace the full gate. Build jobs are capped at two in `.cargo/config.toml`. `CARGO_TARGET_DIR` may share a local compilation cache.

The hook installer changes only this repository's `core.hooksPath`. It does not install a global hook. The quality gate runs on Linux and macOS. The trusted `.github/workflows/pr.yml` and `.github/workflows/release.yml` entry points read the reviewed full 40-character sibling harness commit from `.github/harness-revision`.

The workflows reject a missing or mutable harness revision. The installed selector reads the pin from the exact admitted CLI commit and verifies that it equals or precedes protected main in the fixed official harness repository. Preparation binds the checked-out commit, tree and source archive digest into the admitted context. The [readable dependency review prerequisite](ci-bootstrap.md#readable-dependency-review-prerequisite) records protected PR #9's 40d callers and source context. Its Linux Secret Service setup and unchanged baseline gate passed on Linux and macOS with the working 4a harness. That run used installed b2; it does not prove the later native vault gate or installed 40d readable harness extraction. Keep those exact-source checks separate.

The harness and shared UI repositories are public and need no special read credential. See [CONTRIBUTING.md](../CONTRIBUTING.md) for human admission and approval, and [RELEASING.md](../RELEASING.md) for remote checks and publication gates. The PTY gate checks interaction and terminal restoration on its host. Local macOS results do not prove Linux behavior. Windows terminal behavior requires separate evidence.

For the prepared CI action, source scope, and protected adoption order, see [CI setup and protected adoption](ci-bootstrap.md). It retains the current reviewed harness pin. Source preparation is separate from hosted product verification.

## Ownership and Rust design rules

| Module | Owns |
| --- | --- |
| `args` | Command parsing, environment selection and effective configuration |
| `auth` | Host browser launch, bounded loopback callback handling and safe account presentation through `ejm-harness-auth` |
| `browser` | Cancellable, timed OS URL launcher supervision for explicit sign-in |
| `app` | Bounded view state and durable-event application |
| `ui` | Input dispatch, harness calls and terminal lifecycle |
| `terminal` | Native mode capture/restoration and key-repeat policy |
| `terminal_input` | Owned bounded input parser, event adaptation and reset |
| `render` | Drawing and text layout |
| `main` | Process composition and plain/JSON entry points |
| `remote` | Native account transport, Noise pairing, bounded protocol, shared session authority and vault adapter |
| Sibling ejm-harness | Shared authentication and credential storage, providers, run policy, context, tools, persistence and resource admission |

Keep each piece responsible for one coherent job. Extend real variable seams through the public harness traits. A provider or tool implementation must honor its contract, including cancellation, budgets and error reporting. Prefer small capability interfaces over a large “service” object. Compose the concrete app in `main`; pass the harness into the UI. Do not introduce a dependency injection container, factory hierarchy or trait for a single fixed renderer.

Use trusted, maintained packages for established infrastructure. Protocol parsing, cryptography and other standard concerns should use a reviewed package when one meets the requirements. Custom code should provide ejm-cli's distinct behavior, including its TUI interaction and app state. If a package cannot meet a verified requirement, document that gap before replacing it.

The CLI uses Hyper for loopback callback HTTP, `url` for URI validation and `open` for OS browser handlers. Alacritty's VTE parser removes control sequences from displayed output; keep its default features disabled so its OSC buffer stays fixed at 1 KiB. Termwiz owns terminal input parsing, with thin native mode and event adapters. See [Terminal input boundary](terminal-input.md) for limits, package choices and the candidate's open verification gates. Browser completion, cancellation and plain-text display policy remain CLI responsibilities.

These are Rust applications of single responsibility, substitutability, interface segregation and dependency inversion. Ownership, enums and `Result` should make illegal states and failure paths clear. A new abstraction needs a current second use or a documented variable boundary. Rendering must never launch a subprocess or contact a model provider.

## Strict lint policy

Rust warnings, Clippy `all` and Clippy `pedantic` are denied. Production code forbids unsafe code and denies `unwrap`, `expect`, explicit panic, unreachable markers, unfinished code markers, debug macros, process exits and forgotten allocations. Normal CLI stdout/stderr remains part of the product interface.

Fix the cause of a lint. Do not add workspace-wide allowances. An item-level `#[expect(..., reason = "...")]` is permitted only for a precise, reviewed exception. Unfulfilled expectations fail the build. Current intentional exceptions are independent boolean command switches in `Args`, and fixture `unwrap` calls within test-only modules. Neither exception applies to production state transitions or fallible setup.

The dependency checker denies unknown registries, unknown Git sources, wildcard versions, unapproved licenses, advisories and new duplicate-version families. `deny.toml` records exact-version exceptions for incompatible upstream requirements. Recheck those exceptions when updating the related dependencies; do not widen them to a whole crate family.

## Tests and review

For the separate main and daily Rust reliability workflow, see [Rust test reliability](flaky-tests.md).

Use replay or local HTTP fixtures. No paid provider calls belong in tests. Prove a changed user action or failure condition with a focused test before running the complete gate. Preserve failed drafts, durable event order, explicit grants, Unicode boundaries, bounded scrollback and prompt cancellation. Run the PTY check after terminal changes; a render snapshot alone cannot prove raw-mode restoration.

Record measured latency and memory with hardware, compiler profile and workload. The target is 100 managed agents with hosted models on four cores and 8 GB RAM. Do not claim that target from a single current-machine fixture.

## Native remote publication

The [native source acceptance record](native-remote-publication.md) binds
protected CLI PR #21's merge, tree and Linux/macOS results after PR #20. It preserves
C8's 40d source context, Linux setup and PR #6's consolidated publication guide,
plus the prerequisite's protected UI pin and install context.
The selected harness pin is actual protected completion API merge
`42e4f9cf33bd3525aec344cfdffabc251271f782`: 358 tracked files from tree
`826e02310f1097e13806ca5ca1bf5cc00d71e966`, with the exact native lock.
Retain the unchanged full gate for each following source before hosted review
and protected adoption. The d8 local pass remains earlier evidence, separate
from PR #21's exact-source acceptance. The following
[inert UI/Relay prerequisite](ci-bootstrap.md#current-inert-shared-ui-and-relay-inputs)
advances static review inputs; Worker/UI and deployment acceptance remain separate.

The native queue correction consumes the harness owner's exact current-run
completion subscription. It keeps one completion receiver for the revocable
remote target, polls completion after local input, discards an atomically reported
local-successor handoff and rechecks the share and current
run before admitting a prompt. This reuses the existing watch result, without a
new task, channel, queue engine or dependency. The backend contract covers a real
internal queued target, two remote successors and revocation during completion.
The shared text limit is 16 KiB of UTF-8 bytes, checked before acknowledgement.
Focused backend, recovery and corrected d8 SIGTERM checks are recorded separately.
The [correction record](native-remote-publication.md#prepared-completion-and-prompt-corrections)
retains the earlier scope; PR #21's acceptance covers the final native source.
Following changes still need their own full and hosted gates.

The normal gate drives the native private-terminal fixture through
`scripts/check_terminal.py`; all eight fixture cases and the real-command
SIGTERM case, memory-only host startup and four blocked-observer loop cases must
pass before claiming native terminal acceptance. The gate
selects the exact unit-test executable from
compiler-artifact JSON emitted by that same locked Cargo test invocation,
including cache hits. Missing or ambiguous artifacts fail; timestamps in a
shared directory do not select a binary. `check_terminal.py --native-fixture
CARGO_TEST_MESSAGES_JSONL` exposes the same validated path to other supervisors.
The supervisor never builds or calls a model provider. Worker/browser source
and Workerd/native round-trip fixtures belong to the following publication layer.
This native slice adds no unconditional Node, browser or Worker gate.

## Public docs maintenance

Review public docs with every change. Update affected setup, configuration, workflows, APIs, examples, errors, limits and recovery in the same change. Keep implemented source, test evidence and released services distinct. Explain when there is no docs impact. Include docs in normal release review and verify publication before claiming the live site changed.

Guides live in `site/content/`. `site/build/docs-pages.mjs` maps route metadata and canonical `docs/*.md` references. Keep detailed contracts in those references. Vite and the public `@ejm/site`, `@ejm/ssg` and `@ejm/ui` packages build every route into ignored `site/dist`; never commit or hand edit that output.

Check out the exact public UI revision in `.github/ui-source.json` beside the CLI, install its root lockfile, then run:

```sh
npm --prefix ../ejm-ui ci --ignore-scripts
npm --prefix site ci --ignore-scripts
npm --prefix site run check
```

Use Node 24 for site work. The gate checks types, build, generated headers and built HTML. Keep gzip route budgets at 95 KiB JavaScript, 14 KiB CSS, 70 KiB fonts and 30 KiB HTML. Also run the shared activated transfer/performance gate and check actual phone/desktop dimensions, both themes, keyboard focus, contents, deep fragments with history, copy controls, overflow and no-script content.

Sites consume public shared exports. Shared packages own tokens, layouts, controls, theme behavior, assets, analytics, headers, demos and prerendering. Do not copy shared implementations, add local CSS or pass visual overrides. Keep fixed prose in `Static` and required interaction outside it. The home recording is a labelled local example; no model runs in the browser and it does not prove native setup or hosted inference.

The trusted workflow reads `.github/ui-source.json` from protected main. Merge a reviewed pin adoption PR before a site PR uses the new API. Its adoption run uses the old pin; a fresh admitted site PR must verify the installed revision. Keep hosted governance pins, legal metadata, runtime dependencies and security overrides intact.

### Cloudflare site hosting

The `ejmlabs-cli` Worker serves `site/dist` at `https://cli.ejmlabs.com` through `site/wrangler.jsonc`. Its `env.staging` block serves the same build as the `ejmlabs-cli-staging` Worker at `https://cli-staging.ejmlabs.com`, which is publicly reachable and serves the latest main.

A push to main runs `.github/workflows/deploy.yml`, the canonical caller of the shared `ejmlabs/.github` workflow `deploy-site.yml`. A build job with `contents: read` and no secrets builds the site from that main commit, with `ejm-ui` checked out at the `.github/ui-source.json` pin. A staging job then runs `wrangler deploy --env staging` automatically. A production job deploys the same built files with the top-level config only after the required reviewer (@everettmorgan) approves the run in the GitHub `production` environment. Deploy jobs run Wrangler 4.151.3 pinned in the shared repository, not this repository's dependencies, with the `CLOUDFLARE_API_TOKEN` secret of the `staging` and `production` GitHub environments.

Approving the production job is the owner's deploy authorization. A manual `wrangler deploy` from a workstation is no longer the procedure. A newer merge cancels an older production deploy that is still waiting. Local dry runs are still fine. `site/package.json` pins Wrangler. Before merging, run the site gate and a dry run with that Wrangler from `site/`. `--env ""` names production, and `--env staging` checks staging:

```sh
npx wrangler deploy --config wrangler.jsonc --env "" --dry-run
npx wrangler deploy --config wrangler.jsonc --env staging --dry-run
```

Keep `workers_dev` and `preview_urls` explicitly false and preserve custom-domain zone protections. Run these checks on staging after each merge, and on production after approval: verify the provider mapping and both flags, canonical home/docs/reference routes, a missing route, security/cache headers and activated playback assets. Bind the deployed bytes and Worker version to the product commit, UI commit and lockfiles. Check that the former alternate hostname does not serve the site. A request from one network does not prove worldwide reachability.

For recovery, redeploy an earlier verified Worker version in Cloudflare, or approve an earlier run's production job while it is still available. Retain both disabled alternate origins and verify the restored routes and headers. Site publication is separate from native binary, package or source-version release. Follow [RELEASING.md](../RELEASING.md) for those gates.
## Release binaries

`.github/workflows/build.yml` builds `ejm-cli` with `cargo build --release --locked` at the harness revision in `.github/harness-revision`. Its jobs have `contents: read` and no secrets. `scripts/package_release.py` archives the binary with `LICENSE`, `THIRD_PARTY_NOTICES.md` and `HARNESS_REVISION`, runs `--version` and `--replay run "Hello"` against the unpacked archive, and prints the archive digest. A `collect` job uploads the one `release-files` artifact, with a combined `build-sha256.txt`, for the shared native release workflow, which holds the only write permission. `build.yml` runs on main and on dispatch. It has no macOS job because macOS Apple silicon is not offered until it is notarized. The build runner's glibc sets the Linux minimum (2.39 on `ubuntu-24.04`) and `libdbus-1` is a runtime requirement. Release notes live in `docs/release-notes/`.

## Project skills

The skills in `.agents/skills` route implementation, review, terminal and theme work, sign-in, packages, performance, testing, dependencies, the sibling harness, docs and claims, the marketing site, and releases. The six `openspec-*` skills handle exploration, proposals, plan updates, implementation, spec sync and archival. All feature and bugfix work follows OpenSpec because this project has an `openspec/` directory. `AGENTS.md` lists which skill fits each task. The gate checks each skill's frontmatter and references, that the exact set matches `scripts/check_architecture.py`, and that `AGENTS.md` routes to every skill. Keep them concise and specific to this project. Read only the skill that matches the task, and update the owning reference when a convention changes. When you add a skill, add it to the check script and to the table in `AGENTS.md`.
