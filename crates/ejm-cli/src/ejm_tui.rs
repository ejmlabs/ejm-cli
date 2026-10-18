//! ejmlabs terminal building blocks for ratatui apps. See ui/TUI.md for what each block looks like.
//!
//! Hand written, unlike `ejm_theme.rs`, which is generated from `ui/theme.json`. This file sits on
//! top of it and turns its palettes and glyphs into styles and small widget helpers, so an app
//! does not rebuild the basics.
//!
//! Expected layout: both files are sibling modules in your crate.
//!
//! ```text
//! src/lib.rs (or main.rs)    mod ejm_theme; mod ejm_tui;
//! src/ejm_theme.rs           copied from design-kit/ui/rust/ejm_theme.rs
//! src/ejm_tui.rs             copied from design-kit/ui/rust/ejm_tui.rs
//! ```
//!
//! Dependencies: `ratatui` 0.30 (default features off), `unicode-width` 0.2. Text from the
//! network, tools or files must be stripped of terminal control sequences before it reaches any
//! helper here (ejm-cli does this with `vte` in `safe_text`). Every width is in terminal columns,
//! counted with `unicode-width`, never with `len()`.

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    symbols::border,
    text::{Line, Span},
    widgets::{Block, BorderType, Padding},
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use super::ejm_theme::{
    ASCII, BASIC_DARK, BASIC_LIGHT, Basic, DARK, Glyphs, LIGHT, Palette, UNICODE,
};

/// Smallest terminal width the app supports. Below this show only the enlarge notice.
pub const MIN_WIDTH: u16 = 20;
/// Smallest terminal height the app supports.
pub const MIN_HEIGHT: u16 = 8;

// ---------------------------------------------------------------------------------------------
// Theme
// ---------------------------------------------------------------------------------------------

/// Which palette to prefer: from the terminal (`Auto`), or forced by a flag.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Appearance {
    /// Read `COLORFGBG`, then assume a dark terminal.
    #[default]
    Auto,
    Light,
    Dark,
}

/// Text styles for every role. No style sets a background, except `selected`, which reverses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Theme {
    pub accent: Style,
    pub dim: Style,
    pub warning: Style,
    pub error: Style,
    pub success: Style,
    /// Terminal default colors.
    pub plain: Style,
    pub bold: Style,
    /// Accent, reversed. The only filled surface in the kit.
    pub selected: Style,
    /// Diff hunk headers. Palette `special` in truecolor, `dim` on 16 colors (Basic has no
    /// special), plain under NO_COLOR.
    pub special: Style,
}

impl Theme {
    /// Choose styles in the TUI.md order: no color, then truecolor palette, then 16 colors.
    pub fn detect(
        no_color: bool,
        appearance: Appearance,
        colorfgbg: Option<&str>,
        colorterm: Option<&str>,
    ) -> Self {
        if no_color {
            return Self::no_color();
        }
        let light = is_light(appearance, colorfgbg);
        if is_truecolor(colorterm) {
            Self::from_palette(if light { &LIGHT } else { &DARK })
        } else {
            Self::from_basic(if light { &BASIC_LIGHT } else { &BASIC_DARK })
        }
    }

    /// `detect` reading `NO_COLOR`, `COLORFGBG` and `COLORTERM` from the environment.
    /// `no_color_flag` is your `--no-color` flag. `NO_COLOR` counts when set.
    pub fn from_env(no_color_flag: bool, appearance: Appearance) -> Self {
        let var = |name: &str| std::env::var(name).ok();
        Self::detect(
            no_color_flag || std::env::var_os("NO_COLOR").is_some(),
            appearance,
            var("COLORFGBG").as_deref(),
            var("COLORTERM").as_deref(),
        )
    }

    /// Bold only: accent, warning and error bold; dim, success and special plain.
    pub fn no_color() -> Self {
        let bold = Style::default().add_modifier(Modifier::BOLD);
        Self::build(bold, Style::default(), bold, bold, Style::default(), Style::default())
    }

    /// Truecolor styles from a generated palette.
    pub fn from_palette(p: &Palette) -> Self {
        let fg = |c| Style::default().fg(c);
        Self::build(fg(p.accent), fg(p.dim), fg(p.warning), fg(p.error), fg(p.success), fg(p.special))
    }

    /// Named 16 color styles. Basic has no special role, so hunk headers use dim.
    pub fn from_basic(b: &Basic) -> Self {
        let fg = |c| Style::default().fg(c);
        Self::build(fg(b.accent), fg(b.dim), fg(b.warning), fg(b.error), fg(b.success), fg(b.dim))
    }

    fn build(accent: Style, dim: Style, warning: Style, error: Style, success: Style, special: Style) -> Self {
        Self {
            accent,
            dim,
            warning,
            error,
            success,
            plain: Style::default(),
            bold: Style::default().add_modifier(Modifier::BOLD),
            selected: accent.add_modifier(Modifier::REVERSED),
            special,
        }
    }
}

/// Light when forced, or when `COLORFGBG` ("fg;bg", ANSI indexes) has background 7 or 9 to 15.
pub fn is_light(appearance: Appearance, colorfgbg: Option<&str>) -> bool {
    match appearance {
        Appearance::Light => true,
        Appearance::Dark => false,
        Appearance::Auto => colorfgbg
            .and_then(|value| value.rsplit(';').next())
            .and_then(|bg| bg.parse::<u8>().ok())
            .is_some_and(|bg| bg == 7 || (9..=15).contains(&bg)),
    }
}

/// True when `COLORTERM` contains `truecolor` or `24bit`.
pub fn is_truecolor(colorterm: Option<&str>) -> bool {
    colorterm.is_some_and(|v| v.contains("truecolor") || v.contains("24bit"))
}

// ---------------------------------------------------------------------------------------------
// Glyphs
// ---------------------------------------------------------------------------------------------

/// UNICODE unless `plain` is set or the locale is not UTF-8. An unset locale counts as not UTF-8.
pub fn glyph_set(plain: bool, locale: Option<&str>) -> &'static Glyphs {
    if !plain && locale.is_some_and(is_utf8_locale) { &UNICODE } else { &ASCII }
}

/// True when a locale string such as `en_US.UTF-8` or `C.utf8` names UTF-8.
pub fn is_utf8_locale(locale: &str) -> bool {
    let l = locale.to_ascii_lowercase();
    l.contains("utf-8") || l.contains("utf8")
}

