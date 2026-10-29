# Rust test reliability

The reliability workflow runs on main and daily. It checks out the reviewed
harness and UI revisions, runs Rust workspace tests twice through Cargo and twice
through the pinned cargo-nextest release, and reports each test with both a failure and a
pass. Every run must complete, and any failure leaves the job failed. The CLI
workspace has no library doctest target. The existing human-admitted gate is unchanged; this workflow does
not run unadmitted pull requests.

To reproduce locally, install it with
`cargo install --locked --version "$(python3 -c 'import tomllib; print(tomllib.load(open("tools.toml", "rb"))["cargo-nextest"]["version"])')" cargo-nextest`, then run
`python3 scripts/flake_check.py /tmp/cli-flakes` with a new output directory.
The detector retains JUnit reports and logs locally; CI uploads them for seven
days. Nextest runs individual
tests in separate processes, so it does not replace the existing Cargo gate or
cover interference between tests in one process. Logs are available in CI.

Nextest lists the workspace tests on the host before the attempts and retains that
per-platform inventory with the evidence. Cargo and nextest attempts include ignored tests and
must report every inventoried identity. Missing, skipped or errored results make the
run incomplete. Comparable tests with a pass and a failure are still reported even
when other results are incomplete.

The version and Linux x86-64 archive digest in `tools.toml` are the source
of truth for the workflow. Update both together after reviewing the upstream
release and its checksum; the workflow validates the archive and installed
version. cargo-nextest is a CI and local test tool, not a shipped CLI dependency
or part of the product runtime. Its [0.9.124 source manifest](https://github.com/nextest-rs/nextest/blob/cargo-nextest-0.9.124/cargo-nextest/Cargo.toml)
licenses it under Apache-2.0 OR MIT. The prebuilt archive includes its own
dependencies, outside this workspace's `cargo deny` license and advisory
checks; review the release and bundled dependencies when updating it.
