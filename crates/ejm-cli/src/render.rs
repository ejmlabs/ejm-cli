use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::Line,
    widgets::{Clear, List, ListItem, ListState, Padding, Paragraph},
};

use crate::{
    app::{App, Overlay, RunState},
    args::Args,
    ejm_theme::Glyphs,
    ejm_tui::{
        self, Header, Theme, composer_block, composer_layout, composer_rows, empty_state,
        header_lines, hints, is_ascii, kv_line, list_row, popup, short_id, status_line,
        titled_block, too_small_lines, transcript_block,
    },
    extensions::Palette,
};

/// A package theme (`--theme`) replaces the kit colors for the five text roles it names.
fn themed(palette: Palette) -> Theme {
    let fg = |color| Style::default().fg(color);
    let accent = fg(palette.accent);
    Theme {
        accent,
        dim: fg(palette.dim),
        warning: fg(palette.warning),
        error: fg(palette.error),
        success: fg(palette.positive),
        plain: Style::default(),
        bold: Style::default().add_modifier(Modifier::BOLD),
        selected: accent.add_modifier(Modifier::REVERSED),
        special: fg(palette.dim),
    }
}

pub fn draw(
    frame: &mut Frame<'_>,
    app: &mut App,
    args: &Args,
    snapshot: &ejm_harness::HarnessSnapshot,
) {
    let no_color = args.no_color || std::env::var_os("NO_COLOR").is_some();
    let theme = match app.palette {
        Some(palette) if !no_color => themed(palette),
        _ => Theme::from_env(args.no_color, args.appearance.into()),
    };
    draw_screen(
        frame,
        app,
        args,
        snapshot,
        &theme,
        ejm_tui::glyphs_from_env(args.plain),
    );
}

fn draw_screen(
    frame: &mut Frame<'_>,
    app: &mut App,
    args: &Args,
    snapshot: &ejm_harness::HarnessSnapshot,
    theme: &Theme,
    glyphs: &Glyphs,
) {
    let area = frame.area();
    if ejm_tui::too_small(area) {
        frame.render_widget(
            Paragraph::new(too_small_lines("ejm-cli", usize::from(area.width))),
            area,
        );
        return;
    }
    // Box border and pad take 4 columns and the prompt takes 2 more.
    let composer_width = usize::from(area.width.saturating_sub(6).max(1));
    let composer = composer_layout(&app.composer.text, app.composer.cursor, composer_width);
    let input_height = u16::try_from(composer.0.len())
        .unwrap_or(u16::MAX)
        .clamp(1, 6)
        + 2;
    let [header, transcript, status, widgets, input, hint_area] = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(1),
        Constraint::Length(1),
        Constraint::Length(u16::try_from(app.widget_lines.len()).unwrap_or(4).min(4)),
        Constraint::Length(input_height.min(area.height.saturating_sub(5))),
        Constraint::Length(1),
    ])
    .areas(area);
    draw_header(frame, app, args, theme, glyphs, header);
    draw_transcript(frame, app, theme, transcript);
    draw_status(frame, app, theme, glyphs, status, snapshot);
    frame.render_widget(
        Paragraph::new(app.widget_lines.join("\n")).style(theme.dim),
        widgets,
    );
    draw_composer(frame, app, theme, glyphs, input, composer);
    frame.render_widget(
        Paragraph::new(hints(
            theme,
            glyphs,
            &[
                ("Enter", "send"),
                ("Ctrl+C", "stop/quit"),
                ("Alt+Enter", "new line"),
                ("Ctrl+K", "steer"),
                ("Ctrl+S", "sessions"),
                ("/help", "commands"),
            ],
            usize::from(area.width),
        )),
        hint_area,
    );

    draw_overlay(frame, app, theme, glyphs);
}

fn draw_header(
    frame: &mut Frame<'_>,
    app: &App,
    args: &Args,
    theme: &Theme,
    glyphs: &Glyphs,
    header: Rect,
) {
    let lines = header_lines(
        theme,
        glyphs,
        &Header {
            app: "ejm-cli",
            provider: &safe_text(args.provider_label()),
            title: &safe_text(&app.session.title),
            grants: &args.grants(),
            elevated: args.allow_shell || args.allow_write,
            cwd: &safe_text(&app.session.cwd.display().to_string()),
            session_id: &app.session.id,
        },
        usize::from(header.width),
    );
    frame.render_widget(Paragraph::new(lines.to_vec()), header);
}

fn draw_transcript(frame: &mut Frame<'_>, app: &mut App, theme: &Theme, transcript: Rect) {
    let width = usize::from(transcript.width.saturating_sub(2).max(1));
    let mut lines = Vec::new();
    if app.blocks.is_empty() {
        lines.extend(empty_state(
            theme,
            "What shall we build?",
            &[
                "Ask about this repository, investigate a bug, or describe a change.",
                "Read tools are ready. Write and shell access require explicit flags.",
                "Use /help for commands. Sessions are saved locally.",
            ],
            width,
        ));
    }
    for block in &app.blocks {
        lines.extend(transcript_block(
            theme,
            block.kind,
            &block.title,
            &block.text,
            width,
        ));
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
            .block(ratatui::widgets::Block::default().padding(Padding::horizontal(1))),
        transcript,
    );
}

