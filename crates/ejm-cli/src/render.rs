use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
};
use unicode_width::UnicodeWidthChar;

use crate::{
    app::{App, BlockKind, Overlay, RunState},
    args::{Appearance, Args},
};

#[derive(Clone, Copy)]
struct Theme {
    accent: Style,
    dim: Style,
    warning: Style,
    error: Style,
    positive: Style,
}

/// ejmlabs brand colors for text on a terminal background we do not paint.
/// Light uses Forest and darkened tints for contrast on Paper. Dark uses Sage
/// and lightened tints. Warning and error are not brand colors; they are
/// chosen to keep at least 4.5:1 on Paper (light) and on Ink (dark).
const LIGHT: crate::extensions::Palette = crate::extensions::Palette {
    accent: Color::Rgb(0x29, 0x44, 0x3B),
    dim: Color::Rgb(0x5B, 0x68, 0x56),
    warning: Color::Rgb(0x8A, 0x5A, 0x12),
    error: Color::Rgb(0xA3, 0x3A, 0x2E),
    positive: Color::Rgb(0x2F, 0x6B, 0x3A),
};
const DARK: crate::extensions::Palette = crate::extensions::Palette {
    accent: Color::Rgb(0xB7, 0xC6, 0xA8),
    dim: Color::Rgb(0x8A, 0x9A, 0x7B),
    warning: Color::Rgb(0xE0, 0xB5, 0x65),
    error: Color::Rgb(0xE5, 0x82, 0x6F),
    positive: Color::Rgb(0xA8, 0xC9, 0x9A),
};

fn is_light(appearance: Appearance, colorfgbg: Option<&str>) -> bool {
    match appearance {
        Appearance::Light => true,
        Appearance::Dark => false,
        // "fg;bg" with ANSI indexes. Backgrounds 7 and 9 to 15 are light.
        Appearance::Auto => colorfgbg
            .and_then(|value| value.rsplit(';').next())
            .and_then(|bg| bg.parse::<u8>().ok())
            .is_some_and(|bg| bg == 7 || (9..=15).contains(&bg)),
    }
}

fn truecolor(colorterm: Option<&str>) -> bool {
    colorterm.is_some_and(|value| value.contains("truecolor") || value.contains("24bit"))
}

impl Theme {
    fn from_palette(palette: crate::extensions::Palette) -> Self {
        Self {
            accent: Style::default().fg(palette.accent),
            dim: Style::default().fg(palette.dim),
            warning: Style::default().fg(palette.warning),
            error: Style::default().fg(palette.error),
            positive: Style::default().fg(palette.positive),
        }
    }

    fn new(
        no_color: bool,
        palette: Option<crate::extensions::Palette>,
        light: bool,
        truecolor: bool,
    ) -> Self {
        if no_color {
            Self {
                accent: Style::default().add_modifier(Modifier::BOLD),
                dim: Style::default(),
                warning: Style::default().add_modifier(Modifier::BOLD),
                error: Style::default().add_modifier(Modifier::BOLD),
                positive: Style::default(),
            }
        } else if let Some(palette) = palette {
            Self::from_palette(palette)
        } else if truecolor {
            Self::from_palette(if light { LIGHT } else { DARK })
        } else if light {
            // Named colors keep working on 16 and 256 color terminals.
            Self::from_palette(crate::extensions::Palette {
                accent: Color::Blue,
                dim: Color::DarkGray,
                warning: Color::Magenta,
                error: Color::Red,
                positive: Color::Green,
            })
        } else {
            Self::from_palette(crate::extensions::Palette {
                accent: Color::Cyan,
                dim: Color::DarkGray,
                warning: Color::Yellow,
                error: Color::Red,
                positive: Color::Green,
            })
        }
    }
}

pub fn draw(
    frame: &mut Frame<'_>,
    app: &mut App,
    args: &Args,
    snapshot: &ejm_harness::HarnessSnapshot,
) {
    let area = frame.area();
    let theme = Theme::new(
        args.no_color || std::env::var_os("NO_COLOR").is_some(),
        app.palette,
        is_light(args.appearance, std::env::var("COLORFGBG").ok().as_deref()),
        truecolor(std::env::var("COLORTERM").ok().as_deref()),
    );
    if area.width < 20 || area.height < 8 {
        frame.render_widget(
            Paragraph::new("Enlarge the terminal\nto use ejm-cli."),
            area,
        );
        return;
    }
    let composer_width = area.width.saturating_sub(4).max(1) as usize;
    let composer = composer_layout(&app.composer.text, app.composer.cursor, composer_width);
    let input_height = u16::try_from(composer.0.len())
        .unwrap_or(u16::MAX)
        .clamp(1, 6)
        + 2;
    let [header, transcript, status, widgets, input, hints] = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(1),
        Constraint::Length(1),
        Constraint::Length(u16::try_from(app.widget_lines.len()).unwrap_or(4).min(4)),
        Constraint::Length(input_height.min(area.height.saturating_sub(5))),
        Constraint::Length(1),
    ])
    .areas(area);
    draw_header(frame, app, args, theme, header);
    draw_transcript(frame, app, theme, transcript);
    draw_status(frame, app, theme, status, snapshot);
    frame.render_widget(
        Paragraph::new(app.widget_lines.join("\n")).style(theme.dim),
        widgets,
    );
    draw_composer(frame, app, theme, input, composer);
    let hint_text = if area.width >= 85 {
        " Enter send · Alt+Enter newline · Ctrl+K steer · Ctrl+S sessions · Ctrl+C stop/quit"
    } else {
        " Enter send · Ctrl+C stop/quit · /help"
    };
    frame.render_widget(Paragraph::new(hint_text).style(theme.dim), hints);

    draw_overlay(frame, app, theme);
}

