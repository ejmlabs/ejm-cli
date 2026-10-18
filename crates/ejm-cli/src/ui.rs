use std::{
    io::{self, Stdout},
    panic,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use crossterm::{
    cursor::Show,
    event::{
        self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
        Event as InputEvent, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEventKind,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ejm_harness::{Harness, Session, SessionOptions};
use ratatui::{Terminal, backend::CrosstermBackend};
use tokio::task::JoinSet;

use crate::{
    app::{App, Overlay, RunState},
    args::Args,
    extensions::Extensions,
    render,
};

pub async fn run(harness: Harness, args: &Args, extensions: &Extensions) -> Result<()> {
    let mut events = harness.subscribe();
    let session = if let Some(id) = &args.session {
        find_session(&harness, id).await?
    } else {
        let cwd = args.cwd()?;
        let title = workspace_title(&cwd);
        harness
            .create_session(SessionOptions { cwd, title })
            .await?
    };
    let mut app = load_session(&harness, session).await?;
    app.palette = extensions.palette;
    if extensions.count() > 0 {
        app.notice(format!(
            "{} trusted native packages enabled. /extensions shows grants.",
            extensions.count()
        ));
    }
    let mut extension_tasks = ExtensionTasks::default();
    let mut widget_tick = tokio::time::interval(Duration::from_secs(1));
    widget_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut terminal = TerminalGuard::enter()?;
    let mut tick = tokio::time::interval(Duration::from_millis(32));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut last_status_draw = Instant::now();
    let shutdown = termination_signal();
    tokio::pin!(shutdown);
    loop {
        tokio::select! {
            () = &mut shutdown => return Ok(()),
            _ = tick.tick() => {
                for _ in 0..64 {
                    if !event::poll(Duration::ZERO)? { break; }
                    let input = event::read()?;
                    match handle_input(input, &mut app, &harness, args, extensions, &mut extension_tasks).await {
                        Ok(true) => return Ok(()),
                        Ok(false) => {},
                        Err(error) => app.error(format!("{error:#}")),
                    }
                }
                if app.dirty || (app.is_running() && last_status_draw.elapsed() >= Duration::from_millis(250)) {
                    let snapshot = harness.status().await;
                    terminal.terminal.draw(|frame| render::draw(frame, &mut app, args, &snapshot))?;
                    app.dirty = false;
                    last_status_draw = Instant::now();
                }
            }
            event = events.recv() => match event {
                Ok(event) => app.apply(event),
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                    recover_events(&harness, &mut app).await?;
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => return Ok(()),
            },
            _ = widget_tick.tick(), if extensions.has_widgets() && !extension_tasks.widget_active && !extension_tasks.widgets_paused => {
                extension_tasks.widget_active = true;
                let extensions = extensions.clone();
                let context = extensions.context(&harness, &app.session);
                extension_tasks.jobs.spawn(async move { ExtensionReply::Widgets(extensions.widgets(context).await) });
            }
            reply = extension_tasks.jobs.join_next(), if !extension_tasks.jobs.is_empty() => {
                match reply {
                    Some(Ok(ExtensionReply::Widgets(result))) => {
                        extension_tasks.widget_active = false;
                        match result { Ok(lines) => app.widget_lines = lines, Err(error) => { extension_tasks.widgets_paused = true; app.widget_lines = vec![format!("Widget paused: {}. /widgets resume to retry.", render::safe_text(&error.to_string()))]; } }
                    }
                    Some(Ok(ExtensionReply::Command(result))) => {
                        extension_tasks.command_active = false;
                        match result { Ok(text) => app.notice(text), Err(error) => app.error(format!("{error:#}")) }
                    }
                    Some(Err(error)) => { extension_tasks.widget_active = false; extension_tasks.command_active = false; app.error(format!("Extension task failed: {error}")); }
                    None => {}
                }
                app.dirty = true;
            }
        }
    }
}

async fn handle_input(
    input: InputEvent,
    app: &mut App,
    harness: &Harness,
    args: &Args,
    extensions: &Extensions,
    extension_tasks: &mut ExtensionTasks,
) -> Result<bool> {
    app.dirty = true;
    match input {
        InputEvent::Key(key)
            if key.kind == KeyEventKind::Press || key.kind == KeyEventKind::Repeat =>
        {
            if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                if app.run_state == RunState::Running {
                    harness.cancel(&app.session.id).await?;
                    app.run_state = RunState::Stopping;
                    app.status = "Stopping…".into();
                    return Ok(false);
                }
                return Ok(true);
            }
            if app.overlay.is_some() {
                return handle_overlay_key(key, app, harness).await;
            }
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                match key.code {
                    KeyCode::Char('d') if app.composer.text.is_empty() => return Ok(true),
                    KeyCode::Char('s') => open_sessions(app, harness).await?,
                    KeyCode::Char('k') => {
                        if app.is_running() && !app.composer.text.trim().is_empty() {
                            let text = app.composer.text.clone();
                            harness.steer(&app.session.id, text).await?;
                            app.composer.take();
                        } else {
                            app.notice(
                                "Steering needs an active run and a message in the composer.",
                            );
                        }
                    }
                    KeyCode::Char('j') => app.composer.insert("\n"),
                    KeyCode::Char('a') => app.composer.home(),
                    KeyCode::Char('e') => app.composer.end(),
                    KeyCode::Char('u') => {
                        app.composer.text.clear();
                        app.composer.cursor = 0;
                    }
                    _ => {}
                }
            } else {
                match key.code {
                    KeyCode::Enter
                        if key
                            .modifiers
                            .intersects(KeyModifiers::ALT | KeyModifiers::SHIFT) =>
                    {
                        app.composer.insert("\n");
                    }
                    KeyCode::Enter => {
                        let text = app.composer.text.clone();
                        if !text.trim().is_empty() {
                            if submit(text, app, harness, args, extensions, extension_tasks).await?
                            {
                                return Ok(true);
                            }
                            app.composer.take();
                        }
                    }
                    KeyCode::Char(character) => app.composer.insert(&character.to_string()),
                    KeyCode::Backspace => app.composer.backspace(),
                    KeyCode::Delete => app.composer.delete(),
                    KeyCode::Left => app.composer.left(),
                    KeyCode::Right => app.composer.right(),
                    KeyCode::Home => app.composer.home(),
                    KeyCode::End => {
                        app.composer.end();
                        app.scroll_from_bottom = 0;
                    }
                    KeyCode::Up => app.composer.history(true),
                    KeyCode::Down => app.composer.history(false),
                    KeyCode::PageUp => {
                        app.scroll_from_bottom = app.scroll_from_bottom.saturating_add(12);
                    }
                    KeyCode::PageDown => {
                        app.scroll_from_bottom = app.scroll_from_bottom.saturating_sub(12);
                    }
                    KeyCode::Esc => app.scroll_from_bottom = 0,
                    _ => {}
                }
            }
        }
        InputEvent::Paste(text) => app.composer.insert(&text),
        InputEvent::Mouse(mouse) => match mouse.kind {
            MouseEventKind::ScrollUp => {
                app.scroll_from_bottom = app.scroll_from_bottom.saturating_add(3);
            }
            MouseEventKind::ScrollDown => {
                app.scroll_from_bottom = app.scroll_from_bottom.saturating_sub(3);
            }
            _ => {}
        },
        _ => {}
    }
    Ok(false)
}

async fn handle_overlay_key(key: KeyEvent, app: &mut App, harness: &Harness) -> Result<bool> {
    match key.code {
        KeyCode::Esc => app.overlay = None,
        KeyCode::Up => {
            if let Some(Overlay::Sessions { selected, .. }) = &mut app.overlay {
                *selected = selected.saturating_sub(1);
            }
        }
        KeyCode::Down => {
            if let Some(Overlay::Sessions { sessions, selected }) = &mut app.overlay {
                *selected = (*selected + 1).min(sessions.len().saturating_sub(1));
            }
        }
        KeyCode::Enter => {
            let session = if let Some(Overlay::Sessions { sessions, selected }) = &app.overlay {
                sessions.get(*selected).cloned()
            } else {
                None
            };
            if let Some(session) = session {
                *app = load_session(harness, session).await?;
            } else {
                app.overlay = None;
            }
        }
        _ => {}
    }
    Ok(false)
}

async fn submit(
    text: String,
    app: &mut App,
    harness: &Harness,
    args: &Args,
    extensions: &Extensions,
    extension_tasks: &mut ExtensionTasks,
) -> Result<bool> {
    if let Some(command) = text.strip_prefix('/') {
        let (name, value) = command
            .split_once(char::is_whitespace)
            .unwrap_or((command, ""));
        let value = value.trim();
        match name {
            "help" => app.overlay = Some(Overlay::Help),
            "quit" | "exit" => return Ok(true),
            "sessions" | "agents" => open_sessions(app, harness).await?,
            "new" => {
                let cwd = args.cwd()?;
                let session = harness
                    .create_session(SessionOptions {
                        title: if value.is_empty() {
                            workspace_title(&cwd)
                        } else {
                            value.chars().take(72).collect()
                        },
                        cwd,
                    })
                    .await?;
                *app = App::new(session);
            }
            "resume" if !value.is_empty() => {
                *app = load_session(harness, find_session(harness, value).await?).await?;
            }
            "stop" => {
                harness.cancel(&app.session.id).await?;
                if app.is_running() {
                    app.run_state = RunState::Stopping;
                    app.status = "Stopping…".into();
                } else {
                    app.notice("No active run in this session.");
                }
            }
            "queue" if !value.is_empty() => {
                if app.is_running() {
                    harness.enqueue(&app.session.id, value.into()).await?;
                } else {
                    harness.start(&app.session.id, value.into()).await?;
                    app.run_state = RunState::Running;
                }
            }
            "steer" if !value.is_empty() => {
                harness.steer(&app.session.id, value.into()).await?;
            }
            _ => extension_command(name, value, app, harness, extensions, extension_tasks).await?,
        }
    } else if app.is_running() {
        harness.enqueue(&app.session.id, text).await?;
    } else {
        harness.start(&app.session.id, text).await?;
        app.run_state = RunState::Running;
        app.status = "Starting…".into();
        app.scroll_from_bottom = 0;
    }
    Ok(false)
}

#[derive(Default)]
struct ExtensionTasks {
    jobs: JoinSet<ExtensionReply>,
    command_active: bool,
    widget_active: bool,
    widgets_paused: bool,
}

enum ExtensionReply {
    Command(Result<String>),
    Widgets(Result<Vec<String>>),
}

async fn extension_command(
    name: &str,
    value: &str,
    app: &mut App,
    harness: &Harness,
    extensions: &Extensions,
    tasks: &mut ExtensionTasks,
) -> Result<()> {
    match name {
        "extensions" => app.notice(extensions.summary()),
        "widgets" if value == "pause" || value == "resume" => {
            tasks.widgets_paused = value == "pause";
            app.notice(if tasks.widgets_paused {
                "Widgets paused."
            } else {
                "Widgets resumed."
            });
        }
        "theme" if !value.is_empty() => {
            app.palette = Some(extensions.theme(value)?);
            app.dirty = true;
        }
        "prompt" | "skill" if !value.is_empty() => {
            let (asset, input) = value.split_once(char::is_whitespace).unwrap_or((value, ""));
            let mut prompt = extensions.prompt(name, asset)?;
            if !input.is_empty() {
                prompt.push_str("\n\n");
                prompt.push_str(input);
            }
            if app.is_running() {
                harness.enqueue(&app.session.id, prompt).await?;
            } else {
                harness.start(&app.session.id, prompt).await?;
                app.run_state = RunState::Running;
            }
        }
        "reload" if !value.is_empty() => {
            anyhow::ensure!(
                harness.status().await.active_agents == 0,
                "wait for all runs to finish before reloading native packages"
            );
            anyhow::ensure!(
                !tasks.command_active && !tasks.widget_active,
                "wait for extension callbacks before reloading"
            );
            let context = extensions.context(harness, &app.session);
            let extensions = extensions.clone();
            let package = value.to_owned();
            tasks.command_active = true;
            tasks.jobs.spawn(async move {
                ExtensionReply::Command(
                    extensions
                        .reload(&package, context)
                        .await
                        .map(|()| format!("Reloaded {package}.")),
                )
            });
        }
        _ => {
            let (package, command) = extensions.command(name)?;
            anyhow::ensure!(
                !tasks.command_active,
                "a package command is already running"
            );
            let context = extensions.context(harness, &app.session);
            let extensions = extensions.clone();
            let input = value.to_owned();
            tasks.command_active = true;
            tasks.jobs.spawn(async move {
                let result = extensions
                    .pool
                    .invoke_command(
                        &package,
                        &command,
                        serde_json::json!({"input":input}),
                        context,
                    )
                    .await
                    .and_then(|output| {
                        let text = output.as_str().map_or_else(
                            || serde_json::to_string_pretty(&output),
                            |text| Ok(text.to_owned()),
                        )?;
                        anyhow::ensure!(text.len() <= 64 * 1024, "command output exceeds 64 KiB");
                        Ok(text)
                    });
                ExtensionReply::Command(result)
            });
        }
    }
    Ok(())
}

async fn open_sessions(app: &mut App, harness: &Harness) -> Result<()> {
    let sessions = harness.list_sessions().await?;
    let selected = sessions
        .iter()
        .position(|session| session.id == app.session.id)
        .unwrap_or(0);
    app.overlay = Some(Overlay::Sessions { sessions, selected });
    Ok(())
}

async fn find_session(harness: &Harness, id: &str) -> Result<Session> {
    let sessions = harness.list_sessions().await?;
    let matches = sessions
        .into_iter()
        .filter(|session| session.id.starts_with(id))
        .collect::<Vec<_>>();
    anyhow::ensure!(
        matches.len() <= 1,
        "session prefix {id} is ambiguous; use the complete ID"
    );
    matches
        .into_iter()
        .next()
        .with_context(|| format!("session {id} does not exist"))
}

fn workspace_title(cwd: &std::path::Path) -> String {
    cwd.file_name()
        .map(|name| name.to_string_lossy().chars().take(60).collect::<String>())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "Workspace".into())
}