fn draw_status(
    frame: &mut Frame<'_>,
    app: &App,
    theme: &Theme,
    glyphs: &Glyphs,
    status: Rect,
    snapshot: &ejm_harness::HarnessSnapshot,
) {
    let counts = [
        format!("{} agents", snapshot.active_agents),
        format!(
            "models {}+{}q",
            snapshot.model_requests, snapshot.waiting_model_requests
        ),
        format!(
            "tools {}+{}q",
            snapshot.tools.running, snapshot.tools.queued
        ),
    ];
    let state = if app.scroll_from_bottom > 0 {
        ejm_tui::RunState::ScrolledUp(app.scroll_from_bottom)
    } else {
        match app.run_state {
            RunState::Idle => ejm_tui::RunState::Idle,
            RunState::Running => ejm_tui::RunState::Running,
            RunState::Stopping => ejm_tui::RunState::Stopping,
        }
    };
    frame.render_widget(
        Paragraph::new(status_line(
            theme,
            glyphs,
            &counts,
            state,
            &safe_text(&app.status),
            usize::from(status.width),
        )),
        status,
    );
}

fn draw_composer(
    frame: &mut Frame<'_>,
    app: &App,
    theme: &Theme,
    glyphs: &Glyphs,
    input: Rect,
    composer: (Vec<String>, usize, usize),
) {
    let (composer_lines, cursor_row, cursor_col) = composer;
    let composer_width = usize::from(input.width.saturating_sub(6).max(1));
    let visible_input_rows = input.height.saturating_sub(2).max(1) as usize;
    let first_input_row = cursor_row.saturating_sub(visible_input_rows - 1);
    let mut visible_input = composer_lines
        .into_iter()
        .skip(first_input_row)
        .take(visible_input_rows)
        .collect::<Vec<_>>();
    // `composer_rows` shows the placeholder when every row is empty. A draft of only
    // newlines is not empty, so keep its rows with a blank cell.
    if !app.composer.text.is_empty() && visible_input.iter().all(String::is_empty) {
        for row in &mut visible_input {
            *row = " ".into();
        }
    }
    frame.render_widget(
        Paragraph::new(composer_rows(
            theme,
            glyphs,
            &visible_input,
            app.is_running(),
        ))
        .block(composer_block(theme, glyphs, app.is_running())),
        input,
    );
    if app.overlay.is_none() {
        // Border, pad and the two-cell prompt sit left of the text.
        let x = input.x
            + 4
            + u16::try_from(cursor_col.min(composer_width.saturating_sub(1))).unwrap_or(0);
        let y =
            input.y + 1 + u16::try_from(cursor_row.saturating_sub(first_input_row)).unwrap_or(0);
        if y < input.bottom().saturating_sub(1) {
            frame.set_cursor_position((x, y));
        }
    }
}

fn draw_overlay(frame: &mut Frame<'_>, app: &App, theme: &Theme, glyphs: &Glyphs) {
    let area = frame.area();
    match &app.overlay {
        Some(Overlay::Help) => draw_help(frame, theme, glyphs),
        Some(Overlay::Sessions { sessions, selected }) => {
            let popup = popup(area, 86, 20);
            frame.render_widget(Clear, popup);
            let (arrows, dot) = (
                if is_ascii(glyphs) {
                    "Up/Down"
                } else {
                    "\u{2191}/\u{2193}"
                },
                glyphs.separator,
            );
            let block = titled_block(
                glyphs,
                &format!("sessions {dot} {arrows} choose {dot} Enter open {dot} Esc close"),
                theme.plain,
            );
            let width = usize::from(block.inner(popup).width);
            let items = sessions
                .iter()
                .enumerate()
                .map(|(index, session)| {
                    let status = match session.status {
                        ejm_harness::SessionStatus::Idle => "idle",
                        ejm_harness::SessionStatus::Running => "running",
                        ejm_harness::SessionStatus::Interrupted => "interrupted",
                    };
                    ListItem::new(list_row(
                        theme,
                        glyphs,
                        short_id(&session.id),
                        &format!("[{status}]  {}", safe_text(&session.title)),
                        index == *selected,
                        width,
                    ))
                })
                .collect::<Vec<_>>();
            let mut state = ListState::default().with_selected(Some(*selected));
            frame.render_stateful_widget(List::new(items).block(block), popup, &mut state);
        }
        None => {}
    }
}

