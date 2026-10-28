//! The TUI screen, composed from the shared ejmlabs terminal blocks in `ejm-tui`.
//!
//! `WorkspaceLayout` places the regions. The transcript draws each block as an `ejm-tui`
//! `Message` or `ToolCall`, the working line is an `ActivityLine`, the counts a `ResourceLine`,
//! and the footer `workspace_controls`. Nothing here draws a box: groups are separated by space
//! and one hairline `Rule`, as the shared contract requires.

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Margin, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Clear, Paragraph},
};
use unicode_width::UnicodeWidthChar;

use ejm_tui::{
    ActivityLine, CommandMenu, Markdown, Message, MessageKind, ResourceCount, ResourceLine, Row,
    RowState, Rule, SessionHeader, Status, Text, TextVariant, Theme, ToolCall, WorkspaceLayout,
    elapsed_text, truncate, workspace_controls,
};

use crate::{
    app::{App, BlockKind, Overlay, RunState, ToolDetail, TranscriptBlock},
    args::Args,
    extensions::Palette,
};

/// Smallest supported terminal. Below it only the enlarge notice shows.
const MIN_WIDTH: u16 = 20;
const MIN_HEIGHT: u16 = 8;
/// The key that shows complete tool output, named in collapsed previews.
const EXPAND_KEY: &str = "Ctrl+O";
/// Columns left blank on each side, so every region shares one left and one right edge.
const MARGIN: u16 = 2;

/// A package theme (`--theme`) replaces the shared colors for the roles it names.
fn themed(base: &Theme, palette: Palette) -> Theme {
    let fg = |color| Style::default().fg(color);
    Theme {
        accent: fg(palette.accent),
        muted: fg(palette.dim),
        code: fg(palette.dim),
        pending: fg(palette.dim),
        rule: fg(palette.dim),
        warn: fg(palette.warning),
        error: fg(palette.error),
        ok: fg(palette.positive),
        ..*base
    }
}

pub fn draw(
    frame: &mut Frame<'_>,
    app: &mut App,
    args: &Args,
    snapshot: &ejm_harness::HarnessSnapshot,
) {
    let base = Theme::from_env(args.no_color, args.appearance.into(), args.plain);
    let no_color = args.no_color || std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty());
    let theme = match app.palette {
        Some(palette) if !no_color => themed(&base, palette),
        _ => base,
    };
    draw_screen(frame, app, args, snapshot, &theme);
}

fn draw_screen(
    frame: &mut Frame<'_>,
    app: &mut App,
    args: &Args,
    snapshot: &ejm_harness::HarnessSnapshot,
    theme: &Theme,
) {
    let area = frame.area();
    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        frame.render_widget(
            Text::new(theme, "Enlarge the terminal to use ejm-cli."),
            area,
        );
        return;
    }
    let area = area.inner(Margin::new(MARGIN, 0));
    let spin =
        app.run_started
            .filter(|_| app.is_running())
            .map_or(theme.glyphs.running, |started| {
                let elapsed = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
                theme.spinner(elapsed, args.plain)
            });
    // The prompt glyph and its space take two columns.
    let draft_width = usize::from(area.width.saturating_sub(2).max(1));
    let draft = composer_layout(&app.composer.text, app.composer.cursor, draft_width);
    let layout = WorkspaceLayout::new(area, draft.0.len(), false, true, app.widget_lines.len());

    frame.render_widget(
        Paragraph::new(header_line(app, args, theme, layout.header.width)),
        layout.header,
    );
    let transcript = layout.transcript;
    if let Some(overlay) = &app.overlay {
        draw_overlay(frame, app, overlay, theme, transcript);
    } else {
        let setup = args
            .setup_needed
            .as_deref()
            .map(|reason| (reason, args.api_key_env.as_str()));
        draw_transcript(frame, app, theme, spin, setup, transcript);
        draw_commands(frame, app, theme, transcript);
    }
    let resources = ResourceLine {
        agents: snapshot.active_agents,
        models: ResourceCount {
            active: snapshot.model_requests,
            queued: snapshot.waiting_model_requests,
        },
        tools: ResourceCount {
            active: snapshot.tools.running,
            queued: snapshot.tools.queued,
        },
    };
    // Counts share the working line, right aligned, and only while something is active or queued.
    let counts = resources.line(theme, layout.activity.width / 2);
    let counts_width = match counts.width() {
        0 => 0,
        columns => u16::try_from(columns + 2).unwrap_or(0),
    };
    let [work, count_area] =
        Layout::horizontal([Constraint::Min(0), Constraint::Length(counts_width)])
            .areas(layout.activity);
    frame.render_widget(Paragraph::new(activity(app, theme, spin, work.width)), work);
    frame.render_widget(
        Paragraph::new(counts).alignment(Alignment::Right),
        count_area,
    );
    frame.render_widget(
        Paragraph::new(
            app.widget_lines
                .iter()
                .map(|line| {
                    Line::styled(
                        truncate(line, usize::from(area.width), theme.glyphs.ellipsis),
                        theme.muted,
                    )
                })
                .collect::<Vec<_>>(),
        ),
        layout.notices,
    );
    draw_composer(frame, app, theme, layout.composer, draft);
    frame.render_widget(
        Paragraph::new(controls(app, args, theme, layout.controls.width)),
        layout.controls,
    );
}