/// The first non-empty of `LC_ALL`, `LC_CTYPE`, `LANG`, which is the order libc uses.
pub fn pick_locale<'a>(lc_all: Option<&'a str>, lc_ctype: Option<&'a str>, lang: Option<&'a str>) -> Option<&'a str> {
    [lc_all, lc_ctype, lang].into_iter().flatten().find(|v| !v.is_empty())
}

/// `glyph_set` reading the locale from the environment. `plain` is your `--plain` flag.
pub fn glyphs_from_env(plain: bool) -> &'static Glyphs {
    let var = |name: &str| std::env::var(name).ok();
    let (a, b, c) = (var("LC_ALL"), var("LC_CTYPE"), var("LANG"));
    glyph_set(plain, pick_locale(a.as_deref(), b.as_deref(), c.as_deref()))
}

/// True for the ASCII glyph set (compares the separator, which differs between the sets).
pub fn is_ascii(g: &Glyphs) -> bool {
    g.separator == ASCII.separator
}

// ---------------------------------------------------------------------------------------------
// Text measuring and fitting
// ---------------------------------------------------------------------------------------------

/// Display width in terminal columns.
pub fn width(s: &str) -> usize {
    UnicodeWidthStr::width(s)
}

fn take_cols(s: &str, max: usize) -> String {
    let (mut used, mut out) = (0, String::new());
    for c in s.chars() {
        let w = c.width().unwrap_or(0);
        if used + w > max {
            break;
        }
        used += w;
        out.push(c);
    }
    out
}

/// Cut to `max` columns, ending with `ellipsis` when something was removed. Never splits a wide char.
pub fn truncate(s: &str, max: usize, ellipsis: &str) -> String {
    if width(s) <= max {
        return s.to_string();
    }
    let ew = width(ellipsis);
    if ew >= max {
        return take_cols(ellipsis, max);
    }
    let mut out = take_cols(s, max - ew);
    out.push_str(ellipsis);
    out
}

/// Pad with spaces on the right up to `w` columns.
pub fn pad_right(s: &str, w: usize) -> String {
    format!("{s}{}", " ".repeat(w.saturating_sub(width(s))))
}

/// Pad with spaces on the left up to `w` columns.
pub fn pad_left(s: &str, w: usize) -> String {
    format!("{}{s}", " ".repeat(w.saturating_sub(width(s))))
}

/// Break at `w` columns with no regard for words. Use for code, diffs and tool output.
pub fn wrap_chars(text: &str, w: usize) -> Vec<String> {
    let w = w.max(1);
    let (mut lines, mut cur, mut cols) = (Vec::new(), String::new(), 0);
    for c in text.chars() {
        let cw = c.width().unwrap_or(0);
        if cols + cw > w && !cur.is_empty() {
            lines.push(std::mem::take(&mut cur));
            cols = 0;
        }
        cur.push(c);
        cols += cw;
    }
    lines.push(cur);
    lines
}

/// Break one logical line at spaces to `w` columns, keeping its indent. Long words break by column.
pub fn wrap_text(text: &str, w: usize) -> Vec<String> {
    let w = w.max(1);
    let trimmed = text.trim_start_matches(' ');
    let indent = text.len() - trimmed.len();
    let indent = if indent >= w { 0 } else { indent };
    let pad = " ".repeat(indent);
    let mut lines = Vec::new();
    let mut cur = pad.clone();
    for word in trimmed.split_whitespace() {
        let fresh = cur.len() == indent;
        let need = width(&cur) + usize::from(!fresh) + width(word);
        if need <= w {
            if !fresh {
                cur.push(' ');
            }
            cur.push_str(word);
            continue;
        }
        if !fresh {
            lines.push(std::mem::replace(&mut cur, pad.clone()));
        }
        let mut rest = word;
        while indent + width(rest) > w {
            let mut head = take_cols(rest, w - indent);
            if head.is_empty() {
                head = rest.chars().next().map(String::from).unwrap_or_default();
            }
            rest = &rest[head.len()..];
            lines.push(format!("{pad}{head}"));
        }
        cur.push_str(rest);
    }
    lines.push(cur);
    lines
}

/// Join styled pieces into one line, cutting the end with an ellipsis if wider than `max`.
fn fit(parts: Vec<(String, Style)>, max: usize, ellipsis: &str) -> Line<'static> {
    let total: usize = parts.iter().map(|(t, _)| width(t)).sum();
    if total <= max {
        return Line::from(parts.into_iter().map(|(t, s)| Span::styled(t, s)).collect::<Vec<_>>());
    }
    let mut budget = max.saturating_sub(width(ellipsis));
    let (mut spans, mut last) = (Vec::new(), Style::default());
    for (text, style) in parts {
        if budget == 0 {
            break;
        }
        let piece = take_cols(&text, budget);
        budget -= width(&piece);
        last = style;
        let cut = piece.len() < text.len();
        spans.push(Span::styled(piece, style));
        if cut {
            break;
        }
    }
    spans.push(Span::styled(take_cols(ellipsis, max), last));
    Line::from(spans)
}

fn styled_lines(texts: Vec<String>, style: Style) -> Vec<Line<'static>> {
    texts.into_iter().map(|t| Line::styled(t, style)).collect()
}

// ---------------------------------------------------------------------------------------------
// Motion
// ---------------------------------------------------------------------------------------------

/// Spinner glyph for `elapsed_ms`, 80 ms per frame. Reduced motion returns the static running glyph.
pub fn spinner_frame(g: &Glyphs, elapsed_ms: u64, reduced_motion: bool) -> &'static str {
    if reduced_motion {
        return g.running;
    }
    g.spinner[(elapsed_ms / 80) as usize % g.spinner.len()]
}

// ---------------------------------------------------------------------------------------------
// Header, status, hints
// ---------------------------------------------------------------------------------------------

/// Fields of the two header lines.
pub struct Header<'a> {
    pub app: &'a str,
    pub provider: &'a str,
    pub title: &'a str,
    /// Spelled out, for example `read + write`.
    pub grants: &'a str,
    /// True when write or shell access is on. Turns the grants text warning.
    pub elevated: bool,
    pub cwd: &'a str,
    pub session_id: &'a str,
}

/// First 8 characters of a session id.
pub fn short_id(id: &str) -> &str {
    id.get(..8).unwrap_or(id)
}

