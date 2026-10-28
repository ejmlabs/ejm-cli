---
name: ejm-cli-theme
description: Change or review ejm-cli colors, light and dark appearance, glyphs, NO_COLOR behavior, or use of the shared ejm-tui terminal blocks.
---

# ejm-cli theme and appearance

Read `crates/ejm-cli/src/render.rs` and the shared crate at `../ejm-ui/crates/ejm-tui` (its README, `src/style.rs`, `src/conversation.rs` and `src/workspace.rs`). The rules and states come from `../ejm-ui/packages/core/contract.json`, which the Ink and web targets share.

- Build screens from ejm-tui blocks. When ejm-cli needs a block or state the crate lacks, add it to the contract and to both terminal targets in ejm-ui (Rust and Ink) so every CLI gains it, rather than drawing a one-off here. A new color needs a new role in `packages/tokens`, not a one-off in the app.
- No boxes or frames. Group with space and one hairline `Rule`.
- Never paint a background. Colors are text colors that stay readable on the user's own terminal. Selection is the `›` mark and bold, not a filled row.
- `ejm_tui::Theme::from_env` chooses colors: `--no-color`, `NO_COLOR` or `TERM=dumb` first, then truecolor, then 256 colors, then the named 16 color set. Light or dark comes from `--appearance`, then `COLORFGBG`, and defaults to dark.
- Color is never the only signal. Every state also has a word or a glyph, and the app stays fully usable under `NO_COLOR`. Use ASCII glyphs when the locale is not UTF-8.
- Keep motion calm. One spinner while work runs and nothing when idle.
- New text roles must keep 4.5:1 contrast on both light and dark grounds. Check with the kit's contrast data before adding one.

Prove a change with `TestBackend` renders and the appearance detection tests, then look at it for real in dark, light, 16 color and `NO_COLOR` with `--appearance`. Use `$ejm-cli-terminal` for input and restoration, and finish with `./scripts/check`.