fn is_controller(app: &App) -> bool {
    app.remote.as_ref().is_some_and(|remote| remote.controller)
}

fn grants(app: &App, args: &Args) -> (String, bool) {
    if is_controller(app) {
        ("host grants apply".into(), false)
    } else {
        (args.grants(), args.allow_shell || args.allow_write)
    }
}

fn header_line(app: &App, args: &Args, theme: &Theme, width: u16) -> Line<'static> {
    let sep = theme.glyphs.separator;
    let provider = if is_controller(app) {
        "remote session"
    } else if app.connection_ready && args.setup_needed.is_none() {
        args.provider_label()
    } else {
        "not connected"
    };
    let mut connection = safe_text(provider);
    if let Some(remote) = &app.remote {
        connection = format!("{} {sep} {connection}", safe_text(&remote.label));
    }
    SessionHeader {
        name: "ejm-cli",
        version: env!("CARGO_PKG_VERSION"),
        connection: &connection,
        directory: &safe_text(&display_path(&app.session.cwd)),
    }
    .line(theme, width)
}

/// Home directory shown as `~`, so the part of the path that matters fits.
fn display_path(path: &std::path::Path) -> String {
    let text = path.display().to_string();
    std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .and_then(|home| {
            path.strip_prefix(&home)
                .ok()
                .map(std::path::Path::to_path_buf)
        })
        .map_or(text, |rest| {
            if rest.as_os_str().is_empty() {
                "~".into()
            } else {
                format!("~/{}", rest.display())
            }
        })
}

/// One transcript block as shared `Message` or `ToolCall` lines, then a blank line.
fn block_lines(
    theme: &Theme,
    block: &TranscriptBlock,
    detail: ToolDetail,
    spin: &str,
    width: u16,
) -> Vec<Line<'static>> {
    let mut lines = if let BlockKind::Summary(level) = block.kind {
        // `✓ completed · 3 steps · 4s` under the reply's text column: the result glyph and
        // word, then muted details.
        let mut line = Status::new(theme, level)
            .label(&block.title)
            .line(width.saturating_sub(2));
        line.spans.insert(0, Span::raw("  "));
        let room = usize::from(width).saturating_sub(line.width());
        let details = format!(" {} {}", theme.glyphs.separator, block.text);
        line.spans.push(Span::styled(
            truncate(&details, room, theme.glyphs.ellipsis),
            theme.muted,
        ));
        vec![line]
    } else if let Some(view) = &block.tool {
        ToolCall::new(theme, view.state, &view.name)
            .detail(&view.detail)
            .output(&block.text)
            .expanded(detail == ToolDetail::Full)
            .expand_key(EXPAND_KEY)
            .spin(spin)
            .lines(width)
    } else {
        let kind = match block.kind {
            BlockKind::User => MessageKind::User,
            BlockKind::Assistant => MessageKind::Assistant,
            BlockKind::Notice | BlockKind::Tool | BlockKind::Summary(_) => MessageKind::Notice,
            BlockKind::Error => MessageKind::Error,
        };
        let message = Message::new(theme, kind, &block.text);
        if matches!(block.title.as_str(), "ejm-cli" | "error" | "") {
            message.lines(width)
        } else {
            message.title(&block.title).lines(width)
        }
    };
    lines.push(Line::default());
    lines
}

