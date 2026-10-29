# Design

## Layout

See the [CLI reference](../../../docs/ejm-cli.md#interaction) for the current screen layout.

## Transcript

Each block draws as an `ejm_tui::Message` or `ToolCall`, so the gutter glyphs, Markdown, tool previews and their ASCII and `NO_COLOR` forms are shared with every ejmlabs CLI. Tool blocks carry a `ToolView` (name, main argument, `ToolCallState`); `text` holds only the output, which keeps byte accounting and remote tests valid. The renderer wraps from the newest block backwards only as far as the screen and scroll offset need.

## Composer

ejm-cli keeps its own character wrapping for the draft so the real terminal cursor lands exactly, which input methods and screen readers follow. The rows use the shared prompt glyph and theme.

## Motion and redraw

The spinner advances at 80 ms per frame only while a run is active. Idle screens redraw only on change. The second Ctrl+C is valid for two seconds.