fn draw_header(frame: &mut Frame<'_>, app: &App, args: &Args, theme: Theme, header: Rect) {
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled(" ejm-cli ", theme.accent.add_modifier(Modifier::BOLD)),
                Span::raw(safe_text(args.provider_label())),
                Span::styled(format!("  ·  {}", safe_text(&app.session.title)), theme.dim),
            ]),
            Line::from(vec![
                Span::styled(
                    format!(" grants: {}", args.grants()),
                    if args.allow_shell || args.allow_write {
                        theme.warning
                    } else {
                        theme.dim
                    },
                ),
                Span::styled(
                    format!("  ·  {}", safe_text(&app.session.cwd.display().to_string())),
                    theme.dim,
                ),
                Span::styled(format!("  ·  {}", short_id(&app.session.id)), theme.dim),
            ]),
        ]),
        header,
    );
}

fn draw_transcript(frame: &mut Frame<'_>, app: &mut App, theme: Theme, transcript: Rect) {
    let width = transcript.width.saturating_sub(2).max(1) as usize;
    let mut lines = Vec::new();
    if app.blocks.is_empty() {
        lines.extend([
            Line::raw(""),
            Line::styled(
                "  What shall we build?",
                theme.accent.add_modifier(Modifier::BOLD),
            ),
            Line::raw(""),
        ]);
        for text in [
            "Ask about this repository, investigate a bug, or describe a change.",
            "Read tools are ready. Write and shell access require explicit flags.",
            "Use /help for commands. Sessions are saved locally.",
        ] {
            lines.extend(wrap_line(text, width, theme.dim));
            lines.push(Line::raw(""));
        }
    }
    for block in &app.blocks {
        let style = match block.kind {
            BlockKind::User => theme.accent,
            BlockKind::Assistant => Style::default().add_modifier(Modifier::BOLD),
            BlockKind::Tool => theme.dim,
            BlockKind::Error => theme.error,
            BlockKind::Notice => theme.warning,
        };
        lines.extend(wrap_line(&block.title, width, style));
        let mut fenced = false;
        let mut fenced_diff = false;
        let contains_diff = block
            .text
            .lines()
            .any(|line| line.starts_with("@@ ") || line.starts_with("diff --git "));
        for text in block.text.lines() {
            let line_style = if text.trim_start().starts_with("```") {
                fenced = !fenced;
                fenced_diff = fenced
                    && (text.trim_start().starts_with("```diff")
                        || text.trim_start().starts_with("```patch"));
                theme.dim
            } else if (fenced_diff || contains_diff)
                && text.starts_with('+')
                && !text.starts_with("+++")
            {
                theme.positive
            } else if (fenced_diff || contains_diff)
                && text.starts_with('-')
                && !text.starts_with("---")
            {
                theme.error
            } else if text.starts_with('#') {
                theme.accent.add_modifier(Modifier::BOLD)
            } else if fenced || block.kind == BlockKind::Tool {
                theme.dim
            } else if block.kind == BlockKind::Error {
                theme.error
            } else {
                Style::default()
            };
            lines.extend(wrap_line(text, width, line_style));
        }
        lines.push(Line::raw(""));
    }
    let available = transcript.height as usize;
    app.scroll_from_bottom = app
        .scroll_from_bottom
        .min(lines.len().saturating_sub(available));
    let end = lines.len().saturating_sub(app.scroll_from_bottom);
    let start = end.saturating_sub(available);
    let visible = lines.drain(start..end).collect::<Vec<_>>();
    frame.render_widget(
        Paragraph::new(visible)
            .block(Block::default().padding(ratatui::widgets::Padding::horizontal(1))),
        transcript,
    );
}