/// The two header lines: name, provider and title; then grants, directory and short id.
pub fn header_lines(theme: &Theme, g: &Glyphs, h: &Header<'_>, w: usize) -> [Line<'static>; 2] {
    let sep = format!("  {}  ", g.separator);
    let first = vec![
        (format!(" {} ", h.app), theme.accent.add_modifier(Modifier::BOLD)),
        (h.provider.to_string(), theme.plain),
        (format!("{sep}{}", h.title), theme.dim),
    ];
    let grants = if h.elevated { theme.warning } else { theme.dim };
    let second = vec![
        (format!(" grants: {}", h.grants), grants),
        (format!("{sep}{}", h.cwd), theme.dim),
        (format!("{sep}{}", short_id(h.session_id)), theme.dim),
    ];
    [fit(first, w, g.ellipsis), fit(second, w, g.ellipsis)]
}

/// What the run is doing, for the status line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunState {
    Idle,
    Running,
    Stopping,
    /// The transcript is scrolled up by this many lines.
    ScrolledUp(usize),
}

/// Counts joined by the separator, then the run state. Drops counts from the right if too narrow.
/// `label` is the status word ("Ready", "Streaming…") for Idle and Running; the other states
/// have fixed text.
pub fn status_line(theme: &Theme, g: &Glyphs, counts: &[String], state: RunState, label: &str, w: usize) -> Line<'static> {
    let run = match state {
        RunState::Idle => label.to_string(),
        RunState::Running => format!("{} {label}", g.running),
        RunState::Stopping => format!("{} Stopping{}", g.running, g.ellipsis),
        RunState::ScrolledUp(n) => format!(
            "{} {n} line{} above latest {} End to follow",
            g.up,
            if n == 1 { "" } else { "s" },
            g.separator
        ),
    };
    let style = if matches!(state, RunState::Running | RunState::Stopping) { theme.accent } else { theme.dim };
    let sep = format!(" {} ", g.separator);
    let compose = |n: usize| {
        let mut parts: Vec<&str> = counts[..n].iter().map(String::as_str).collect();
        parts.push(&run);
        format!(" {}", parts.join(&sep))
    };
    let mut n = counts.len();
    while n > 0 && width(&compose(n)) > w {
        n -= 1;
    }
    fit(vec![(compose(n), style)], w, g.ellipsis)
}

/// Key hints as `Key action` pairs joined by the separator, dim. List the most important pair
/// first: pairs that do not fit are dropped from the end.
pub fn hints_text(g: &Glyphs, pairs: &[(&str, &str)], w: usize) -> String {
    let sep = format!(" {} ", g.separator);
    let items: Vec<String> = pairs.iter().map(|(k, a)| format!("{k} {a}")).collect();
    let mut out = String::from(" ");
    for (i, item) in items.iter().enumerate() {
        let next = if i == 0 { item.clone() } else { format!("{sep}{item}") };
        if width(&out) + width(&next) > w {
            break;
        }
        out.push_str(&next);
    }
    if out == " " {
        if let Some(first) = items.first() {
            out.push_str(&truncate(first, w.saturating_sub(1), g.ellipsis));
        }
    }
    take_cols(&out, w)
}

/// `hints_text` as a dim line.
pub fn hints(theme: &Theme, g: &Glyphs, pairs: &[(&str, &str)], w: usize) -> Line<'static> {
    Line::styled(hints_text(g, pairs, w), theme.dim)
}

// ---------------------------------------------------------------------------------------------
// Transcript, diff, empty and error states
// ---------------------------------------------------------------------------------------------

/// Kind of transcript block. Sets the title style.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockKind {
    User,
    Assistant,
    Tool,
    Notice,
    Error,
}

/// Style for one diff line, or `None` when it is not a diff line. File headers dim, hunk
/// headers special, `+` success, `-` error.
pub fn diff_style(theme: &Theme, text: &str) -> Option<Style> {
    if ["+++", "---", "diff --git", "index "].iter().any(|p| text.starts_with(p)) {
        Some(theme.dim)
    } else if text.starts_with("@@") {
        Some(theme.special)
    } else if text.starts_with('+') {
        Some(theme.success)
    } else if text.starts_with('-') {
        Some(theme.error)
    } else {
        None
    }
}

/// One diff line styled by `diff_style`, plain when it is a context line.
pub fn diff_line(theme: &Theme, text: &str) -> Line<'static> {
    Line::styled(text.to_string(), diff_style(theme, text).unwrap_or(theme.plain))
}

/// A titled block and its body as wrapped lines, ending with one blank line. `w` is the usable
/// width (area width minus the one-cell pad on each side). Code fences, diffs and tool output
/// break by column; prose breaks at spaces.
pub fn transcript_block(theme: &Theme, kind: BlockKind, title: &str, body: &str, w: usize) -> Vec<Line<'static>> {
    let title_style = match kind {
        BlockKind::User => theme.accent,
        BlockKind::Assistant => theme.bold,
        BlockKind::Tool => theme.dim,
        BlockKind::Notice => theme.warning,
        BlockKind::Error => theme.error,
    };
    let mut lines = styled_lines(wrap_chars(title, w), title_style);
    let has_diff = body.lines().any(|l| l.starts_with("@@ ") || l.starts_with("diff --git "));
    let (mut fenced, mut fenced_diff) = (false, false);
    for text in body.lines() {
        let t = text.trim_start();
        let (style, by_column) = if t.starts_with("```") {
            fenced = !fenced;
            fenced_diff = fenced && (t.starts_with("```diff") || t.starts_with("```patch"));
            (theme.dim, true)
        } else if let Some(s) = (fenced_diff || has_diff).then(|| diff_style(theme, text)).flatten() {
            (s, true)
        } else if text.starts_with('#') {
            (theme.accent.add_modifier(Modifier::BOLD), false)
        } else if fenced || kind == BlockKind::Tool {
            (theme.dim, true)
        } else if kind == BlockKind::Error {
            (theme.error, false)
        } else {
            (theme.plain, false)
        };
        let wrapped = if by_column { wrap_chars(text, w) } else { wrap_text(text, w) };
        lines.extend(styled_lines(wrapped, style));
    }
    lines.push(Line::raw(""));
    lines
}

