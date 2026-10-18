---
name: ejm-cli-change
description: Implement ejm-cli commands, session interaction, or TUI changes through the public ejm-harness API.
---

# ejm-cli changes

Read `AGENTS.md` and [the contributor guide](../../../docs/contributing.md). Map the request to argument parsing, app state, input handling, rendering, and documentation before editing.

- Model state belongs in `App`. Terminal drawing stays in `render`. Keep run policy, providers, tools, storage and resource admission in the sibling ejm-harness project.
- Use the public harness methods. A new UI command must preserve cancellation, grants, session identity and durable event ordering.
- Keep user input after failed submissions. Never make paste submit a prompt. Bound added history, buffers and background tasks.
- Wire flags to their real owning configuration and show effective grants. API keys come from environment variables; saved sign-ins use the shared `ejm-harness-auth` store. Credentials never appear in flags or logs. Keep browser launch and callback listening in the CLI host.
- Use traits where an actual provider, executor or policy can vary. Do not add a dependency injection framework or a factory for a single concrete implementation.
- Use trusted, maintained packages for general infrastructure and protocol handling. Keep custom code focused on ejm-cli's distinct behavior and document a verified reason when a package cannot meet the need.

Add an observable check for changed behavior. Use `$ejm-cli-terminal` for terminal lifecycle work and `$ejm-cli-performance` for latency or memory work. Finish with `./scripts/check` and report actual results and limits.
