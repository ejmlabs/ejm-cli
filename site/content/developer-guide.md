## Set up a checkout {#setup}

The CLI owns terminal interaction and consumes the public Rust harness API. Check out both repositories as siblings and select the recorded compatible harness source:

```sh Check out compatible source
git clone https://github.com/ejmlabs/ejm-harness.git
git clone https://github.com/ejmlabs/ejm-cli.git
cd ejm-cli
git -C ../ejm-harness checkout --detach "$(cat .github/harness-revision)"
```

Read [AGENTS.md](https://github.com/ejmlabs/ejm-cli/blob/main/AGENTS.md), the matching project skill and [tools.toml](https://github.com/ejmlabs/ejm-cli/blob/main/tools.toml). The native gate requires Rust 1.99.1 with Clippy/rustfmt, Python 3.11 or later, cargo-deny 0.20.2 and cargo-machete 0.9.2:

```sh Install pinned tools and hooks
rustup toolchain install 1.99.1 --profile minimal --component clippy,rustfmt
cargo install --locked --version 0.20.2 cargo-deny
cargo install --locked --version 0.9.2 cargo-machete
./scripts/install-hooks
cargo build --release --locked
./target/release/ejm-cli --replay
```

The rustup override selects the toolchain without changing your global default. Tool installation and advisory updates need network access. Keep Cargo jobs at most two; `.cargo/config.toml` sets the cap. The core binary needs no Node or Python runtime. Optional code packages use Node 24 or later or Python 3.11 or later.

Linux credential builds need pkg-config and libdbus development headers. The installed [setup action](https://github.com/ejmlabs/ejm-cli/blob/main/.github/actions/setup-ci/action.yml) records hosted prerequisites. Saved native credentials require the matching OS key service. Headless relay clients can explicitly use `--memory-only`; never add a plaintext fallback.

### Native Windows checks {#windows-setup}

Native Windows terminal behavior needs separate evidence. Linux and macOS gate results, WSL and cross-compilation do not prove it. The [native acceptance reference](/docs/reference/platforms/) records checked source and remaining platform/distribution gates.

## Find the owning module {#architecture}

The CLI owns commands, terminal lifecycle, bounded view state, host browser callbacks and native relay adapters. Harness crates own providers, authentication mechanics, execution policy, persistence and resource admission. Keep this dependency direction and use public interfaces.

### Package boundaries {#package-boundaries}

<!-- table: Project ownership -->

| Owner | Responsibility |
| --- | --- |
| `ejm-harness` | Shared agent/session state, providers and auth, tools, policy, persistence and admission. |
| `ejm-cli` | Terminal UX, CLI commands, bounded views, host browser/relay adapters and product guides. |
| `ejm-ide` | Editor interaction, workspace services and desktop/OS adapters. |
| `ejm-ui` | Shared tokens, visuals, site infrastructure, demos and capture tools. |
| `ejmlabs` | Project discovery, public pages and download assembly. |

Check existing public APIs before adding behavior. Products consume shared owners; shared runtime must not depend on products or frontend frameworks. Local Cargo paths and npm `file:` links are valid when they use public exports. Do not copy shared implementations or import private sibling source. Record any missing contract, its owner, checks and removal condition before adding a narrow adapter. Preserve concurrent work and report owner and affected-consumer checks.

### CLI modules {#cli-modules}

<!-- table: CLI module boundaries -->

| Module | Responsibility |
| --- | --- |
| `args` | Parse commands, launch flags and environment settings. |
| `auth`, `browser` | Host callback, browser lifetime and safe account presentation through harness auth. |
| `app` | Bounded terminal view state and durable-event application. |
| `ui`, `render`, `terminal`, `terminal_input` | Input, drawing, terminal modes and restoration. |
| `remote` | Native account transport, pairing, bounded protocol and session authority. |
| `extensions`, `packages` | Public package and export adaptation. |
| `main` | Compose TUI, plain text and JSON execution. |

Rendering cannot launch a subprocess or call a provider. Keep native grants separate from security confinement and preserve the fixed process admission limits.

## Make a focused change {#change}

1. Inspect the working tree and preserve other contributors' edits. Read the matching skill.
2. Use OpenSpec for feature and bugfix work. Name the owner, public interface, dependencies and affected consumers.
3. Implement the smallest change that preserves drafts, durable events, grants, Unicode boundaries and cancellation.
4. Add tests for meaningful behavior or a concrete regression. Use local replay or HTTP fixtures and no paid provider calls.
5. Review public docs, canonical references, examples, help and release notes together. Explain when a change has no public docs impact.

Use the [development reference](/docs/reference/contributing/) for strict lints, package choices and dependency policy. Fix lint causes rather than widening allowances. Native execution and enabled package code run as the user and are not a sandbox.

## Run the required checks {#checks}

```sh Run the native gate
./scripts/check
```

This gate checks architecture/skill routing, format, strict Clippy, deterministic tests, release binaries, Rust documentation, unused dependencies, licenses/advisories and real PTY interaction/restoration. It is the local, pre-commit and hosted entry point. The active source workflow runs it on Linux and macOS. Record the exact source, harness pin, lockfile, host and omitted checks; a local pass does not prove hosted acceptance or another platform.

Use `cargo test -p ejm-cli NAME` for a focused test. Replay needs no model account. `--replay-file` reads deterministic public `ModelResponse` fixtures; it is not a production model service. Preserve failed drafts, durable event order, cancellation and terminal restoration in tests.

## Native remote source {#remote}

The [acceptance record](/docs/reference/platforms/) binds the accepted native source to its compatible harness and Linux/macOS checks. Worker/browser adoption, a hosted service, binary distribution and live round trips have separate gates. The [remote reference](/docs/reference/remote-control/) owns exact setup, pairing, memory-only operation and recovery.

Before persistent relay use on each machine and origin, including a fresh install, stop older writers and run `remote migrate-credentials`. Confirm only after those writers stop. Preserve endpoint identities, namespace separation, retirement markers, destination reload before legacy cleanup and fail-closed key handling. A missing marked key cannot become a new silent identity.

Account admission does not grant session access. The host approves the full fingerprint and retains its tool grants and exact-run authority. Browser clients require trust in operator-delivered code. Do not fold unreviewed Worker/backend changes into a static site release or imply that publishing these guides activates a service.

## Preserve execution boundaries {#safety}

- Use the public harness auth API. Bind the callback before opening the browser. Never put bearer tokens in arguments, logs or terminal content.
- Remove untrusted terminal control sequences. Keep display and input bounded and restore terminal state after errors, cancellation and panic.
- Keep session, model and tool counts separate. A session does not reserve a local command slot.
- Preserve cancellation and exact selected-run authority during native sharing; an acknowledged prompt is not completed work.
- Review trusted package code and bundled dependencies before enabling it. Named export grants do not confine native code.

## Maintain the documentation site {#website}

### Review docs with every change {#docs-policy}

Update affected public guides, canonical references, README, examples and release notes in the same change. Cover setup, configuration, workflows, errors, limits and recovery when relevant. Distinguish implemented source, checked behavior and released services. A local build does not update the live site.

### Edit the owned sources {#docs-sources}

The Vite/React site prerenders routes to `site/dist`. That directory is ignored build output. Guides live in `site/content/`; `site/build/docs-pages.mjs` maps route metadata and canonical `docs/*.md` references. Keep detailed contracts in their existing canonical source. Shared `@ejm/site/docs` compiles Markdown and public `@ejm/ui/docs` renders it.

The public shared UI source is selected by `.github/ui-source.json`. Check out that exact commit beside the CLI. Package links use public `@ejm/*` exports. UI owns components, themes, headers, demos and prerendering; this site owns content and routes. Add shared patterns in their owner, never local CSS or visual overrides. `Static className="contents"` is the allowed structural boundary. Keep navigation, search, copy controls and playback interactive outside fixed prose. The first CSS import uses `source(none)`; public style profiles declare their own source sets.

### Check and preview affected pages {#docs-checks}

Use Node 24 for the shared site toolchain. Install the locked public UI source, then the site:

```sh Install and check the site
npm --prefix ../ejm-ui ci --ignore-scripts
npm --prefix site ci --ignore-scripts
npm --prefix site run check
```

The site gate runs types, build, generated headers and built-HTML tests. The build itself fails a page with more than one h1, a skipped heading level, a missing skip link or `main#main`, or an external link without `rel="external"`. Load time is the only performance bar: run the shared load-time check against its baseline and record cold-load FCP/LCP, CLS, TBT and no-script output at the actual viewport.

```sh Preview built routes
python3 -m http.server 4831 --directory site/dist --bind 127.0.0.1
```

Check phone and desktop widths in both themes, keyboard focus and skip navigation, contents, deep guide fragments with Back/Forward, copy controls, tables, overflow and console errors. Static prose and links must work without scripts.

The home contains one labelled local recording. The public shared player owns its frame, lazy loading and Pause/Resume/Restart controls. No model runs in the browser; the recording does not prove current native setup or hosted inference. Keep useful completed output available without scripts.

Publishing uses only the `ejmlabs-cli` Worker, its `ejmlabs-cli-staging` staging Worker and `site/wrangler.jsonc`. Each merge to main deploys staging; production waits for owner approval in the GitHub `production` environment. Keep `workers_dev` and `preview_urls` false, preserve zone protections and verify canonical routes, assets, headers and provider settings on staging after each merge and on production after approval. Read [hosting and recovery](/docs/reference/contributing/#cloudflare-site-hosting).

## Contribute and release {#review}

Read [Contributing](https://github.com/ejmlabs/ejm-cli/blob/main/CONTRIBUTING.md) and the [Code of Conduct](https://github.com/ejmlabs/ejm-cli/blob/main/CODE_OF_CONDUCT.md). A current human maintainer posts `/ready FULL_HEAD_SHA`, then `/approve FULL_HEAD_SHA` after review. New commits need fresh decisions. Required statuses are Human readiness, Maintainer approval, Quality, Repository standards and AI review. Keep the comments unchanged; use the documented revoke/retry process and never bypass protection.

The workflow reads the UI pin from trusted main, so merge its reviewed adoption PR before the site PR uses new UI contracts. A pin adoption pass uses the old dependency; a fresh admitted site PR must exercise the installed pin. Harness selection has its own current rules in [CI setup](/docs/reference/relay-ci/).

Static site publication does not publish native installers, packages or a source version. Follow the separate [release policy](https://github.com/ejmlabs/ejm-cli/blob/main/RELEASING.md) for version metadata, tags and maintainer-triggered source drafts. Record final source and deployment identities and verify the live site before claiming publication.

## Keep the source references close {#reference}

- [CLI contract](/docs/reference/cli/): launch flags, providers, commands, packages and bounds.
- [Native remote guide](/docs/reference/remote-control/): transport, credentials, pairing and recovery.
- [Development reference](/docs/reference/contributing/) and [CI source setup](/docs/reference/relay-ci/): tools, ownership and required checks.
- [Native acceptance](/docs/reference/platforms/) and [benchmarks](/docs/reference/benchmarks/): source identities, evidence and limits.
- [Support](https://github.com/ejmlabs/ejm-cli/blob/main/SUPPORT.md) and [Security](https://github.com/ejmlabs/ejm-cli/blob/main/SECURITY.md): reporting without secrets or private state.