/// The welcome, or while no provider is configured, the reason and the ways to connect.
/// `setup` is the reason and the API key variable.
fn empty_state(theme: &Theme, setup: Option<(&str, &str)>, width: u16) -> Vec<Line<'static>> {
    let (title, body, note) = match setup {
        Some((reason, key)) => (
            "Connect a model to start",
            format!(
                "- Sign in with ChatGPT: `ejm-cli login chatgpt`, then `ejm-cli --provider chatgpt`\n\
                 - Use an API key: set `{key}`, plus `EJM_CLI_MODEL` and `EJM_CLI_BASE_URL` for \
                 other providers, then restart\n\
                 - Try it offline: `ejm-cli --replay`"
            ),
            format!("Reason: {}", safe_text(reason)),
        ),
        None => (
            "What shall we build?",
            "Ask about this repository, investigate a bug, or describe a change.".into(),
            "Ctrl+S opens saved sessions. --allow-write and --allow-shell set what tools may do."
                .into(),
        ),
    };
    let mut lines = Text::new(theme, title)
        .variant(TextVariant::Strong)
        .lines(width);
    lines.push(Line::default());
    lines.extend(Markdown::new(theme, &body).lines(width));
    lines.push(Line::default());
    lines.extend(
        Text::new(theme, &note)
            .variant(TextVariant::Muted)
            .lines(width),
    );
    lines.push(Line::default());
    lines
}

fn draw_transcript(
    frame: &mut Frame<'_>,
    app: &mut App,
    theme: &Theme,
    spin: &str,
    setup: Option<(&str, &str)>,
    area: Rect,
) {
    let inner = area;
    let available = usize::from(inner.height);
    // Wrap from the newest block back, only as far as the screen and scroll offset need, so a
    // long transcript costs no more per frame than a short one.
    let wanted = available + app.scroll_from_bottom;
    let mut chunks = Vec::new();
    let mut total = 0;
    for block in app.blocks.iter().rev() {
        let lines = block_lines(theme, block, app.tool_detail, spin, inner.width);
        total += lines.len();
        chunks.push(lines);
        if total >= wanted {
            break;
        }
    }
    if total < wanted {
        // While not connected the steps stay above any notices, since no run can push them away.
        if app.blocks.is_empty() || setup.is_some() {
            let lines = empty_state(theme, setup, inner.width);
            total += lines.len();
            chunks.push(lines);
        }
        app.scroll_from_bottom = app.scroll_from_bottom.min(total.saturating_sub(available));
    }
    let mut lines = chunks.into_iter().rev().flatten().collect::<Vec<_>>();
    let end = lines.len().saturating_sub(app.scroll_from_bottom);
    let start = end.saturating_sub(available);
    frame.render_widget(
        Paragraph::new(lines.drain(start..end).collect::<Vec<_>>()),
        inner,
    );
}

/// Matching commands at the bottom of the transcript, just above the composer.
fn draw_commands(frame: &mut Frame<'_>, app: &App, theme: &Theme, area: Rect) {
    if app.command_query().is_none() {
        return;
    }
    let matches = app.command_matches();
    let usages = matches
        .iter()
        .map(|entry| entry.usage())
        .collect::<Vec<_>>();
    let items = matches
        .iter()
        .zip(&usages)
        .map(|(entry, usage)| (usage.as_str(), entry.description.as_str()))
        .collect::<Vec<_>>();
    let menu = CommandMenu::new(theme, &items).selected(app.command_picker.selected);
    // One cleared row above the menu sets it apart from the transcript it covers.
    let height = menu.height().min(8).min(area.height.saturating_sub(2));
    if height == 0 {
        return;
    }
    let cleared = Rect::new(area.x, area.bottom() - height - 1, area.width, height + 1);
    frame.render_widget(Clear, cleared);
    frame.render_widget(menu, Rect::new(area.x, cleared.y + 1, area.width, height));
}

/// The working line: spinner, step, elapsed time and the stop key while a run is active; the
/// last result when idle; and where the view is when scrolled up.
fn activity(app: &App, theme: &Theme, spin: &str, width: u16) -> Line<'static> {
    let sep = format!(" {} ", theme.glyphs.separator);
    let status = safe_text(&app.status).replace('\u{b7}', theme.glyphs.separator);
    let scrolled = (app.scroll_from_bottom > 0).then(|| {
        let n = app.scroll_from_bottom;
        format!(
            "{} {n} line{} above latest",
            theme.glyphs.up,
            if n == 1 { "" } else { "s" }
        )
    });
    match app.run_state {
        RunState::Running => {
            // "Thinking · step 2" reads as "Thinking… (step 2 · 12s · Esc to stop)".
            let (word, progress) = status.split_once(&sep).unwrap_or((&status, ""));
            let elapsed = app
                .run_started
                .map(|started| elapsed_text(started.elapsed().as_secs()))
                .unwrap_or_default();
            let scrolled = scrolled.unwrap_or_default();
            ActivityLine {
                label: word,
                active: true,
                details: &[progress, &elapsed, "Esc to stop", &scrolled],
                spin: Some(spin),
            }
            .line(theme, width)
        }
        RunState::Stopping => ActivityLine {
            label: "Stopping",
            active: true,
            ..ActivityLine::default()
        }
        .line(theme, width),
        RunState::Idle => {
            let label = match scrolled {
                Some(scrolled) => format!("{scrolled}{sep}End to follow"),
                None if status == "Ready" => String::new(),
                None => status,
            };
            ActivityLine {
                label: &label,
                ..ActivityLine::default()
            }
            .line(theme, width)
        }
    }
}

