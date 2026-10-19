# Performance evidence

ejm-cli's acceptance target is 100 managed hosted-model agents on four CPU
cores, 8 GB RAM, and an SSD. Managed agents share model and local command pools.
The CLI should paint a warm composer in under 300 ms at p95, retain bounded
scrollback, and acknowledge Stop promptly while cleanup proceeds separately.

The sibling ejm-harness repository owns the reproducible benchmark runner and
the raw report under `docs/evidence/benchmark.json`:

```sh
cargo build --release --locked
(cd ../ejm-harness && cargo build --release --locked -p ejm-harness-bench)
(cd ../ejm-harness && python3 scripts/benchmark.py --agents 100 --iterations 20 --output docs/evidence/benchmark.json)
```

The final local release run on 2026-10-18 measured first composer paint at
24.197 ms with fresh state and 41.312 ms at warm p95 across 20 samples. It used
a Mac16,8 with 12 logical CPUs, 24 GiB RAM, macOS 26.5.2, and Rust 1.99.1.
The [raw report](https://github.com/ejmlabs/ejm-harness/blob/main/docs/evidence/benchmark.json) includes the
CLI binary hash and every sample. No hosted API calls were made.

The runner measures process creation to the first composer paint in a real
100x30 PTY, with a replay provider and reused state directory. First paint
includes process creation and local initialization. It does not include remote
model latency. Its engine workloads record separately sampled RSS and actual
governor concurrency.

`./scripts/check` also runs a real PTY check for input, offline output, signal
handling, and terminal restoration. The refresh interval and transcript bounds
are implementation facts, not timing evidence. Input-to-stop and delta-to-paint
timings need separate trace instrumentation.

A result from a modern machine does not establish the old-laptop target, and no
controlled speed or coding-quality comparison with another agent has run.
