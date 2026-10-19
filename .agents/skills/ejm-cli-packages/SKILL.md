---
name: ejm-cli-packages
description: Change or review ejm-cli package commands, trusted native extensions, grants, package themes, widgets, hooks, or registry sign-in.
---

# ejm-cli packages and extensions

Read `crates/ejm-cli/src/packages.rs`, `crates/ejm-cli/src/extensions.rs` and the public `ejm-harness-packages` and `ejm-harness-package-spec` APIs. The harness owns installation, manifests, the registry client and the runtime. The CLI owns command parsing, the trust prompt and presenting results.

- Package commands are search, info, install, update, list, enable, disable, remove, pack, login, logout, whoami, publisher and publish. Installed packages live under the state directory's `packages` folder.
- A package is native code that runs as the user. `enable` requires `--trust`, and each grant (tool, hook, provider, command, widget) names an exact export. Never widen a grant implicitly, and keep grants visible in the TUI and in `/extensions`.
- Output from packages and the registry is untrusted text. Show it through `safe_text`. Keep reads bounded, and keep widget output inside the layout's few reserved lines.
- A package theme must be validated `#RRGGBB` values and may override the built-in palette. Choose it only through `--theme` or the theme command.
- Registry tokens are handled by the harness. The CLI never prints, logs or accepts them as arguments.
- `ensure!` on state before acting, for example that a package is installed before `update`.

Test enable, disable and failure paths with local fixtures and never publish from a test. Use `$ejm-cli-harness` when a change needs a harness API, and finish with `./scripts/check`.
