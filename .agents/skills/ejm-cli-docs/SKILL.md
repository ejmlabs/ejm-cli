---
name: ejm-cli-docs
description: Write or update ejm-cli README, docs, help text, site copy, or any claim about speed, memory, security, or compatibility.
---

# ejm-cli docs and claims

Read `README.md`, [the CLI reference](../../../docs/ejm-cli.md), [benchmark evidence](../../../docs/benchmarks.md) and `crates/ejm-cli/src/args.rs`.

- Names: the page is "cli by ejmlabs", the program, crate and binary are `ejm-cli`, environment variables start with `EJM_CLI_`, and the engine is `ejm-harness`. Use sentence case and plain words. Never use em dashes.
- Keep one fact in one place and link to it. A changed flag updates its `--help` text, `docs/ejm-cli.md`, the README and the site together.
- Every number needs a source with the machine, date and method, and it says what it does not prove. The 100 agent, four core, 8 GB target is a target. Do not claim it, an old-laptop result, a speed or quality comparison with another agent, or Windows terminal behavior without evidence.
- Native execution runs as the user and is not a sandbox. Say so wherever grants are described.
- Examples must run. Copy real output, and say when it is condensed. Do not invent an install command, repository URL or release that does not exist. See `$ejm-cli-release`.
- Describe what the program does for the user in plain terms before how it works.

Check the result with `ejm-cli --help` and by running each documented command against `--replay`. Use `$ejm-cli-site` when the change reaches the public page.