/// Grants at the left, then the few keys that apply right now at the right edge.
fn controls(app: &App, args: &Args, theme: &Theme, width: u16) -> Line<'static> {
    let arrows = if ejm_tui::is_ascii(theme.glyphs) {
        "Up/Down"
    } else {
        "\u{2191}/\u{2193}"
    };
    let picker = [(arrows, "choose"), ("Tab", "complete"), ("Esc", "close")];
    let hints: &[(&str, &str)] = if app.quit_armed.is_some() {
        &[("Ctrl+C", "again to quit")]
    } else if app.command_query().is_some() {
        &picker
    } else if app.is_running() {
        &[("Esc", "stop"), ("Ctrl+K", "steer"), ("Enter", "queue")]
    } else if !app.composer.text.is_empty() {
        &[("Enter", "send"), ("Alt+Enter", "new line")]
    } else if app.tool_detail == ToolDetail::Full {
        // Full output is a mode, so its way back stays named.
        &[(EXPAND_KEY, "hide details"), ("?", "shortcuts")]
    } else {
        // Collapsed previews name Ctrl+O and the welcome names Ctrl+S; ? lists every key.
        &[("/", "commands"), ("?", "shortcuts")]
    };
    let (mode, elevated) = grants(app, args);
    workspace_controls(theme, &format!("grants: {mode}"), elevated, hints, width)
}