fn draw_help(frame: &mut Frame<'_>, theme: &Theme, glyphs: &Glyphs) {
    let popup = popup(frame.area(), 78, 24);
    frame.render_widget(Clear, popup);
    let block = titled_block(
        glyphs,
        &format!("ejm-cli {} help", glyphs.separator),
        theme.plain,
    );
    let width = usize::from(block.inner(popup).width);
    let recall = if is_ascii(glyphs) {
        "Up / Down"
    } else {
        "\u{2191} / \u{2193}"
    };
    let row = |key: &str, text: &str| kv_line(theme, glyphs, key, text, 16, width);
    frame.render_widget(
        Paragraph::new(vec![
            Line::styled("Conversation", theme.accent),
            row("Enter", "Send, or queue while a run is active"),
            row("Alt+Enter", "Insert a newline (Ctrl+J also works)"),
            row("Ctrl+K", "Steer the active run with your draft"),
            row("Ctrl+C", "Stop an active run; otherwise exit"),
            row("Ctrl+D", "Exit if the composer is empty"),
            row(recall, "Recall sent messages"),
            row("PageUp/PageDown", "Scroll transcript; End follows output"),
            Line::raw(""),
            Line::styled("Commands", theme.accent),
            row("/new [title]", "Create a session in this workspace"),
            row("/sessions", "Choose a saved session (Ctrl+S)"),
            row("/resume ID", "Open a saved session"),
            row("/queue MESSAGE", "Queue a follow-up turn"),
            row("/steer MESSAGE", "Send direction to the active run"),
            row("/stop", "Cancel the current run"),
            row("/help", "Show this reference"),
            row("/quit", "Stop runs and exit"),
            Line::raw(""),
            Line::styled(
                "Grants are fixed at launch. Esc closes this panel.",
                theme.dim,
            ),
        ])
        .block(block),
        popup,
    );
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
    use crate::{ejm_theme::UNICODE, ejm_tui::Appearance};
    use clap::Parser;
    use ratatui::{Terminal, backend::TestBackend, style::Color};

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
    fn appearance_flag_maps_onto_kit_appearance() {
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
    fn kit_palette_needs_truecolor() {
        let light = Theme::detect(false, Appearance::Light, None, Some("truecolor"));
        let dark = Theme::detect(false, Appearance::Dark, None, Some("24bit"));
        assert_eq!(light.accent.fg, Some(Color::Rgb(0x29, 0x44, 0x3B)));
        assert_eq!(dark.accent.fg, Some(Color::Rgb(0xB8, 0xC6, 0xAA)));
        assert_eq!(dark.dim.fg, Some(Color::Rgb(0x84, 0x96, 0x82)));
        assert_eq!(light.error.fg, Some(Color::Rgb(0xA6, 0x3A, 0x3A)));
        assert_eq!(
            Theme::detect(false, Appearance::Dark, None, None).accent.fg,
            Some(Color::Cyan)
        );
        assert_eq!(
            Theme::detect(false, Appearance::Light, None, None)
                .accent
                .fg,
            Some(Color::Blue)
        );
        assert_eq!(Theme::no_color().accent.fg, None);
    }

    #[test]
    fn package_theme_replaces_the_five_text_roles() {
        let red = Color::Rgb(255, 0, 0);
        let theme = themed(Palette {
            accent: red,
            dim: red,
            warning: red,
            error: red,
            positive: Color::Green,
        });
        assert_eq!(theme.accent.fg, Some(red));
        assert_eq!(theme.success.fg, Some(Color::Green));
        assert!(theme.selected.add_modifier.contains(Modifier::REVERSED));
    }

    #[test]
    fn composer_counts_wide_characters_and_wrapped_cursor() {
        let (rows, row, col) = composer_layout("a界b🦀", "a界b🦀".len(), 4);
        assert_eq!(rows, ["a界b", "🦀"]);
        assert_eq!((row, col), (1, 2));
    }

    fn screen(args: &Args, glyphs: &Glyphs, width: u16, height: u16) -> String {
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
        let theme = Theme::no_color();
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|frame| draw_screen(frame, &mut app, args, &snapshot, &theme, glyphs))
            .unwrap();
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect::<String>()
    }

    #[test]
    fn renderer_handles_small_screens_and_shows_grants() {
        let args = Args::try_parse_from(["ejm-cli", "--replay", "--allow-write"]).unwrap();
        for (width, height) in [(10, 5), (40, 10), (100, 30)] {
            let text = screen(&args, &UNICODE, width, height);
            if width == 100 {
                assert!(text.contains("grants: read + write"));
                assert!(text.contains("What shall we build?"));
                assert!(text.contains("12 agents · models 2+3q · tools 0+0q · Ready"));
                assert!(text.contains("› Describe a task…"));
                assert!(text.contains("╭ message"));
                assert!(text.contains("Enter send · Ctrl+C stop/quit · Alt+Enter new line"));
            }
        }
        assert!(screen(&args, &UNICODE, 19, 8).contains("Enlarge the"));
    }

    #[test]
    fn plain_mode_draws_only_ascii() {
        let args = Args::try_parse_from(["ejm-cli", "--replay", "--plain"]).unwrap();
        let text = screen(&args, ejm_tui::glyphs_from_env(args.plain), 100, 30);
        assert!(text.is_ascii(), "{text}");
        assert!(text.contains("> Describe a task..."));
        assert!(text.contains("+ message"));
    }
}
