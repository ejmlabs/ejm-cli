# Rust test reliability

The reliability workflow runs on main and daily. It checks out the reviewed
harness and UI revisions, runs Rust workspace tests twice through Cargo and twice
through cargo-nextest 0.9.124, and reports each test with both a failure and a
pass. Every run must complete, and any failure leaves the job failed. The CLI
workspace has no library doctest target. The existing human-admitted gate is unchanged; this workflow does
not run unadmitted pull requests.

To reproduce locally, install it with
`cargo install --locked --version 0.9.124 cargo-nextest`, then run
`python3 scripts/flake_check.py /tmp/cli-flakes` with a new output directory.
The detector retains JUnit reports and logs locally; CI uploads them for seven
days. Nextest runs individual
tests in separate processes, so it does not replace the existing Cargo gate or
cover interference between tests in one process. Logs are available in CI.

Nextest 0.9.124 omits ignored tests from its JUnit reports. Tests reported in
only one attempt make the comparison incomplete; tests omitted from both
are not classified or counted as passes.
