# Design

## Decisions

1. **Run the shared script, do not copy it.** ejmlabs.com keeps its own `site/scripts/perf.mjs`. This site instead runs ejm-ui's `scripts/perf.mjs` from the sibling checkout at the pin, as `ADOPTION.md` tells sites to. The site skill says site infrastructure belongs in ejm-ui and forbids copied shared implementations, and the shared script already handles route discovery, the `ignore` list from `.ejm-ssg/build.json`, gzip serving and baseline updates. The bar is the same as ejmlabs.com: first contentful paint on Lighthouse's desktop profile, failing at 1000 ms over the baseline.
2. **Every route, one run.** The shared default measures every prerendered page (13 here) once. One run takes about 90 seconds; three runs took about 4 minutes. A single run landed within 28 ms of the three-run baseline on every route, far inside the 1000 ms threshold. The baseline itself was recorded as the median of three runs.
3. **A separate command, not part of `npm run check`.** It needs Chrome and takes far longer than the rest of the site gate. ejmlabs.com and ejm-ui also keep `perf` separate.

## Risks

- The numbers depend on the machine class. Re-record with `--update` on a new machine class and state it in the PR.
- The shared gate does not build the site, so nothing runs `perf` in CI. A contributor must run it, as on ejmlabs.com.