/// Wrap composer text to `w` columns by character, so the cursor math stays exact. Returns the
/// rows, then the cursor row and column for byte index `cursor`.
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
        } else if let Some(row) = rows.last_mut() {
            row.push(c);
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

/// A hairline, then the draft under the prompt glyph. The terminal cursor marks the insertion
/// point, so input methods and screen readers follow it.
fn draw_composer(
    frame: &mut Frame<'_>,
    app: &App,
    theme: &Theme,
    area: Rect,
    draft: (Vec<String>, usize, usize),
) {
    let (rows, cursor_row, cursor_col) = draft;
    let rule = if app.is_running() {
        Rule::new(theme).label("Enter queues").line(area.width)
    } else {
        Rule::new(theme).line(area.width)
    };
    let visible = usize::from(area.height.saturating_sub(1)).max(1);
    let first_row = cursor_row.saturating_sub(visible - 1);
    let prompt = Span::styled(
        format!("{} ", theme.glyphs.prompt),
        theme.accent.add_modifier(Modifier::BOLD),
    );
    let mut lines = vec![rule];
    if app.composer.text.is_empty() {
        let hint = if app.is_running() {
            "Queue a follow-up, or Ctrl+K to steer"
        } else {
            "Describe a task"
        };
        lines.push(Line::from(vec![
            prompt,
            Span::styled(format!("{hint}{}", theme.glyphs.ellipsis), theme.muted),
        ]));
    } else {
        for (index, row) in rows.iter().enumerate().skip(first_row).take(visible) {
            let lead = if index == 0 {
                prompt.clone()
            } else {
                Span::raw("  ")
            };
            lines.push(Line::from(vec![
                lead,
                Span::styled(row.clone(), theme.text),
            ]));
        }
    }
    frame.render_widget(Paragraph::new(lines), area);
    if app.overlay.is_none() {
        let width = usize::from(area.width.saturating_sub(2).max(1));
        let x = area.x + 2 + u16::try_from(cursor_col.min(width.saturating_sub(1))).unwrap_or(0);
        let y = area.y + 1 + u16::try_from(cursor_row.saturating_sub(first_row)).unwrap_or(0);
        if y < area.bottom() {
            frame.set_cursor_position((x, y));
        }
    }
}

/// Help and sessions take over the transcript area under a labelled hairline. No popup box.
fn draw_overlay(frame: &mut Frame<'_>, app: &App, overlay: &Overlay, theme: &Theme, area: Rect) {
    frame.render_widget(Clear, area);
    let sep = theme.glyphs.separator;
    let arrows = if ejm_tui::is_ascii(theme.glyphs) {
        "Up/Down"
    } else {
        "\u{2191}/\u{2193}"
    };
    let width = area.width;
    let mut lines = Vec::new();
    let selected = match overlay {
        Overlay::Help { selected } => {
            lines.push(
                Rule::new(theme)
                    .label(&format!(
                        "Shortcuts {sep} {arrows} choose a command {sep} Enter use {sep} Esc close"
                    ))
                    .line(width),
            );
            for (key, action) in [
                ("Enter", "Send, or queue while a run is active"),
                ("Alt+Enter", "Insert a newline (Ctrl+J also works)"),
                ("Ctrl+K", "Steer the active run with your draft"),
                ("Esc", "Stop the active run, or follow the latest output"),
                (
                    "Ctrl+C",
                    "Stop a run, clear the draft, or press twice to exit",
                ),
                (EXPAND_KEY, "Show or hide complete tool output"),
                (
                    arrows,
                    "Recall sent messages; ? on an empty line opens this",
                ),
                ("PageUp/PageDown", "Scroll transcript; End follows output"),
            ]
            .into_iter()
            // Short windows keep at least half the rows for the command list.
            .take(usize::from(area.height).saturating_sub(4) / 2)
            {
                lines.push(
                    Row::new(theme, key)
                        .label_width(16)
                        .value(action)
                        .line(width),
                );
            }
            lines.push(Line::default());
            lines.push(Rule::new(theme).label("Commands").line(width));
            *selected
        }
        Overlay::Sessions { selected, .. } => {
            lines.push(
                Rule::new(theme)
                    .label(&format!(
                        "Sessions {sep} {arrows} choose {sep} Enter open {sep} Esc close"
                    ))
                    .line(width),
            );
            lines.push(Line::default());
            *selected
        }
        Overlay::Status { .. } => {
            lines.push(
                Rule::new(theme)
                    .label(&format!("Status {sep} Esc close"))
                    .line(width),
            );
            lines.push(Line::default());
            0
        }
    };
    let header = lines.len();
    let rows = overlay_rows(app, overlay, theme, selected, width);
    // Keep the selected row in view.
    let room = usize::from(area.height).saturating_sub(header).max(1);
    let start = (selected + 1).saturating_sub(room);
    lines.extend(rows.into_iter().skip(start).take(room));
    frame.render_widget(Paragraph::new(lines), area);
}

/// The selectable rows of an overlay: commands for help, saved sessions for sessions.
fn overlay_rows(
    app: &App,
    overlay: &Overlay,
    theme: &Theme,
    selected: usize,
    width: u16,
) -> Vec<Line<'static>> {
    match overlay {
        Overlay::Help { .. } => {
            let usages = app
                .commands
                .iter()
                .map(crate::commands::CommandEntry::usage)
                .collect::<Vec<_>>();
            let items = app
                .commands
                .iter()
                .zip(&usages)
                .map(|(entry, usage)| (usage.as_str(), entry.description.as_str()))
                .collect::<Vec<_>>();
            CommandMenu::new(theme, &items)
                .selected(selected)
                .lines(width)
        }
        Overlay::Status { rows } => rows
            .iter()
            .map(|(label, value)| {
                Row::new(theme, label)
                    .label_width(12)
                    .value(&safe_text(value))
                    .line(width)
            })
            .collect(),
        Overlay::Sessions { sessions, .. } => sessions
            .iter()
            .enumerate()
            .map(|(index, session)| {
                // Idle is the usual state, so only the others are named.
                let status = match session.status {
                    ejm_harness::SessionStatus::Idle => "",
                    ejm_harness::SessionStatus::Running => "running",
                    ejm_harness::SessionStatus::Interrupted => "interrupted",
                };
                let current = if session.id == app.session.id {
                    " (current)"
                } else {
                    ""
                };
                let title = format!("{}{current}", safe_text(&session.title));
                let meta = [status, &age(session.created_at_ms)]
                    .into_iter()
                    .filter(|part| !part.is_empty())
                    .collect::<Vec<_>>()
                    .join(&format!(" {sep} ", sep = theme.glyphs.separator));
                Row::new(theme, session.id.get(..8).unwrap_or(&session.id))
                    .label_width(8)
                    .value(&title)
                    .meta(&meta)
                    .state(if index == selected {
                        RowState::Selected
                    } else {
                        RowState::Default
                    })
                    .line(width)
            })
            .collect(),
    }
}