async fn load_session(harness: &Harness, session: Session) -> Result<App> {
    let mut app = App::new(session);
    recover_events(harness, &mut app).await?;
    app.run_state = if app.session.status == ejm_harness::SessionStatus::Running {
        RunState::Running
    } else {
        RunState::Idle
    };
    Ok(app)
}

async fn termination_signal() {
    #[cfg(unix)]
    {
        if let Ok(mut terminate) =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {},
                _ = terminate.recv() => {},
            }
            return;
        }
    }
    let _ = tokio::signal::ctrl_c().await;
}

async fn recover_events(harness: &Harness, app: &mut App) -> Result<()> {
    loop {
        let page = harness
            .events(&app.session.id, app.last_sequence, 512)
            .await?;
        if page.is_empty() {
            break;
        }
        for event in page {
            app.apply(event);
        }
        tokio::task::yield_now().await;
    }
    Ok(())
}

struct TerminalGuard {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl TerminalGuard {
    fn enter() -> Result<Self> {
        let previous_hook = panic::take_hook();
        panic::set_hook(Box::new(move |information| {
            restore_terminal();
            previous_hook(information);
        }));
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        if let Err(error) = execute!(
            stdout,
            EnterAlternateScreen,
            EnableMouseCapture,
            EnableBracketedPaste
        ) {
            restore_terminal();
            return Err(error.into());
        }
        match Terminal::new(CrosstermBackend::new(stdout)) {
            Ok(terminal) => Ok(Self { terminal }),
            Err(error) => {
                restore_terminal();
                Err(error.into())
            }
        }
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        restore_terminal();
    }
}

fn restore_terminal() {
    let _ = disable_raw_mode();
    let _ = execute!(
        io::stdout(),
        DisableBracketedPaste,
        DisableMouseCapture,
        LeaveAlternateScreen,
        Show
    );
}