fn draw_status(
    frame: &mut Frame<'_>,
    app: &App,
    theme: Theme,
    status: Rect,
    snapshot: &ejm_harness::HarnessSnapshot,
) {
    let run_status = if app.scroll_from_bottom > 0 {
        format!(
            " ↑ {} lines above latest · End to follow",
            app.scroll_from_bottom
        )
    } else {
        format!(
            " {}{}",
            if app.is_running() { "● " } else { "" },
            if app.run_state == RunState::Stopping {
                "Stopping…".into()
            } else {
                safe_text(&app.status)
            }
        )
    };
    let status_text = format!(
        " {} agents · models {}+{}q · tools {}+{}q ·{}",
        snapshot.active_agents,
        snapshot.model_requests,
        snapshot.waiting_model_requests,
        snapshot.tools.running,
        snapshot.tools.queued,
        run_status,
    );
    frame.render_widget(
        Paragraph::new(status_text).style(if app.is_running() {
            theme.accent
        } else {
            theme.dim
        }),
        status,
    );
}

fn draw_composer(
    frame: &mut Frame<'_>,
    app: &App,
    theme: Theme,
    input: Rect,
    composer: (Vec<String>, usize, usize),
) {
    let (composer_lines, cursor_row, cursor_col) = composer;
    let composer_width = usize::from(input.width.saturating_sub(4).max(1));
    let visible_input_rows = input.height.saturating_sub(2).max(1) as usize;
    let first_input_row = cursor_row.saturating_sub(visible_input_rows - 1);
    let visible_input = composer_lines
        .into_iter()
        .skip(first_input_row)
        .take(visible_input_rows)
        .map(Line::raw)
        .collect::<Vec<_>>();
    let placeholder = app.composer.text.is_empty();
    frame.render_widget(
        Paragraph::new(if placeholder {
            vec![Line::styled(
                if app.is_running() {
                    "Queue a follow-up, or Ctrl+K to steer…"
                } else {
                    "Describe a task…"
                },
                theme.dim,
            )]
        } else {
            visible_input
        })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(theme.accent)
                .title(if app.is_running() {
                    " message · Enter queues "
                } else {
                    " message "
                })
                .padding(ratatui::widgets::Padding::horizontal(1)),
        ),
        input,
    );
    if app.overlay.is_none() {
        let x = input.x
            + 2
            + u16::try_from(cursor_col.min(composer_width.saturating_sub(1))).unwrap_or(0);
        let y =
            input.y + 1 + u16::try_from(cursor_row.saturating_sub(first_input_row)).unwrap_or(0);
        if y < input.bottom().saturating_sub(1) {
            frame.set_cursor_position((x, y));
        }
    }
}

fn draw_overlay(frame: &mut Frame<'_>, app: &App, theme: Theme) {
    let area = frame.area();
    match &app.overlay {
        Some(Overlay::Help) => draw_help(frame, theme),
        Some(Overlay::Sessions { sessions, selected }) => {
            let popup = popup(area, 86, 20);
            frame.render_widget(Clear, popup);
            let items = sessions
                .iter()
                .map(|session| {
                    let status = match session.status {
                        ejm_harness::SessionStatus::Idle => "idle",
                        ejm_harness::SessionStatus::Running => "running",
                        ejm_harness::SessionStatus::Interrupted => "interrupted",
                    };
                    ListItem::new(format!(
                        "{}  [{status}]  {}",
                        short_id(&session.id),
                        safe_text(&session.title)
                    ))
                })
                .collect::<Vec<_>>();
            let mut state = ListState::default().with_selected(Some(*selected));
            frame.render_stateful_widget(
                List::new(items)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title(" sessions · ↑/↓ choose · Enter open · Esc close "),
                    )
                    .highlight_style(theme.accent.add_modifier(Modifier::REVERSED)),
                popup,
                &mut state,
            );
        }
        None => {}
    }
}

fn draw_help(frame: &mut Frame<'_>, theme: Theme) {
    let popup = popup(frame.area(), 78, 24);
    frame.render_widget(Clear, popup);
    frame.render_widget(
        Paragraph::new(vec![
            Line::styled("Conversation", theme.accent),
            Line::raw("Enter            Send, or queue while a run is active"),
            Line::raw("Alt+Enter        Insert a newline (Ctrl+J also works)"),
            Line::raw("Ctrl+K           Steer the active run with your draft"),
            Line::raw("Ctrl+C           Stop an active run; otherwise exit"),
            Line::raw("Ctrl+D           Exit if the composer is empty"),
            Line::raw("↑ / ↓            Recall sent messages"),
            Line::raw("PageUp/PageDown  Scroll transcript; End follows output"),
            Line::raw(""),
            Line::styled("Commands", theme.accent),
            Line::raw("/new [title]     Create a session in this workspace"),
            Line::raw("/sessions        Choose a saved session (Ctrl+S)"),
            Line::raw("/resume ID       Open a saved session"),
            Line::raw("/queue MESSAGE   Queue a follow-up turn"),
            Line::raw("/steer MESSAGE   Send direction to the active run"),
            Line::raw("/stop            Cancel the current run"),
            Line::raw("/help            Show this reference"),
            Line::raw("/quit            Stop runs and exit"),
            Line::raw(""),
            Line::styled(
                "Grants are fixed at launch. Esc closes this panel.",
                theme.dim,
            ),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" ejm-cli · help ")
                .padding(ratatui::widgets::Padding::horizontal(1)),
        ),
        popup,
    );
}

