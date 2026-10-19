---
name: ejm-cli-theme
description: Change or review ejm-cli colors, light and dark appearance, glyphs, NO_COLOR behavior, or use of the ejmlabs design kit in the terminal.
---

# ejm-cli theme and appearance

Read [the theme module](../../../crates/ejm-cli/src/ejm_theme.rs), `crates/ejm-cli/src/ejm_tui.rs` and `crates/ejm-cli/src/render.rs`. The visual rules live in the ejmlabs design kit's terminal guide at `../ejmlabs/design-kit/ui/TUI.md`, and this repo follows it.

- `ejm_theme.rs` is generated from the kit's `ui/theme.json`. Never edit it by hand. Update it by copying the kit's `ui/rust/ejm_theme.rs`. `ejm_tui.rs` is the hand written widget layer from the same kit. A new color needs a new role in the kit, not a one-off in the app.
- Never paint a background. Colors are text colors that stay readable on the user's own terminal. The only filled surface is the reversed selected row.
- Choose colors in this order: `--no-color` or `NO_COLOR` first, then truecolor (`COLORTERM` contains `truecolor` or `24bit`) with the dark or light palette, then the named 16 color set. Pick light or dark from `--appearance`, then `COLORFGBG`, and default to dark.
- Color is never the only signal. Every state also has a word or a glyph, and the app stays fully usable under `NO_COLOR`. Use ASCII glyphs when the locale is not UTF-8.
- Keep motion calm. One spinner while work runs and nothing when idle.
- New text roles must keep 4.5:1 contrast on both light and dark grounds. Check with the kit's contrast data before adding one.

Prove a change with `TestBackend` renders and the appearance detection tests, then look at it for real in dark, light, 16 color and `NO_COLOR` with `--appearance`. Use `$ejm-cli-terminal` for input and restoration, and finish with `./scripts/check`.