/// The empty transcript: title in bold accent, then dim help lines, each followed by a blank line.
pub fn empty_state(theme: &Theme, title: &str, help: &[&str], w: usize) -> Vec<Line<'static>> {
    let mut lines = vec![Line::raw(""), Line::styled(format!("  {title}"), theme.accent.add_modifier(Modifier::BOLD)), Line::raw("")];
    for text in help {
        lines.extend(styled_lines(wrap_text(text, w), theme.dim));
        lines.push(Line::raw(""));
    }
    lines
}

/// An error: what failed, then the next step, both in the error style, wrapped.
pub fn error_state(theme: &Theme, what: &str, next: &str, w: usize) -> Vec<Line<'static>> {
    let mut lines = styled_lines(wrap_text(what, w), theme.error);
    lines.extend(styled_lines(wrap_text(next, w), theme.error));
    lines
}

/// Which banner: a notice or an error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BannerKind {
    Notice,
    Error,
}

/// A glyph and a sentence, wrapped with a hanging indent. Notice is `!` in warning, error is the fail glyph in error.
pub fn banner(theme: &Theme, g: &Glyphs, kind: BannerKind, text: &str, w: usize) -> Vec<Line<'static>> {
    let (glyph, style) = match kind {
        BannerKind::Notice => (g.warn, theme.warning),
        BannerKind::Error => (g.fail, theme.error),
    };
    let hang = width(glyph) + 1;
    wrap_text(text, w.saturating_sub(hang).max(1))
        .into_iter()
        .enumerate()
        .map(|(i, l)| Line::styled(if i == 0 { format!("{glyph} {l}") } else { format!("{} {l}", " ".repeat(hang - 1)) }, style))
        .collect()
}

/// Replacement screen text below the minimum size.
pub fn too_small_lines(app: &str, w: usize) -> Vec<Line<'static>> {
    wrap_text(&format!("Enlarge the terminal to use {app}."), w).into_iter().map(Line::raw).collect()
}

/// True when the terminal is below `MIN_WIDTH` by `MIN_HEIGHT`.
pub fn too_small(area: Rect) -> bool {
    area.width < MIN_WIDTH || area.height < MIN_HEIGHT
}

// ---------------------------------------------------------------------------------------------
// Composer, blocks, overlays
// ---------------------------------------------------------------------------------------------

const ASCII_BORDER: border::Set = border::Set {
    top_left: "+",
    top_right: "+",
    bottom_left: "+",
    bottom_right: "+",
    vertical_left: "|",
    vertical_right: "|",
    horizontal_top: "-",
    horizontal_bottom: "-",
};

/// A bordered block with a title and one cell of side padding. Rounded corners, or `+ - |` in ASCII.
pub fn titled_block(g: &Glyphs, title: &str, border_style: Style) -> Block<'static> {
    let block = Block::bordered().border_style(border_style).title(format!(" {title} ")).padding(Padding::horizontal(1));
    if is_ascii(g) { block.border_set(ASCII_BORDER) } else { block.border_type(BorderType::Rounded) }
}

/// The composer box: accent border, title "message", with a queue note while running.
pub fn composer_block(theme: &Theme, g: &Glyphs, running: bool) -> Block<'static> {
    let title = if running { format!("message {} Enter queues", g.separator) } else { "message".to_string() };
    titled_block(g, &title, theme.accent)
}

/// Composer rows: the prompt glyph in accent on the first row, two spaces on the rest. An empty
/// composer shows a dim placeholder. Wrap the text with `composer_layout` at the block's inner
/// width minus 2 for the prompt, and place the terminal cursor yourself.
pub fn composer_rows(theme: &Theme, g: &Glyphs, rows: &[String], running: bool) -> Vec<Line<'static>> {
    if rows.iter().all(String::is_empty) {
        let hint = if running { format!("Queue a follow-up, or Ctrl+K to steer{}", g.ellipsis) } else { format!("Describe a task{}", g.ellipsis) };
        return vec![Line::from(vec![Span::styled(format!("{} ", g.prompt), theme.accent), Span::styled(hint, theme.dim)])];
    }
    (rows.iter().enumerate())
        .map(|(i, row)| {
            let lead = if i == 0 { Span::styled(format!("{} ", g.prompt), theme.accent) } else { Span::raw("  ") };
            Line::from(vec![lead, Span::raw(row.clone())])
        })
        .collect()
}

/// Wrap composer text to `w` columns. Returns the rows, then the cursor row and column for byte index `cursor`.
pub fn composer_layout(text: &str, cursor: usize, w: usize) -> (Vec<String>, usize, usize) {
    let w = w.max(1);
    let mut rows = vec![String::new()];
    let (mut cols, mut pos) = (0, (0, 0));
    for (index, c) in text.char_indices() {
        let cw = c.width().unwrap_or(0);
        if c != '\n' && cols + cw > w {
            rows.push(String::new());
            cols = 0;
        }
        if index == cursor {
            pos = (rows.len() - 1, cols);
        }
        if c == '\n' {
            rows.push(String::new());
            cols = 0;
        } else {
            rows.last_mut().expect("rows is never empty").push(c);
            cols += cw;
        }
    }
    if cursor == text.len() {
        if cols >= w {
            rows.push(String::new());
            cols = 0;
        }
        pos = (rows.len() - 1, cols);
    }
    (rows, pos.0, pos.1)
}

/// A rectangle centered in `area`, at most `w` by `h`, leaving a one-cell margin.
pub fn popup(area: Rect, w: u16, h: u16) -> Rect {
    let w = w.min(area.width.saturating_sub(2));
    let h = h.min(area.height.saturating_sub(2));
    Rect::new(area.x + (area.width - w) / 2, area.y + (area.height - h) / 2, w, h)
}

/// A picker or list row: `label`, two spaces, dim `detail`. The selected row is reversed accent across the full width.
pub fn list_row(theme: &Theme, g: &Glyphs, label: &str, detail: &str, selected: bool, w: usize) -> Line<'static> {
    let text = truncate(&format!(" {label}  {detail}"), w, g.ellipsis);
    if selected {
        return Line::styled(pad_right(&text, w), theme.selected);
    }
    let label_part = truncate(&format!(" {label}"), w, g.ellipsis);
    let rest = w.saturating_sub(width(&label_part));
    let detail_part = truncate(&format!("  {detail}"), rest, g.ellipsis);
    Line::from(vec![Span::styled(label_part, theme.plain), Span::styled(detail_part, theme.dim)])
}

// ---------------------------------------------------------------------------------------------
// Tabs, key-value, table, progress, steps, prompts
// ---------------------------------------------------------------------------------------------

