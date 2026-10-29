---
name: ejm-cli-site
description: Change, preview or deploy the cli.ejmlabs.com site under site/, a Vite and React project on ejm-ui, including its Markdown docs, budgets, headers, copy and Cloudflare Worker.
---

# ejm-cli product and documentation site

Apply the [minimal-copy rule](../../../AGENTS.md):
prefer a clear visual or no added content, and keep only the words needed
for meaning, access, decisions and recovery.

Read `site/package.json`, `site/vite.config.ts`, `site/build/docs-pages.mjs`, `site/src/site.tsx`, `site/src/pages/home.server.tsx`, `site/wrangler.jsonc` and [docs maintenance](../../../docs/contributing.md#public-docs-maintenance). The site is the public page for "cli by ejmlabs", while the program, crate and binary are `ejm-cli`.

- The site is built on ejm-ui (`@ejm/ui`, `@ejm/ssg`, `@ejm/site`, `@ejm/tokens`, `@ejm/demos`), linked from the sibling checkout `../ejm-ui` with `file:` dependencies through public exports. Select the immutable UI revision in `.github/ui-source.json`, then run `npm ci --ignore-scripts` in `ejm-ui` and in `site/`. Coordinate shared changes with its owner within the authorized scope and follow its rules. Brand files, the theme cookie and script, analytics, `_headers` and the head tags come from `@ejm/site` and `@ejm/tokens`. Do not copy them, hand edit `_headers` or add page CSS.
- `site/dist` is build output (`npm run build`), which `site/wrangler.jsonc` serves. It is ignored by git. Every route is prerendered. Use the shared `Static` boundary from `@ejm/ssg` for fixed prose. Keep shared navigation, the sidebar filter, contents list, code tabs, code-copy controls and playback live. Use shared route-aware links so a route with static prose loads its built document.
- Budgets (gzip: JavaScript 95 KiB, CSS 14 KiB, HTML 30 KiB, fonts 70 KiB) fail the build. Never raise them: trim, or load code later. Import the public `@ejm/ui/styles-site.css` with `source(none)`, then docs and product preview style profiles in `src/styles.css`. Shared packages own their Tailwind sources. Do not scan or exclude private sibling source paths.
- Shared components own every visual choice. Use the public semantic layouts, structured header and footer identity, fixed `Hero` entrance, and `ShowcaseFrame` for player size and disclosure. The shared build check rejects local CSS, inline styles, visual `className` values and unsupported overrides. Only `Static className="contents"` is allowed for static structure. Add a missing pattern in ejm-ui, then rebuild each site.
- Guides are Markdown in `site/content/`; references are `docs/*.md`. The Node-only `@ejm/site/docs` compiler supports `{#id}` headings, code-fence labels, `> [!NOTE]`, table captions and code tabs. Shared `@ejm/ui/docs` renders the compiled model. Keep only page metadata and repository link mapping in `build/docs-pages.mjs` and `build/docs-plugin.mjs`; add new pages to that list.
- Copy states only facts the repo can prove: [the CLI reference](../../../docs/ejm-cli.md) and [benchmark evidence](../../../docs/benchmarks.md). Use `$ejm-cli-docs` for claims. Describe ejm-cli as open source under MIT, use technical headings, and link source, issues and contribution guidance. Keep ejmlabs open to projects beyond these tools. Avoid sales slogans, adoption claims and first-person plural pronouns. Do not show an install command or download that `$ejm-cli-release` says does not exist. Keep public source links direct. The product and shared UI repositories are public; compatible source revisions do not establish installer availability or CI adoption. The recorded demo is labelled so.
- Review `/docs/`, `/docs/user-guide/`, `/docs/developer-guide/` and the reference pages for every change and update the Markdown and the canonical references together. State planned, implemented, tested and released status and relevant verification, availability, security and performance limits. Record the reason when there is no public docs impact.

Run `npm run check` in `site/`: typecheck, build with the budget gate, `ejm-site-headers dist/_headers --check`, and the tests, which parse every built page without running a script (one h1, skip link, `main#main`, toggle last in the header, head tags, external links, local links and anchors, commands and flags exist, banned wording, no em dashes). Also run `node ../../ejm-ui/scripts/perf.mjs dist` from `site/` for transfer sizes, LCP and CLS.

Preview with `python3 -m http.server 4831 --directory dist --bind 127.0.0.1` from `site/`, or `npm run dev`. Check desktop and a 375 px phone in light and dark with `node ../../ejm-ui/scripts/shoot.mjs URL --theme light` (zero overflow, zero small tap targets), keyboard focus, the skip link and an empty console. Static text and links must work without scripts.

Deploy only when the user asks to publish. Production goes live at once on `cli.ejmlabs.com` through the Worker `ejmlabs-cli`: `wrangler deploy --config site/wrangler.jsonc` after `npm run build`. Use only the configured `ejmlabs-cli` Worker. Keep other products, sites, assets and service configuration outside this project. Afterward, fetch the live page, check status and security headers with `curl -sI`, and look at it in a browser.

## Package boundaries

Read the [root ownership rules](../../../AGENTS.md) before this workflow. Check placement, dependency direction and existing public APIs/exports. Put shared behavior in its owner; do not add private sibling imports, copied shared implementations, consuming-product dependencies in shared packages or cycles. Keep necessary host adapters narrow and document any missing contract, contract checks and migration path.

Record the owner, public interface and affected consumers in plans and review. Check changed-owner behavior and affected consumer contracts and report omissions. Read each affected repo's rules and coordinate files within the authorized scope. Keep CLI copy and page composition here; tokens, reusable components, site infrastructure, builds, demos and captures belong in ejm-ui.

## Small footprint and fast execution

Small footprint and fast execution are the primary engineering goals, within correctness, security, and required behavior. Minimize memory, CPU, startup time, latency, binary or bundle size, I/O, and dependency cost. Choose the smallest suitable design and the fastest measured option for the real workload. Avoid needless services, background work, allocations, copies, and abstractions. Define budgets for affected paths and measure representative workloads before claiming an improvement. Document trade-offs when speed and footprint conflict, and justify any regression. Apply these goals to package choice, tests, and cleanup.

## Package reuse

Do not handroll functionality when a trusted, maintained package already meets the functional, security, footprint, and performance requirements. Check the standard library, platform APIs, and existing dependencies first, then evaluate established packages for maintenance, security, license, compatibility, and measured resource cost. Use the smallest suitable option and keep custom code to product-specific behavior and thin adapters. If none fits, document the alternatives and unmet requirement with evidence before writing custom code. A measured footprint or speed constraint can rule out a package; preference or assumption cannot. Apply this rule to production code, tests, fixtures, scripts, and tooling.

## Tests must earn their place

Every test must protect meaningful behavior, a project contract, or a concrete regression risk. Do not add or keep tests that merely mirror implementation, assert incidental wording or source shape, duplicate equivalent coverage, or retest a dependency without protecting a project contract. If a test provides no distinct protection, remove or replace it. Match test cost and scope to the risk; a reversible, low-impact edit does not need a new test by default.

## Campground rule

Leave the area you touch better than you found it. Fix nearby defects, stale guidance, dead code, and needless complexity when the improvement is safe, relevant, and easy to verify. Keep cleanup within the task’s scope, preserve behavior and unrelated work, and record larger follow-up work rather than starting an unrelated rewrite.

## Keep public docs current, honest, and detailed

For every change, review the public docs site and update all affected documentation in the same change. Keep it accurate, honest, and detailed enough for users and contributors to use, operate, and extend the current product. Cover relevant setup, configuration, workflows, APIs, examples, errors, limits, and recovery. Distinguish planned, implemented, tested, and released behavior; state verification, availability, security, and performance limits plainly. Keep navigation, links, examples, generated content, and reference material in sync through the existing docs build. Check the affected docs and include them in the normal release process. If a change has no public docs impact, record the reason in its summary. Do not claim the live site is updated without verifying publication.
