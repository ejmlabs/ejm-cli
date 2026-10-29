# Tasks

## 1. Load-time check

- [x] 1.1 Replace `"budget": "ejm-ssg budget"` in `site/package.json` with `"perf": "node ../../ejm-ui/scripts/perf.mjs dist --baseline perf-baseline.json"`.
- [x] 1.2 Record `site/perf-baseline.json` with `node ../../ejm-ui/scripts/perf.mjs dist --baseline perf-baseline.json --runs 3 --update` from a clean build at ejm-ui `05d314aaf26960096567ef932081ff1db0ceeb26`.
- [x] 1.3 Run `npm run perf` against the same build: every route within the threshold, exit 0.
- [x] 1.4 Prove a regression fails: add a render-blocking 4 MB stylesheet to `/` in a copy of `dist`, run the check with `--routes /`, expect exit 1 naming `/`.

## 2. Docs

- [x] 2.1 Replace the gzip route budgets paragraph in `docs/contributing.md` with the load-time bar and add `npm --prefix site run perf` to the site commands.
- [x] 2.2 Update the `ejm-cli-site` skill (description, budget rule, check commands) and the `site/vite.config.ts` comment.
- [x] 2.3 Add a Website changelog entry.

## 3. Verify

- [x] 3.1 Run `npm --prefix site run check`.
- [x] 3.2 Run `./scripts/check`.