/// How long ago a session was created, such as `5m ago` or `3d ago`.
fn age(created_at_ms: u64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| {
            u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX)
        });
    if created_at_ms == 0 || created_at_ms > now {
        return String::new();
    }
    let minutes = (now - created_at_ms) / 60_000;
    match minutes {
        0 => "just now".into(),
        1..60 => format!("{minutes}m ago"),
        60..1440 => format!("{}h ago", minutes / 60),
        _ => format!("{}d ago", minutes / 1440),
    }
}

// Provider and tool output must never become terminal escape sequences.
pub fn safe_text(text: &str) -> String {
    let mut plain = PlainText(String::with_capacity(text.len()));
    vte::Parser::new().advance(&mut plain, text.as_bytes());
    plain.0
}

struct PlainText(String);

impl vte::Perform for PlainText {
    fn print(&mut self, value: char) {
        if !value.is_control()
            && !matches!(value, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
        {
            self.0.push(value);
        }
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            b'\n' => self.0.push('\n'),
            b'\t' => self.0.push_str("    "),
            _ => {}
        }
    }
}

#[cfg(test)]
#[expect(
    clippy::unwrap_used,
    reason = "Rendering fixtures must fail this test when initialization fails."
)]
mod tests {
    use super::*;
    use clap::Parser;
    use ejm_tui::{Appearance, theme::ASCII};
    use ratatui::{Terminal, backend::TestBackend, buffer::Buffer, style::Color};

    #[test]
    fn strips_terminal_control_sequences_and_preserves_readable_text() {
        assert_eq!(
            safe_text("\x1b[31mred\x1b[0m\x1b]52;c;secret\x07\r\nline\tend"),
            "red\nline    end"
        );
        assert!(!safe_text("a\u{009b}b\u{202e}c").contains(char::is_control));
        assert_eq!(
            safe_text("\x1bPqprivate\x1b\\ok\x1b_hidden\x1b\\end"),
            "okend"
        );
        assert_eq!(
            safe_text(&format!("\x1b]52;c;{}\x07visible", "x".repeat(64 * 1024))),
            "visible"
        );
    }

    #[test]
    fn appearance_flag_maps_onto_the_shared_appearance() {
        assert_eq!(
            Appearance::from(crate::args::Appearance::Light),
            Appearance::Light
        );
        assert_eq!(
            Appearance::from(crate::args::Appearance::Dark),
            Appearance::Dark
        );
        assert_eq!(
            Appearance::from(crate::args::Appearance::Auto),
            Appearance::Auto
        );
    }

    #[test]
    fn package_theme_replaces_the_named_roles() {
        let red = Color::Rgb(255, 0, 0);
        let theme = themed(
            &Theme::no_color(),
            Palette {
                accent: red,
                dim: red,
                warning: red,
                error: red,
                positive: Color::Green,
            },
        );
        assert_eq!(theme.accent.fg, Some(red));
        assert_eq!(theme.ok.fg, Some(Color::Green));
        assert_eq!(theme.text, Style::default());
    }

    #[test]
    fn composer_counts_wide_characters_and_wrapped_cursor() {
        let (rows, row, col) = composer_layout("a界b🦀", "a界b🦀".len(), 4);
        assert_eq!(rows, ["a界b", "🦀"]);
        assert_eq!((row, col), (1, 2));
    }

    fn app() -> App {
        let session = serde_json::from_value(serde_json::json!({
            "id": "123456789", "title": "test", "cwd": "/tmp", "created_at_ms": 0,
            "status": "idle"
        }))
        .unwrap();
        App::new(session)
    }