/// Tab labels in a row. The active tab is reversed accent, the others dim.
pub fn tabs_line(theme: &Theme, g: &Glyphs, labels: &[&str], active: usize, w: usize) -> Line<'static> {
    let mut parts = vec![(" ".to_string(), theme.plain)];
    for (i, label) in labels.iter().enumerate() {
        if i > 0 {
            parts.push((" ".to_string(), theme.plain));
        }
        parts.push((format!(" {label} "), if i == active { theme.selected } else { theme.dim }));
    }
    fit(parts, w, g.ellipsis)
}

/// Dim key padded to `key_width`, then the plain value, cut to `w`.
pub fn kv_line(theme: &Theme, g: &Glyphs, key: &str, value: &str, key_width: usize, w: usize) -> Line<'static> {
    fit(vec![(pad_right(key, key_width), theme.dim), (format!("  {value}"), theme.plain)], w, g.ellipsis)
}

/// How a table cell sits in its column. Use `Right` for numbers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align {
    Left,
    Right,
}

/// One table row from `(text, column width, alignment)`, cells two spaces apart, each cut to its column.
pub fn table_row(g: &Glyphs, cols: &[(&str, usize, Align)], style: Style) -> Line<'static> {
    let cells: Vec<String> = cols
        .iter()
        .map(|&(text, w, align)| {
            let t = truncate(text, w, g.ellipsis);
            if align == Align::Right { pad_left(&t, w) } else { pad_right(&t, w) }
        })
        .collect();
    Line::styled(format!(" {}", cells.join("  ")), style)
}

fn bar_cells(n: usize, frac: f64) -> String {
    if frac >= 1.0 {
        return "=".repeat(n);
    }
    if frac <= 0.0 || n == 0 {
        return "-".repeat(n);
    }
    let filled = ((frac * n as f64) as usize).min(n - 1);
    format!("{}>{}", "=".repeat(filled), "-".repeat(n - filled - 1))
}

/// ASCII bar of exactly `w` columns, like `[=====>-----]  50%`. From width 9 up it has brackets and a
/// percent; from 3 to 8 brackets only; below 3 bare cells. 100% only when `done >= total`.
pub fn progress_bar(done: u64, total: u64, w: usize) -> String {
    let frac = if total == 0 { 0.0 } else { done.min(total) as f64 / total as f64 };
    let pct = if total == 0 { 0 } else { (u128::from(done.min(total)) * 100 / u128::from(total)) as u32 };
    match w {
        0 => String::new(),
        1..=2 => bar_cells(w, frac),
        3..=8 => format!("[{}]", bar_cells(w - 2, frac)),
        _ => format!("[{}] {:>3}%", bar_cells(w - 7, frac), pct),
    }
}

/// A dim label then the bar, accent while running and success when complete. `w` is the total width.
pub fn progress_line(theme: &Theme, g: &Glyphs, label: &str, done: u64, total: u64, w: usize) -> Line<'static> {
    let label = format!(" {label}  ");
    let bar_w = w.saturating_sub(width(&label)).max(w.min(9));
    let style = if total > 0 && done >= total { theme.success } else { theme.accent };
    fit(vec![(label, theme.dim), (progress_bar(done, total, bar_w), style)], w, g.ellipsis)
}

/// State of one step in a job.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepState {
    Pending,
    Running,
    Done,
    Failed,
}

/// One step: glyph and text. Done success, failed error, running accent with `spin`, pending dim.
pub fn step_line(theme: &Theme, g: &Glyphs, state: StepState, text: &str, spin: &str, w: usize) -> Line<'static> {
    let (glyph, style) = match state {
        StepState::Pending => (g.idle, theme.dim),
        StepState::Running => (spin, theme.accent),
        StepState::Done => (g.ok, theme.success),
        StepState::Failed => (g.fail, theme.error),
    };
    fit(vec![(format!(" {glyph} {text}"), style)], w, g.ellipsis)
}

/// A yes or no question with the default capitalized, like `› Allow shell access? [y/N]`.
pub fn confirm_line(theme: &Theme, g: &Glyphs, question: &str, default_yes: bool, w: usize) -> Line<'static> {
    let choice = if default_yes { "[Y/n]" } else { "[y/N]" };
    fit(
        vec![(format!(" {} ", g.prompt), theme.accent), (question.to_string(), theme.plain), (format!(" {choice}"), theme.dim)],
        w,
        g.ellipsis,
    )
}

/// A one-line question with its answer so far, or a dim hint when the answer is empty.
pub fn prompt_line(theme: &Theme, g: &Glyphs, label: &str, input: &str, hint: &str, w: usize) -> Line<'static> {
    let answer = if input.is_empty() { (hint.to_string(), theme.dim) } else { (input.to_string(), theme.plain) };
    fit(vec![(format!(" {} ", g.prompt), theme.accent), (format!("{label}  "), theme.plain), answer], w, g.ellipsis)
}

