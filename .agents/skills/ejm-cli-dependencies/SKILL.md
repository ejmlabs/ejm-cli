---
name: ejm-cli-dependencies
description: Update ejm-cli Rust dependencies, compiler pins, quality tools, CI configuration, or the sibling ejm-harness dependency boundary.
---

# ejm-cli dependencies and checks

Read [the contributor guide](../../../docs/contributing.md), `tools.toml`, both Cargo manifests, `Cargo.lock`, and the changed check or workflow.

Keep ejm-cli an independent workspace consuming the public sibling ejm-harness crates. Review any new production dependency against `scripts/check_architecture.py`, including binary size, startup, licenses, maintenance and transitive features.

Use trusted, maintained packages for standard infrastructure. Review the smallest suitable package for protocol parsing, cryptography, persistence and other established concerns. Custom TUI interaction and app state are ejm-cli product behavior. A custom replacement for a standard concern needs a concrete unmet requirement and evidence that the package alternatives do not satisfy it.

Use the pinned compiler and committed lockfile. Update a pin deliberately with its evidence. Never bypass a failing lint with a workspace-wide allowance. An item-level expectation requires a precise reason and must describe a real exception; remove it when no longer needed.

CI action references and the harness checkout revision must be immutable commits. Keep the full `./scripts/check` gate for local completion, release and CI; the pre-commit hook runs only quick checks. New tools need an exact version and a documented installation command. Run new scripts, validate skill routing, and execute the full check before reporting completion.
