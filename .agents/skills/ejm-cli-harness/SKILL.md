---
name: ejm-cli-harness
description: Change how ejm-cli depends on the sibling ejm-harness crates, refresh the lockfile after harness changes, or coordinate with the harness and IDE projects.
---

# ejm-cli and the sibling harness

Read the workspace `Cargo.toml`, `crates/ejm-cli/Cargo.toml`, `deny.toml`, `scripts/check_architecture.py` and [the contributor guide](../../../docs/contributing.md).

- ejm-cli consumes the public crates by path from `../ejm-harness/crates/`: `ejm-harness`, `ejm-harness-coding`, `ejm-harness-exec`, `ejm-harness-auth`, `ejm-harness-packages` and `ejm-harness-package-spec`. Rust modules use underscores, for example `ejm_harness_exec`. Use only their public interface.
- Model calls, tools, process execution and storage belong in the harness. If the CLI needs something new, ask for a public harness API instead of reaching around it.
- After the harness changes a dependency, refresh `Cargo.lock` before blaming the code. Stale pins caused compile errors and duplicate crates before. Unify versions with `cargo update -p NAME --precise VERSION` rather than adding a `deny.toml` skip. A skip is an exact version with a written reason.
- Never edit `../ejm-harness` or `../ejm-ide` without the owner's agreement. Other agents may be working there. If a sibling does not compile, tell its session what fails and let it fix it, then wait.
- Keep the allowed dependency list in `scripts/check_architecture.py` current, with a reason for each addition.
- CI pins the harness by `EJM_HARNESS_REPOSITORY` and an immutable `EJM_HARNESS_REVISION`. Never point CI at a branch.

Run `cargo check --locked --all-targets -p ejm-cli` after a harness update, then `./scripts/check`. See `$ejm-cli-dependencies` for tools and compiler pins.
