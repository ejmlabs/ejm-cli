---
name: ejm-cli-performance
description: Measure or improve ejm-cli startup, event-to-paint latency, rendering cost, memory footprint, or behavior under many managed agents.
---

# ejm-cli performance

Start with a defined workload, compiler profile, hardware, operating system and measurement method. Use deterministic replay or a local fixture. Never spend provider credits for a benchmark without authorization.

Measure the complete process tree. Separate startup, parked sessions, model requests, local tool jobs and terminal rendering. Record p50/p95 and peak resident memory where measurable. A managed session is not an executing build.

Check long output, slow readers, cancellation and session switching under load. Keep the two runtime threads, small blocking pool, bounded transcripts/composer history, fixed refresh cadence and durable replay behavior visible in measurements.

Change the proven bottleneck, then rerun the same workload. Report raw commands and results in the benchmark evidence. The target is 100 managed agents using hosted models on four cores and 8 GB RAM. A current Mac or a deterministic fixture alone does not prove that target hardware or real hosted latency.

Run `./scripts/check` after the change. Keep performance tests stable by checking limits and regressions with a stated tolerance instead of an unexplained wall-clock cutoff.