fn popup(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width.saturating_sub(2));
    let height = height.min(area.height.saturating_sub(2));
    Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    )
}

fn short_id(id: &str) -> &str {
    id.get(..8).unwrap_or(id)
}

fn wrap_line(text: &str, width: usize, style: Style) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut columns = 0;
    for character in text.chars() {
        let character_width = character.width().unwrap_or(0);
        if columns + character_width > width && !current.is_empty() {
            lines.push(Line::styled(std::mem::take(&mut current), style));
            columns = 0;
        }
        current.push(character);
        columns += character_width;
    }
    lines.push(Line::styled(current, style));
    lines
}

pub fn composer_layout(text: &str, cursor: usize, width: usize) -> (Vec<String>, usize, usize) {
    let width = width.max(1);
    let mut rows = vec![String::new()];
    let mut columns = 0;
    let mut cursor_position = (0, 0);
    for (index, character) in text.char_indices() {
        let character_width = character.width().unwrap_or(0);
        if character != '\n' && columns + character_width > width {
            rows.push(String::new());
            columns = 0;
        }
        if index == cursor {
            cursor_position = (rows.len() - 1, columns);
        }
        if character == '\n' {
            rows.push(String::new());
            columns = 0;
        } else {
            let last = rows.len() - 1;
            rows[last].push(character);
            columns += character_width;
        }
    }
    if cursor == text.len() {
        if columns >= width {
            rows.push(String::new());
            columns = 0;
        }
        cursor_position = (rows.len() - 1, columns);
    }
    (rows, cursor_position.0, cursor_position.1)
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
    use ratatui::{Terminal, backend::TestBackend};
    use unicode_width::UnicodeWidthStr;

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
    fn brand_palette_needs_truecolor() {
        assert!(truecolor(Some("truecolor")));
        assert!(truecolor(Some("24bit")));
        assert!(!truecolor(Some("")));
        assert!(!truecolor(None));
        let light = Theme::new(false, None, true, true);
        let dark = Theme::new(false, None, false, true);
        assert_eq!(light.accent.fg, Some(Color::Rgb(0x29, 0x44, 0x3B)));
        assert_eq!(dark.accent.fg, Some(Color::Rgb(0xB7, 0xC6, 0xA8)));
        assert_eq!(
            Theme::new(false, None, false, false).accent.fg,
            Some(Color::Cyan)
        );
        assert_eq!(Theme::new(true, None, true, true).accent.fg, None);
    }

    #[test]
    fn composer_counts_wide_characters_and_wrapped_cursor() {
        let (rows, row, col) = composer_layout("a界b🦀", "a界b🦀".len(), 4);
        assert_eq!(rows, ["a界b", "🦀"]);
        assert_eq!((row, col), (1, 2));
        assert_eq!("界".width(), 2);
    }

    #[test]
    fn renderer_handles_small_screens_and_shows_grants() {
        let args = Args::try_parse_from(["ejm-cli", "--replay", "--allow-write"]).unwrap();
        let session = serde_json::from_value(serde_json::json!({
            "id": "123456789", "title": "test", "cwd": "/tmp", "created_at_ms": 0,
            "status": "idle"
        }))
        .unwrap();
        let mut app = App::new(session);
        let governor =
            ejm_harness_exec::Governor::new(ejm_harness_exec::ResourceLimits::default()).unwrap();
        let snapshot = ejm_harness::HarnessSnapshot {
            active_agents: 12,
            model_requests: 2,
            waiting_model_requests: 3,
            tools: governor.snapshot(),
        };
        for (width, height) in [(10, 5), (40, 10), (100, 30)] {
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            terminal
                .draw(|frame| draw(frame, &mut app, &args, &snapshot))
                .unwrap();
            let text = terminal
                .backend()
                .buffer()
                .content()
                .iter()
                .map(ratatui::buffer::Cell::symbol)
                .collect::<String>();
            if width == 100 {
                assert!(text.contains("grants: read + write"));
                assert!(text.contains("What shall we build?"));
                assert!(text.contains("12 agents · models 2+3q · tools 0+0q"));
            }
        }
    }
}
