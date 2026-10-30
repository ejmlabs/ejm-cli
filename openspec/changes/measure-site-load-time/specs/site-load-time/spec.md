# Spec Delta

## Purpose

Give cli.ejmlabs.com a performance check that fails on a regression, using the load-time bar ejm-ui offers sites.

## ADDED Requirements

### Requirement: The site has a load-time check
`npm run perf` in `site/` MUST run the pinned ejm-ui `scripts/perf.mjs` against `site/dist` with `site/perf-baseline.json`, measure first contentful paint for every prerendered route, and exit non-zero when a route is 1000 ms or more above its baseline. The site MUST NOT keep a script that names a removed `ejm-ssg` subcommand.

#### Scenario: Within the baseline
- **WHEN** `npm run perf` runs after `npm run build` on an unchanged site
- **THEN** it prints one row per route with its first contentful paint and baseline, and exits 0

#### Scenario: A route regresses by a second or more
- **WHEN** a route adds enough render-blocking work to delay first contentful paint by 1000 ms or more
- **THEN** `npm run perf` exits 1 and names the route, both numbers and the difference

### Requirement: The baseline changes only by review
`site/perf-baseline.json` MUST be recorded with `npm run perf -- --update` and change only in a reviewed change whose description gives the measurement and the reason.

#### Scenario: Recording a new baseline
- **WHEN** a contributor runs `npm run perf -- --update`
- **THEN** the file holds the new medians and the PR states the old and new numbers and why they moved
