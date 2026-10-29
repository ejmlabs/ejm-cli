# Proposal

## Why

`site/package.json` runs `"budget": "ejm-ssg budget"`, but ejm-ui at the pinned revision `05d314aaf26960096567ef932081ff1db0ceeb26` dropped that subcommand in [ejm-ui PR 35](https://github.com/ejmlabs/ejm-ui/pull/35). `npm run budget` only prints `usage: ejm-ssg build | dev`, so cli.ejmlabs.com has no performance check that can fail. `docs/contributing.md` and the `ejm-cli-site` skill still describe gzip byte budgets that nothing enforces.

## What Changes

- Replace the `budget` script with `"perf": "node ../../ejm-ui/scripts/perf.mjs dist --baseline perf-baseline.json"`. This is the load-time check ejm-ui offers sites in `ADOPTION.md` and `scripts/PERF.md`, run from the sibling checkout at the pin. ejmlabs.com uses the same bar (`npm run perf` against `site/perf-baseline.json`).
- Add `site/perf-baseline.json` with first contentful paint for all 13 prerendered routes.
- Update `docs/contributing.md`, the `ejm-cli-site` skill, the `vite.config.ts` comment and the changelog to describe the load-time bar instead of byte budgets.

### Out of scope

- Running `perf` in CI. The shared gate runs `./scripts/check` for ejm-cli and does not build the site. ejmlabs.com also runs `perf` locally only.
- Changing the shared `perf.mjs` or its threshold.

## Capabilities

### New Capabilities

- `site-load-time`: the cli.ejmlabs.com load-time check and its baseline.

### Modified Capabilities

None.

## Impact

- `site/package.json`, `site/perf-baseline.json`, `site/vite.config.ts`, `docs/contributing.md`, `.agents/skills/ejm-cli-site/SKILL.md`, `CHANGELOG.md`.
- No new dependency: Lighthouse, `chrome-launcher` and `picomatch` are ejm-ui root dev dependencies, installed by `npm --prefix ../ejm-ui ci --ignore-scripts`. The check needs Chrome.
- The rendered `/docs/reference/contributing/` page changes with `docs/contributing.md`.