    fn render_app(args: &Args, theme: &Theme, width: u16, height: u16, mut app: App) -> Buffer {
        let governor =
            ejm_harness_exec::Governor::new(ejm_harness_exec::ResourceLimits::default()).unwrap();
        let snapshot = ejm_harness::HarnessSnapshot {
            active_agents: 12,
            model_requests: 2,
            waiting_model_requests: 3,
            tools: governor.snapshot(),
        };
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|frame| draw_screen(frame, &mut app, args, &snapshot, theme))
            .unwrap();
        terminal.backend().buffer().clone()
    }

    fn text(buffer: &Buffer) -> String {
        buffer
            .content()
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect::<String>()
    }

    fn unicode() -> Theme {
        Theme::no_color()
    }

    fn ascii() -> Theme {
        Theme::no_color().with_glyphs(&ASCII)
    }

    #[test]
    fn renderer_handles_small_screens_and_shows_grants() {
        let args = Args::try_parse_from(["ejm-cli", "--replay", "--allow-write"]).unwrap();
        for (width, height) in [(10, 5), (40, 10), (100, 30)] {
            let text = text(&render_app(&args, &unicode(), width, height, app()));
            if width == 100 {
                assert!(text.contains("ejm-cli 0.1.0"), "{text}");
                assert!(text.contains("offline replay"), "{text}");
                assert!(text.contains("/tmp"), "{text}");
                assert!(text.contains("grants: read + write"), "{text}");
                assert!(text.contains("What shall we build?"));
                assert!(
                    text.contains("12 agents · 2 model calls, 3 waiting · 0 tools"),
                    "{text}"
                );
                assert!(text.contains("› Describe a task…"));
                assert!(text.contains("/ commands · ? shortcuts"), "{text}");
                assert!(!text.contains('╭'), "no boxes: {text}");
            }
            if width == 40 {
                assert!(text.contains("grants: read + write"), "{text}");
            }
        }
        assert!(text(&render_app(&args, &unicode(), 19, 8, app())).contains("Enlarge the"));
    }

    #[test]
    fn plain_mode_draws_only_ascii() {
        let args = Args::try_parse_from(["ejm-cli", "--replay", "--plain"]).unwrap();
        let text = text(&render_app(&args, &ascii(), 100, 30, app()));
        assert!(text.is_ascii(), "{text}");
        assert!(text.contains("> Describe a task..."));
        assert!(text.contains("----"));
    }

    #[test]
    fn running_turn_shows_spinner_elapsed_time_and_tool_progress() {
        let args = Args::try_parse_from(["ejm-cli", "--replay"]).unwrap();
        let mut app = app();
        let session = app.session.id.clone();
        let event = |sequence: u64, mut kind: serde_json::Value| {
            let fields = kind.as_object_mut().unwrap();
            fields.insert("session_id".into(), session.clone().into());
            fields.insert("sequence".into(), sequence.into());
            fields.insert("timestamp_ms".into(), 0.into());
            fields.insert("run_id".into(), "run".into());
            serde_json::from_value::<ejm_harness::Event>(kind).unwrap()
        };
        app.apply(event(
            1,
            serde_json::json!({"type": "run_started", "prompt": "Fix the parser"}),
        ));
        app.apply(event(
            2,
            serde_json::json!({"type": "model_started", "step": 2}),
        ));
        app.apply(event(
            3,
            serde_json::json!({"type": "tool_started", "call": {
                "id": "c1", "name": "read_file", "arguments": {"path": "src/args.rs"}
            }}),
        ));
        let text = text(&render_app(&args, &unicode(), 100, 30, app));
        assert!(text.contains("› Fix the parser"), "{text}");
        assert!(text.contains("read_file  src/args.rs  running…"), "{text}");
        assert!(
            text.contains("Running read_file… (0s · Esc to stop)"),
            "{text}"
        );
        assert!(
            text.contains("Esc stop · Ctrl+K steer · Enter queue"),
            "{text}"
        );
        assert!(text.contains("Queue a follow-up"), "{text}");
    }

    #[test]
    fn armed_quit_and_scrolling_are_announced() {
        let args = Args::try_parse_from(["ejm-cli", "--replay"]).unwrap();
        let mut app = app();
        for n in 0..40 {
            app.notice(format!("notice {n}"));
        }
        app.scroll_from_bottom = 5;
        app.quit_armed = Some(std::time::Instant::now());
        let text = text(&render_app(&args, &unicode(), 100, 30, app));
        assert!(text.contains("Ctrl+C again to quit"), "{text}");
        assert!(
            text.contains("↑ 5 lines above latest · End to follow"),
            "{text}"
        );
        assert!(!text.contains("notice 39"), "{text}");
    }

    #[test]
    fn command_menu_filters_and_marks_the_selection_without_color() {
        let args = Args::try_parse_from(["ejm-cli", "--replay", "--plain", "--no-color"]).unwrap();
        for (width, height) in [(100, 30), (40, 12)] {
            let mut app = app();
            app.composer.insert("/s");
            app.command_picker.selected = 2;
            let buffer = render_app(&args, &ascii(), width, height, app);
            let text = text(&buffer);
            assert!(text.is_ascii(), "{text}");
            assert!(text.contains("> /steer MESSAGE"), "{text}");
            assert!(!text.contains("/reload"), "{text}");
            assert!(
                buffer
                    .content()
                    .iter()
                    .all(|cell| cell.fg == Color::Reset && cell.bg == Color::Reset)
            );
        }
    }

    #[test]
    fn dismissed_picker_preserves_the_visible_draft() {
        let args = Args::try_parse_from(["ejm-cli", "--replay"]).unwrap();
        let mut app = app();
        app.composer.insert("/s");
        app.command_picker.dismissed = true;
        let text = text(&render_app(&args, &unicode(), 100, 30, app));
        assert!(text.contains("› /s"), "{text}");
        assert!(!text.contains("/sessions"), "{text}");
    }

    #[test]
    fn help_scrolls_selected_commands_into_view_on_short_screens() {
        let args = Args::try_parse_from(["ejm-cli", "--replay", "--plain"]).unwrap();
        for (width, height) in [(100, 30), (40, 12)] {
            for name in ["help", "sessions", "reload", "exit"] {
                let mut app = app();
                let selected = app
                    .commands
                    .iter()
                    .position(|entry| entry.name == name)
                    .unwrap();
                app.overlay = Some(Overlay::Help { selected });
                let text = text(&render_app(&args, &ascii(), width, height, app));
                assert!(
                    text.contains(&format!("> /{name}")),
                    "{width}x{height}: {text}"
                );
                assert!(text.is_ascii(), "{text}");
            }
        }
    }

    fn event(app: &App, sequence: u64, mut kind: serde_json::Value) -> ejm_harness::Event {
        let fields = kind.as_object_mut().unwrap();
        fields.insert("session_id".into(), app.session.id.clone().into());
        fields.insert("sequence".into(), sequence.into());
        fields.insert("timestamp_ms".into(), 0.into());
        fields.insert("run_id".into(), "run".into());
        serde_json::from_value(kind).unwrap()
    }

    #[test]
    fn finished_run_leaves_a_summary_and_a_quiet_working_line() {
        let args = Args::try_parse_from(["ejm-cli", "--replay"]).unwrap();
        let mut app = app();
        for (sequence, kind) in [
            serde_json::json!({"type": "run_started", "prompt": "Fix the parser"}),
            serde_json::json!({"type": "text_delta", "text": "Done."}),
            serde_json::json!({"type": "run_finished", "summary": {
                "run_id": "run", "status": "completed", "steps": 3, "text": "Done.",
                "usage": {"input_tokens": 0, "output_tokens": 0}, "error": null
            }}),
        ]
        .into_iter()
        .enumerate()
        {
            let event = event(&app, u64::try_from(sequence).unwrap() + 1, kind);
            app.apply(event);
        }
        let text = text(&render_app(&args, &unicode(), 100, 30, app));
        assert!(text.contains("✓ completed · 3 steps "), "{text}");
        assert!(!text.contains("tokens"), "zero usage is not shown: {text}");
        assert!(!text.contains("Ready"), "{text}");
    }

    #[test]
    fn unconfigured_provider_shows_how_to_connect() {
        let mut args = Args::try_parse_from(["ejm-cli"]).unwrap();
        args.setup_needed = Some("set EJM_CLI_API_KEY".into());
        let mut app = app();
        app.error("Not connected.");
        let text = text(&render_app(&args, &unicode(), 100, 30, app));
        assert!(text.contains("not connected"), "{text}");
        assert!(text.contains("Connect a model to start"), "{text}");
        assert!(text.contains("ejm-cli login chatgpt"), "{text}");
        assert!(text.contains("Reason: set EJM_CLI_API_KEY"), "{text}");
        assert!(text.contains("✗ Not connected."), "{text}");
    }

    #[test]
    fn picker_names_its_keys_and_status_opens_as_rows() {
        let args = Args::try_parse_from(["ejm-cli", "--replay"]).unwrap();
        let mut app = app();
        app.composer.insert("/");
        let picker = text(&render_app(&args, &unicode(), 100, 30, app));
        assert!(
            picker.contains("↑/↓ choose · Tab complete · Esc close"),
            "{picker}"
        );
        let mut app = super::tests::app();
        app.overlay = Some(Overlay::Status {
            rows: vec![("Connection", "ready".into()), ("Grants", "read".into())],
        });
        let status = text(&render_app(&args, &unicode(), 100, 30, app));
        assert!(status.contains("Status · Esc close"), "{status}");
        assert!(status.contains("  Connection    ready"), "{status}");
        assert!(!status.contains('!'), "{status}");
    }
}