// ---------------------------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn text(line: &Line<'_>) -> String {
        line.spans.iter().map(|s| s.content.as_ref()).collect()
    }
    fn fg(style: Style) -> Option<ratatui::style::Color> {
        style.fg
    }
    fn rgb(r: u8, g: u8, b: u8) -> Option<ratatui::style::Color> {
        Some(ratatui::style::Color::Rgb(r, g, b))
    }

    #[test]
    fn no_color_beats_everything() {
        let t = Theme::detect(true, Appearance::Light, Some("0;15"), Some("truecolor"));
        assert_eq!(t, Theme::no_color());
        assert_eq!(fg(t.accent), None);
        assert!(t.accent.add_modifier.contains(Modifier::BOLD));
        assert!(t.warning.add_modifier.contains(Modifier::BOLD));
        assert!(t.error.add_modifier.contains(Modifier::BOLD));
        assert_eq!(t.dim, Style::default());
        assert_eq!(t.success, Style::default());
        assert!(t.selected.add_modifier.contains(Modifier::REVERSED));
    }

    #[test]
    fn truecolor_picks_light_or_dark() {
        let dark = Theme::detect(false, Appearance::Auto, None, Some("truecolor"));
        assert_eq!(fg(dark.accent), Some(DARK.accent));
        assert_eq!(fg(dark.special), Some(DARK.special));
        let light = Theme::detect(false, Appearance::Auto, Some("0;15"), Some("24bit"));
        assert_eq!(fg(light.accent), Some(LIGHT.accent));
        assert_eq!(fg(light.error), Some(LIGHT.error));
        // The flag wins over COLORFGBG in both directions.
        assert_eq!(fg(Theme::detect(false, Appearance::Dark, Some("0;15"), Some("truecolor")).accent), Some(DARK.accent));
        assert_eq!(fg(Theme::detect(false, Appearance::Light, Some("15;0"), Some("truecolor")).accent), Some(LIGHT.accent));
        assert_eq!(rgb(0xb8, 0xc6, 0xaa), Some(DARK.accent));
    }

    #[test]
    fn basic_fallback_without_truecolor() {
        use ratatui::style::Color;
        let dark = Theme::detect(false, Appearance::Auto, None, None);
        assert_eq!(fg(dark.accent), Some(Color::Cyan));
        assert_eq!(fg(dark.special), Some(Color::DarkGray));
        let light = Theme::detect(false, Appearance::Auto, Some("0;7"), Some(""));
        assert_eq!(fg(light.accent), Some(Color::Blue));
        assert_eq!(fg(light.warning), Some(Color::Magenta));
    }

    #[test]
    fn appearance_follows_flag_then_colorfgbg() {
        assert!(is_light(Appearance::Light, None));
        assert!(!is_light(Appearance::Dark, Some("0;15")));
        assert!(is_light(Appearance::Auto, Some("0;15")));
        assert!(is_light(Appearance::Auto, Some("0;default;15")));
        assert!(is_light(Appearance::Auto, Some("0;7")));
        assert!(!is_light(Appearance::Auto, Some("15;0")));
        assert!(!is_light(Appearance::Auto, Some("15;default")));
        assert!(!is_light(Appearance::Auto, None));
    }

    #[test]
    fn hints_shorten_at_narrow_widths() {
        let pairs = [("Enter", "send"), ("Ctrl+C", "stop/quit"), ("Ctrl+K", "steer"), ("Ctrl+S", "sessions")];
        let full = hints_text(&UNICODE, &pairs, 200);
        assert_eq!(full, " Enter send \u{b7} Ctrl+C stop/quit \u{b7} Ctrl+K steer \u{b7} Ctrl+S sessions");
        assert_eq!(hints_text(&UNICODE, &pairs, 40), " Enter send \u{b7} Ctrl+C stop/quit");
        assert_eq!(hints_text(&UNICODE, &pairs, 12), " Enter send");
        assert_eq!(hints_text(&UNICODE, &pairs, 6), " Ente\u{2026}");
        assert_eq!(hints_text(&UNICODE, &pairs, 0), "");
        assert_eq!(hints_text(&ASCII, &pairs, 40), " Enter send - Ctrl+C stop/quit");
        for w in 0..80 {
            assert!(width(&hints_text(&UNICODE, &pairs, w)) <= w);
        }
    }

    #[test]
    fn truncate_counts_columns_not_bytes() {
        assert_eq!(truncate("abcdef", 6, "\u{2026}"), "abcdef");
        assert_eq!(truncate("abcdef", 4, "\u{2026}"), "abc\u{2026}");
        assert_eq!(truncate("abcdef", 4, "..."), "a...");
        // Each wide character is two columns and is never split.
        assert_eq!(truncate("\u{754c}\u{754c}\u{754c}", 4, "\u{2026}"), "\u{754c}\u{2026}");
        assert_eq!(truncate("\u{754c}\u{754c}\u{754c}", 5, "\u{2026}"), "\u{754c}\u{754c}\u{2026}");
        assert_eq!(truncate("a\u{1f980}b", 3, "\u{2026}"), "a\u{2026}");
        assert_eq!(truncate("abc", 0, "\u{2026}"), "");
        assert_eq!(truncate("abcdef", 1, "..."), ".");
        for w in 0..10 {
            assert!(width(&truncate("\u{754c}a\u{754c}bc\u{1f980}", w, "\u{2026}")) <= w);
        }
    }

    #[test]
    fn progress_bar_bounds() {
        assert_eq!(progress_bar(0, 10, 20), "[-------------]   0%");
        assert_eq!(progress_bar(5, 10, 20), "[======>------]  50%");
        assert_eq!(progress_bar(10, 10, 20), "[=============] 100%");
        assert_eq!(progress_bar(99, 100, 20), "[============>]  99%");
        assert_eq!(progress_bar(50, 0, 20), "[-------------]   0%");
        assert_eq!(progress_bar(500, 10, 20), progress_bar(10, 10, 20));
        assert_eq!(progress_bar(0, 10, 1), "-");
        assert_eq!(progress_bar(5, 10, 1), ">");
        assert_eq!(progress_bar(10, 10, 1), "=");
        assert_eq!(progress_bar(5, 10, 0), "");
        assert_eq!(progress_bar(5, 10, 5), "[=>-]");
        for w in 0..60 {
            for (d, t) in [(0, 10), (1, 10), (5, 10), (9, 10), (10, 10), (3, 0)] {
                let bar = progress_bar(d, t, w);
                assert_eq!(bar.len(), w, "width {w} done {d}/{t}");
                assert!(bar.is_ascii());
            }
        }
    }

    #[test]
    fn glyphs_follow_locale_and_plain() {
        assert!(!is_ascii(glyph_set(false, Some("en_US.UTF-8"))));
        assert!(!is_ascii(glyph_set(false, Some("C.utf8"))));
        assert!(!is_ascii(glyph_set(false, Some("de_DE.UTF8"))));
        assert!(is_ascii(glyph_set(false, Some("C"))));
        assert!(is_ascii(glyph_set(false, Some("POSIX"))));
        assert!(is_ascii(glyph_set(false, Some("en_US.ISO-8859-1"))));
        assert!(is_ascii(glyph_set(false, Some(""))));
        assert!(is_ascii(glyph_set(false, None)));
        assert!(is_ascii(glyph_set(true, Some("en_US.UTF-8"))));
        assert_eq!(pick_locale(Some(""), Some("C"), Some("en_US.UTF-8")), Some("C"));
        assert_eq!(pick_locale(None, None, Some("en_US.UTF-8")), Some("en_US.UTF-8"));
        assert_eq!(pick_locale(Some(""), None, Some("")), None);
    }

    #[test]
    fn spinner_is_deterministic() {
        assert_eq!(spinner_frame(&UNICODE, 0, false), UNICODE.spinner[0]);
        assert_eq!(spinner_frame(&UNICODE, 79, false), UNICODE.spinner[0]);
        assert_eq!(spinner_frame(&UNICODE, 80, false), UNICODE.spinner[1]);
        assert_eq!(spinner_frame(&UNICODE, 800, false), UNICODE.spinner[0]);
        assert_eq!(spinner_frame(&ASCII, 240, false), "\\");
        assert_eq!(spinner_frame(&ASCII, 320, false), "|");
        assert_eq!(spinner_frame(&UNICODE, 12345, false), spinner_frame(&UNICODE, 12345, false));
        assert_eq!(spinner_frame(&UNICODE, 12345, true), "\u{25cf}");
        assert_eq!(spinner_frame(&ASCII, 12345, true), "*");
    }

    #[test]
    fn status_line_keeps_run_state_and_drops_counts() {
        let t = Theme::detect(false, Appearance::Dark, None, Some("truecolor"));
        let counts = ["1 agents".to_string(), "models 1+0q".to_string(), "tools 1+0q".to_string()];
        let l = status_line(&t, &UNICODE, &counts, RunState::Running, "Streaming\u{2026}", 80);
        assert_eq!(text(&l), " 1 agents \u{b7} models 1+0q \u{b7} tools 1+0q \u{b7} \u{25cf} Streaming\u{2026}");
        assert_eq!(l.spans[0].style, t.accent);
        let l = status_line(&t, &UNICODE, &counts, RunState::Running, "Streaming\u{2026}", 34);
        assert_eq!(text(&l), " 1 agents \u{b7} \u{25cf} Streaming\u{2026}");
        let l = status_line(&t, &UNICODE, &counts, RunState::Idle, "Ready", 10);
        assert_eq!(text(&l), " Ready");
        assert_eq!(l.spans[0].style, t.dim);
        let l = status_line(&t, &ASCII, &counts, RunState::Stopping, "", 80);
        assert!(text(&l).ends_with("* Stopping..."));
        let l = status_line(&t, &UNICODE, &[], RunState::ScrolledUp(12), "", 80);
        assert_eq!(text(&l), " \u{2191} 12 lines above latest \u{b7} End to follow");
        assert!(text(&status_line(&t, &UNICODE, &[], RunState::ScrolledUp(1), "", 80)).contains("1 line above"));
        for w in 0..60 {
            assert!(width(&text(&status_line(&t, &UNICODE, &counts, RunState::Running, "Streaming\u{2026}", w))) <= w);
        }
    }

    #[test]
    fn header_truncates_and_marks_grants() {
        let t = Theme::detect(false, Appearance::Dark, None, Some("truecolor"));
        let h = Header { app: "ejm-cli", provider: "deepseek-chat", title: "Fix the flaky retry test", grants: "write", elevated: true, cwd: "~/code/harbor", session_id: "s_4f2a91xyz" };
        let [a, b] = header_lines(&t, &UNICODE, &h, 80);
        assert_eq!(text(&a), " ejm-cli deepseek-chat  \u{b7}  Fix the flaky retry test");
        assert_eq!(text(&b), " grants: write  \u{b7}  ~/code/harbor  \u{b7}  s_4f2a91");
        assert_eq!(b.spans[0].style, t.warning);
        let [a, b] = header_lines(&t, &UNICODE, &Header { elevated: false, ..h }, 20);
        assert!(width(&text(&a)) <= 20 && text(&a).ends_with('\u{2026}'));
        assert!(width(&text(&b)) <= 20);
        assert_eq!(b.spans[0].style, t.dim);
    }

    #[test]
    fn wrapping_keeps_indent_and_wide_chars() {
        assert_eq!(wrap_text("one two three", 7), ["one two", "three"]);
        assert_eq!(wrap_text("  - one two three", 9), ["  - one", "  two", "  three"]);
        assert_eq!(wrap_text("abcdefghij", 4), ["abcd", "efgh", "ij"]);
        assert_eq!(wrap_text("", 10), [""]);
        assert_eq!(wrap_chars("a\u{754c}b\u{1f980}", 4), ["a\u{754c}b", "\u{1f980}"]);
        for w in 1..30 {
            for l in wrap_text("The test reads the wall clock, so I will inject one. \u{754c}\u{754c}\u{754c} supercalifragilistic", w) {
                assert!(width(&l) <= w.max(2), "{l:?} at {w}");
            }
        }
    }

    #[test]
    fn transcript_block_styles_diff_and_fences() {
        let t = Theme::detect(false, Appearance::Dark, None, Some("truecolor"));
        let body = "Reading it.\n```diff\n--- a/x\n+++ b/x\n@@ -1 +1 @@\n-old\n+new\n context\n```\n# Heading";
        let lines = transcript_block(&t, BlockKind::Assistant, "ejm-cli", body, 40);
        let styles: Vec<_> = lines.iter().map(|l| (text(l), l.style)).collect();
        assert_eq!(styles[0], ("ejm-cli".into(), t.bold));
        assert_eq!(styles[1].1, t.plain);
        let by_text = |s: &str| styles.iter().find(|(x, _)| x == s).unwrap().1;
        assert_eq!(by_text("```diff"), t.dim);
        assert_eq!(by_text("--- a/x"), t.dim);
        assert_eq!(by_text("+++ b/x"), t.dim);
        assert_eq!(by_text("@@ -1 +1 @@"), t.special);
        assert_eq!(by_text("-old"), t.error);
        assert_eq!(by_text("+new"), t.success);
        assert_eq!(by_text(" context"), t.dim);
        assert_eq!(by_text("# Heading"), t.accent.add_modifier(Modifier::BOLD));
        assert_eq!(text(lines.last().unwrap()), "");
        // A "-" line outside a diff is prose, not a removal.
        let lines = transcript_block(&t, BlockKind::Assistant, "ejm-cli", "- a list item", 40);
        assert_eq!(lines[1].style, t.plain);
    }

    #[test]
    fn diff_lines() {
        let t = Theme::detect(false, Appearance::Dark, None, Some("truecolor"));
        assert_eq!(diff_line(&t, "+a").style, t.success);
        assert_eq!(diff_line(&t, "-a").style, t.error);
        assert_eq!(diff_line(&t, "@@ -1 +1 @@").style, t.special);
        assert_eq!(diff_line(&t, "+++ b").style, t.dim);
        assert_eq!(diff_line(&t, " a").style, t.plain);
        // Without color, hunk headers are plain and +/- rely on their first character.
        let n = Theme::no_color();
        assert_eq!(diff_line(&n, "@@ -1 +1 @@").style, Style::default());
        assert_eq!(text(&diff_line(&n, "+a")), "+a");
    }

    #[test]
    fn banner_empty_and_error_states() {
        let t = Theme::detect(false, Appearance::Dark, None, Some("truecolor"));
        let b = banner(&t, &UNICODE, BannerKind::Notice, "shell access is on for this session", 20);
        assert_eq!(b.iter().map(text).collect::<Vec<_>>(), ["! shell access is on", "  for this session"]);
        assert_eq!(b[0].style, t.warning);
        let b = banner(&t, &ASCII, BannerKind::Error, "Request failed", 40);
        assert_eq!(text(&b[0]), "x Request failed");
        assert_eq!(b[0].style, t.error);
        let e = empty_state(&t, "What shall we build?", &["Ask about this repository."], 40);
        assert_eq!(text(&e[1]), "  What shall we build?");
        assert_eq!(e[3].style, t.dim);
        let e = error_state(&t, "Provider request failed: 401", "Check the key, then retry.", 80);
        assert_eq!(e.len(), 2);
        assert!(e.iter().all(|l| l.style == t.error));
    }

    #[test]
    fn rows_tabs_kv_table() {
        let t = Theme::detect(false, Appearance::Dark, None, Some("truecolor"));
        let sel = list_row(&t, &UNICODE, "a1b2c3d4", "[idle]  Fix retry", true, 30);
        assert_eq!(width(&text(&sel)), 30);
        assert_eq!(sel.style, t.selected);
        let plain = list_row(&t, &UNICODE, "a1b2c3d4", "[idle]  Fix retry", false, 30);
        assert_eq!(text(&plain), " a1b2c3d4  [idle]  Fix retry");
        assert_eq!(text(&list_row(&t, &UNICODE, "a1b2c3d4", "[idle]  Fix retry", false, 14)), " a1b2c3d4  [i\u{2026}");
        let tabs = tabs_line(&t, &UNICODE, &["Sessions", "Models"], 0, 40);
        assert_eq!(text(&tabs), "  Sessions   Models ");
        assert_eq!(tabs.spans[1].style, t.selected);
        assert_eq!(tabs.spans[3].style, t.dim);
        let kv = kv_line(&t, &UNICODE, "Enter", "Send, or queue", 8, 40);
        assert_eq!(text(&kv), "Enter     Send, or queue");
        assert_eq!(text(&kv_line(&t, &UNICODE, "Enter", "Send, or queue", 8, 12)), "Enter     S\u{2026}");
        let row = table_row(&UNICODE, &[("harbor", 8, Align::Left), ("1234", 6, Align::Right), ("a very long name", 8, Align::Left)], t.plain);
        assert_eq!(text(&row), " harbor      1234  a very \u{2026}");
    }

    #[test]
    fn prompts_steps_and_progress_line() {
        let t = Theme::detect(false, Appearance::Dark, None, Some("truecolor"));
        assert_eq!(text(&confirm_line(&t, &UNICODE, "Allow shell access?", false, 60)), " \u{203a} Allow shell access? [y/N]");
        assert_eq!(text(&confirm_line(&t, &ASCII, "Continue?", true, 60)), " > Continue? [Y/n]");
        assert_eq!(text(&prompt_line(&t, &UNICODE, "Model", "", "deepseek-chat", 60)), " \u{203a} Model  deepseek-chat");
        assert_eq!(prompt_line(&t, &UNICODE, "Model", "", "x", 60).spans[2].style, t.dim);
        assert_eq!(prompt_line(&t, &UNICODE, "Model", "gpt", "x", 60).spans[2].style, t.plain);
        assert_eq!(text(&step_line(&t, &UNICODE, StepState::Done, "Fetched 12 packages", "", 60)), " \u{2713} Fetched 12 packages");
        assert_eq!(step_line(&t, &UNICODE, StepState::Failed, "x", "", 60).spans[0].style, t.error);
        assert_eq!(text(&step_line(&t, &ASCII, StepState::Pending, "Link", "", 60)), " o Link");
        let p = progress_line(&t, &UNICODE, "Installing", 5, 10, 40);
        assert_eq!(width(&text(&p)), 40);
        assert_eq!(p.spans[1].style, t.accent);
        assert_eq!(progress_line(&t, &UNICODE, "Installing", 10, 10, 40).spans[1].style, t.success);
    }

    #[test]
    fn blocks_and_layout() {
        let t = Theme::detect(false, Appearance::Dark, None, Some("truecolor"));
        let _ = composer_block(&t, &UNICODE, true);
        let _ = composer_block(&t, &ASCII, false);
        let _ = titled_block(&ASCII, "sessions", t.plain);
        let empty = composer_rows(&t, &ASCII, &[String::new()], true);
        assert_eq!(text(&empty[0]), "> Queue a follow-up, or Ctrl+K to steer...");
        let typed = composer_rows(&t, &UNICODE, &["one".to_string(), "two".to_string()], false);
        assert_eq!((text(&typed[0]), text(&typed[1])), ("\u{203a} one".to_string(), "  two".to_string()));
        assert!(too_small(Rect::new(0, 0, 19, 24)));
        assert!(too_small(Rect::new(0, 0, 80, 7)));
        assert!(!too_small(Rect::new(0, 0, 20, 8)));
        assert_eq!(too_small_lines("ejm-cli", 20).iter().map(text).collect::<Vec<_>>(), ["Enlarge the terminal", "to use ejm-cli."]);
        let p = popup(Rect::new(0, 0, 100, 30), 86, 20);
        assert_eq!((p.x, p.y, p.width, p.height), (7, 5, 86, 20));
        let p = popup(Rect::new(0, 0, 20, 8), 86, 20);
        assert_eq!((p.width, p.height), (18, 6));
        let s = "a\u{754c}b\u{1f980}";
        let (rows, row, col) = composer_layout(s, s.len(), 4);
        assert_eq!(rows, ["a\u{754c}b", "\u{1f980}"]);
        assert_eq!((row, col), (1, 2));
        assert_eq!(composer_layout("", 0, 10), (vec![String::new()], 0, 0));
        assert_eq!(composer_layout("ab\ncd", 3, 10).1, 1);
    }
}
