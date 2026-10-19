use std::{
    io::{self, Stdout},
    panic,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use crossterm::{
    cursor::Show,
    event::{
        DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
        Event as InputEvent, KeyCode, KeyEvent, KeyModifiers, MouseEventKind,
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
    commands,
    extensions::Extensions,
    remote::{backend::LocalBackend, host_ui::HostUi},
    render,
    terminal::{Modes, accepts_key},
};

pub async fn run(harness: Harness, args: &Args, extensions: &Extensions) -> Result<()> {
    let backend = LocalBackend::new(harness);
    let cancel = tokio_util::sync::CancellationToken::new();
    with_shutdown(
        &backend,
        cancel.clone(),
        run_terminal(&backend, args, extensions, cancel),
    )
    .await
}

pub(crate) async fn with_shutdown(
    backend: &LocalBackend,
    cancel: tokio_util::sync::CancellationToken,
    operation: impl std::future::Future<Output = Result<()>>,
) -> Result<()> {
    let guard = cancel.drop_guard();
    // The first poll registers SIGTERM before startup can own a raw terminal.
    let result = tokio::select! {
        biased;
        () = termination_signal() => Ok(()),
        result = operation => result,
    };
    drop(guard);
    backend.stop_sharing().await;
    result
}

async fn run_terminal(
    harness: &LocalBackend,
    args: &Args,
    extensions: &Extensions,
    cancel: tokio_util::sync::CancellationToken,
) -> Result<()> {
    let mut remote = HostUi::prepare(args, cancel).await?;
    let app = initial_app(harness, args, extensions).await?;
    let mut terminal = TerminalGuard::enter()?;
    if let Some(role) = crate::remote::host_role(args) {
        remote.share(harness, args, &app.session.id, role).await?;
        if remote.has_panel() {
            terminal.reset_input()?;
        }
    }
    run_app(harness, args, extensions, remote, app, terminal).await
}

pub(crate) async fn run_app(
    harness: &LocalBackend,
    args: &Args,
    extensions: &Extensions,
    mut remote: HostUi,
    mut app: App,
    mut terminal: TerminalGuard,
) -> Result<()> {
    let mut events = harness.subscribe();
    let mut extension_tasks = ExtensionTasks::default();
    let mut widget_tick = tokio::time::interval(Duration::from_secs(1));
    widget_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut tick = tokio::time::interval(Duration::from_millis(32));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut last_status_draw = Instant::now();
    'events: loop {
        tokio::select! {
            _ = tick.tick() => {
                let private_panel = remote.has_panel();
                remote.poll(&mut app, harness, crate::remote::now_ms()?).await;
                if private_panel != remote.has_panel() { terminal.reset_input()?; }
                for _ in 0..64 {
                    if !terminal.poll_input()? { break; }
                    let input = terminal.read_input()?;
                    if remote.handle_panel(&input) {
                        app.dirty = true;
                        if !remote.has_panel() { terminal.reset_input()?; break; }
                        continue;
                    }
                    if HostUi::is_command(&input, &app) {
                        match remote.command(&app.composer.text, harness, args, &app.session.id).await {
                            Ok(message) => { app.composer.take(); if let Some(message) = message { app.notice(message); } }
                            Err(error) => app.error(format!("{error:#}")),
                        }
                        if remote.has_panel() { terminal.reset_input()?; break; }
                        continue;
                    }
                    match handle_input(input, &mut app, harness, args, extensions, &mut extension_tasks).await {
                        Ok(true) => break 'events,
                        Ok(false) => {},
                        Err(error) => app.error(format!("{error:#}")),
                    }
                    if remote.session_changed(&app.session.id) {
                        remote.stop(harness).await;
                        app.notice("Sharing ended because the selected session changed.");
                    }
                }
                harness.settle(crate::remote::now_ms()?).await?;
                if app.dirty || (app.is_running() && last_status_draw.elapsed() >= Duration::from_millis(250)) {
                    draw_app(&mut terminal, &mut app, args, &remote, harness).await?;
                    last_status_draw = Instant::now();
                }
            }
            event = events.recv() => match event {
                Ok(event) => app.apply(event),
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                    recover_events(harness, &mut app).await?;
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            },
            _ = widget_tick.tick(), if extensions.has_widgets() && !extension_tasks.widget_active && !extension_tasks.widgets_paused => {
                extension_tasks.widget_active = true;
                let extensions = extensions.clone();
                let context = extensions.context(harness, &app.session);
                extension_tasks.jobs.spawn(async move { ExtensionReply::Widgets(extensions.widgets(context).await) });
            }
            reply = extension_tasks.jobs.join_next(), if !extension_tasks.jobs.is_empty() => {
                apply_extension_reply(reply, &mut extension_tasks, &mut app);
                app.dirty = true;
            }
        }
    }
    remote.stop(harness).await;
    Ok(())
}

async fn initial_app(harness: &LocalBackend, args: &Args, extensions: &Extensions) -> Result<App> {
    let session = if let Some(id) = &args.session {
        find_session(harness, id).await?
    } else {
        let cwd = args.cwd()?;
        let title = workspace_title(&cwd);
        harness
            .create_session(SessionOptions { cwd, title })
            .await?
    };
    let mut app = load_session(harness, session).await?;
    app.palette = extensions.palette;
    app.commands = commands::catalog(&extensions.commands());
    if extensions.count() > 0 {
        app.notice(format!(
            "{} trusted native packages enabled. /extensions shows grants.",
            extensions.count()
        ));
    }
    Ok(app)
}

async fn draw_app(
    terminal: &mut TerminalGuard,
    app: &mut App,
    args: &Args,
    remote: &HostUi,
    harness: &LocalBackend,
) -> Result<()> {
    let snapshot = harness.status().await;
    terminal.terminal.draw(|frame| {
        render::draw(frame, app, args, &snapshot);
        remote.draw(frame);
    })?;
    app.dirty = false;
    Ok(())
}

async fn handle_input(
    input: InputEvent,
    app: &mut App,
    harness: &LocalBackend,
    args: &Args,
    extensions: &Extensions,
    extension_tasks: &mut ExtensionTasks,
) -> Result<bool> {
    app.dirty = true;
    match input {
        InputEvent::Key(key) if accepts_key(key) => {
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
            if handle_picker_key(key, app) {
                return Ok(false);
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
                    KeyCode::Char('j') => {
                        app.composer.insert("\n");
                        app.command_picker = commands::CommandPicker::default();
                    }
                    KeyCode::Char('a') => app.composer.home(),
                    KeyCode::Char('e') => app.composer.end(),
                    KeyCode::Char('u') => {
                        app.composer.text.clear();
                        app.composer.cursor = 0;
                        app.command_picker = commands::CommandPicker::default();
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
                        edit_composer(key, app);
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
                    _ => edit_composer(key, app),
                }
            }
        }
        InputEvent::Paste(text) => {
            app.command_picker = commands::CommandPicker::default();
            app.composer.insert(&text);
        }
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

pub(crate) fn handle_picker_key(key: KeyEvent, app: &mut App) -> bool {
    if key
        .modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SHIFT)
        || app.command_query().is_none()
    {
        return false;
    }
    let matches = app.command_matches();
    let count = matches.len();
    let exact = matches
        .iter()
        .any(|entry| app.composer.text.strip_prefix('/') == Some(entry.name.as_str()));
    match key.code {
        KeyCode::Up => app.command_picker.selected = app.command_picker.selected.saturating_sub(1),
        KeyCode::Down => {
            app.command_picker.selected =
                (app.command_picker.selected + 1).min(count.saturating_sub(1));
        }
        KeyCode::Tab => app.complete_command(),
        KeyCode::Esc => app.command_picker.dismissed = true,
        KeyCode::Enter if count > 0 && !exact => app.complete_command(),
        _ => return false,
    }
    true
}

pub(crate) fn edit_composer(key: KeyEvent, app: &mut App) {
    if matches!(
        key.code,
        KeyCode::Char(_)
            | KeyCode::Backspace
            | KeyCode::Delete
            | KeyCode::Enter
            | KeyCode::Up
            | KeyCode::Down
    ) {
        app.command_picker = commands::CommandPicker::default();
    }
    match key.code {
        KeyCode::Enter => app.composer.insert("\n"),
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
        KeyCode::PageUp => app.scroll_from_bottom = app.scroll_from_bottom.saturating_add(12),
        KeyCode::PageDown => app.scroll_from_bottom = app.scroll_from_bottom.saturating_sub(12),
        KeyCode::Esc => app.scroll_from_bottom = 0,
        _ => {}
    }
}

async fn handle_overlay_key(key: KeyEvent, app: &mut App, harness: &LocalBackend) -> Result<bool> {
    match key.code {
        KeyCode::Esc => app.overlay = None,
        KeyCode::Up | KeyCode::Down | KeyCode::PageUp | KeyCode::PageDown => {
            let selection = match &mut app.overlay {
                Some(Overlay::Sessions { sessions, selected }) => Some((selected, sessions.len())),
                Some(Overlay::Help { selected }) => Some((selected, app.commands.len())),
                None => None,
            };
            if let Some((selected, count)) = selection {
                let step = if matches!(key.code, KeyCode::PageUp | KeyCode::PageDown) {
                    8
                } else {
                    1
                };
                *selected = if matches!(key.code, KeyCode::Up | KeyCode::PageUp) {
                    selected.saturating_sub(step)
                } else {
                    selected.saturating_add(step).min(count.saturating_sub(1))
                };
            }
        }
        KeyCode::Enter => {
            if let Some(Overlay::Help { selected }) = &app.overlay {
                if let Some(command) = app.commands.get(*selected) {
                    app.composer.replace(&command.completion());
                    app.command_picker.dismissed = true;
                }
                app.overlay = None;
                return Ok(false);
            }
            let session = if let Some(Overlay::Sessions { sessions, selected }) = &app.overlay {
                sessions.get(*selected).cloned()
            } else {
                None
            };
            if let Some(session) = session {
                app.switch_session(load_session(harness, session).await?);
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
    harness: &LocalBackend,
    args: &Args,
    extensions: &Extensions,
    extension_tasks: &mut ExtensionTasks,
) -> Result<bool> {
    if let Some(command) = text.strip_prefix('/') {
        let command = commands::parse(command)?;
        let name = command.name.as_str();
        let value = command.value;
        match name {
            "help" => app.overlay = Some(Overlay::Help { selected: 0 }),
            "status" => show_status(app, harness, args).await,
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
                app.switch_session(App::new(session));
            }
            "resume" if !value.is_empty() => {
                app.switch_session(
                    load_session(harness, find_session(harness, value).await?).await?,
                );
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

async fn show_status(app: &mut App, harness: &LocalBackend, args: &Args) {
    let status = harness.status().await;
    app.notice(format!(
        "Connection: {}\nModel: {}\nAccount: {}\nSession: {}\nGrants: {}\nAgents: {} active\nModels: {} active, {} queued\nTools: {} active, {} queued",
        if app.connection_ready { "ready" } else { "not connected" },
        args.provider_label(), args.account.as_deref().unwrap_or("environment or none"),
        app.session.id, args.grants(), status.active_agents, status.model_requests,
        status.waiting_model_requests, status.tools.running, status.tools.queued,
    ));
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

fn apply_extension_reply(
    reply: Option<Result<ExtensionReply, tokio::task::JoinError>>,
    extension_tasks: &mut ExtensionTasks,
    app: &mut App,
) {
    match reply {
        Some(Ok(ExtensionReply::Widgets(result))) => {
            extension_tasks.widget_active = false;
            match result {
                Ok(lines) => app.widget_lines = lines,
                Err(error) => {
                    extension_tasks.widgets_paused = true;
                    app.widget_lines = vec![format!(
                        "Widget paused: {}. /widgets resume to retry.",
                        render::safe_text(&error.to_string())
                    )];
                }
            }
        }
        Some(Ok(ExtensionReply::Command(result))) => {
            extension_tasks.command_active = false;
            match result {
                Ok(text) => app.notice(text),
                Err(error) => app.error(format!("{error:#}")),
            }
        }
        Some(Err(error)) => {
            extension_tasks.widget_active = false;
            extension_tasks.command_active = false;
            app.error(format!("Extension task failed: {error}"));
        }
        None => {}
    }
}

async fn extension_command(
    name: &str,
    value: &str,
    app: &mut App,
    harness: &LocalBackend,
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

async fn open_sessions(app: &mut App, harness: &LocalBackend) -> Result<()> {
    let sessions = harness.list_sessions().await?;
    let selected = sessions
        .iter()
        .position(|session| session.id == app.session.id)
        .unwrap_or(0);
    app.overlay = Some(Overlay::Sessions { sessions, selected });
    Ok(())
}

async fn find_session(harness: &LocalBackend, id: &str) -> Result<Session> {
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

async fn load_session(harness: &LocalBackend, session: Session) -> Result<App> {
    crate::remote::backend::load_app(harness, session).await
}

pub(crate) async fn termination_signal() {
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

async fn recover_events(harness: &LocalBackend, app: &mut App) -> Result<()> {
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

static PANIC_HOOK: std::sync::Once = std::sync::Once::new();
static PANIC_BASELINE: std::sync::Mutex<Option<Modes>> = std::sync::Mutex::new(None);

pub(crate) struct TerminalGuard {
    pub(crate) terminal: Terminal<CrosstermBackend<Stdout>>,
    baseline: Modes,
    input: crate::terminal_input::Input,
}

impl TerminalGuard {
    pub(crate) fn enter() -> Result<Self> {
        let baseline = Modes::capture()?;
        *PANIC_BASELINE
            .lock()
            .map_err(|_| anyhow::anyhow!("terminal recovery state is unavailable"))? =
            Some(baseline.clone());
        PANIC_HOOK.call_once(|| {
            let previous_hook = panic::take_hook();
            panic::set_hook(Box::new(move |information| {
                let _ = crate::terminal::flush_pending_input();
                restore_terminal();
                if let Ok(saved) = PANIC_BASELINE.try_lock()
                    && let Some(baseline) = saved.as_ref()
                {
                    let _ = baseline.restore();
                }
                previous_hook(information);
            }));
        });
        if let Err(error) = enable_raw_mode() {
            restore_terminal();
            let _ = baseline.restore();
            return Err(error.into());
        }
        let input = match crate::terminal_input::Input::new() {
            Ok(input) => input,
            Err(error) => {
                restore_terminal();
                let _ = baseline.restore();
                return Err(error);
            }
        };
        let mut stdout = io::stdout();
        if let Err(error) = execute!(
            stdout,
            EnterAlternateScreen,
            EnableMouseCapture,
            EnableBracketedPaste
        ) {
            restore_terminal();
            let _ = baseline.restore();
            return Err(error.into());
        }
        match Terminal::new(CrosstermBackend::new(stdout)) {
            Ok(terminal) => Ok(Self {
                terminal,
                baseline,
                input,
            }),
            Err(error) => {
                restore_terminal();
                let _ = baseline.restore();
                Err(error.into())
            }
        }
    }

    pub(crate) fn poll_input(&mut self) -> Result<bool> {
        self.input.poll()
    }

    pub(crate) fn read_input(&mut self) -> Result<InputEvent> {
        self.input.read()
    }

    pub(crate) fn reset_input(&mut self) -> Result<()> {
        self.input.reset()
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        // A reader panic may still hold its package mutex. Panic hooks only
        // flush OS input; ordinary exits can also discard parsed events.
        if !std::thread::panicking() {
            let _ = self.reset_input();
        }
        let _ = crate::terminal::flush_pending_input();
        restore_terminal();
        let _ = self.baseline.restore();
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

#[cfg(test)]
#[expect(
    clippy::unwrap_used,
    reason = "Invalid offline terminal fixtures must fail the test immediately."
)]
mod tests {
    use super::*;
    use clap::Parser;
    use ejm_harness::{HarnessConfig, ReplayProvider};
    use std::sync::Arc;

    struct TestUi {
        _state: tempfile::TempDir,
        harness: LocalBackend,
        app: App,
        args: Args,
        extensions: Extensions,
        extension_tasks: ExtensionTasks,
    }

    impl TestUi {
        async fn new() -> Self {
            let state = tempfile::tempdir().unwrap();
            let harness = Harness::with_hooks(
                HarnessConfig {
                    state_dir: state.path().into(),
                    ..HarnessConfig::default()
                },
                Arc::new(ReplayProvider::echo()),
                Vec::new(),
                Vec::new(),
            )
            .unwrap();
            let session = harness
                .create_session(SessionOptions {
                    cwd: state.path().into(),
                    title: "fixture".into(),
                })
                .await
                .unwrap();
            let args = Args::try_parse_from([
                "ejm-cli",
                "--replay",
                "--state-dir",
                state.path().to_str().unwrap(),
            ])
            .unwrap();
            Self {
                args,
                _state: state,
                harness: LocalBackend::new(harness),
                app: App::new(session),
                extensions: Extensions::empty().unwrap(),
                extension_tasks: ExtensionTasks::default(),
            }
        }

        async fn input(&mut self, input: InputEvent) -> Result<bool> {
            handle_input(
                input,
                &mut self.app,
                &self.harness,
                &self.args,
                &self.extensions,
                &mut self.extension_tasks,
            )
            .await
        }

        async fn key(&mut self, code: KeyCode) -> Result<bool> {
            self.input(InputEvent::Key(KeyEvent::new(code, KeyModifiers::NONE)))
                .await
        }
    }

    #[tokio::test]
    async fn slash_picker_completes_without_running_and_escape_preserves_draft() {
        let mut ui = TestUi::new().await;
        ui.app.composer.insert("/s");
        ui.key(KeyCode::Down).await.unwrap();
        ui.key(KeyCode::Down).await.unwrap();
        ui.key(KeyCode::Tab).await.unwrap();
        assert_eq!(ui.app.composer.text, "/steer ");
        assert!(ui.app.blocks.is_empty());
        assert!(
            ui.harness
                .events(&ui.app.session.id, 0, 100)
                .await
                .unwrap()
                .is_empty()
        );
        ui.app.composer.replace("/st");
        ui.app.command_picker.dismissed = false;
        ui.key(KeyCode::Esc).await.unwrap();
        assert_eq!(ui.app.composer.text, "/st");
        assert!(ui.app.command_query().is_none());
        ui.key(KeyCode::Left).await.unwrap();
        ui.key(KeyCode::Right).await.unwrap();
        assert!(ui.app.command_query().is_none());
        ui.key(KeyCode::Char('a')).await.unwrap();
        assert_eq!(ui.app.command_query(), Some("sta"));
    }

    #[tokio::test]
    async fn invalid_commands_keep_the_draft_and_do_not_start_work() {
        let mut ui = TestUi::new().await;
        ui.app.composer.insert("/resume");
        let error = ui.key(KeyCode::Enter).await.unwrap_err();
        assert!(error.to_string().contains("Usage: /resume ID"));
        assert_eq!(ui.app.composer.text, "/resume");
        assert!(
            ui.harness
                .events(&ui.app.session.id, 0, 100)
                .await
                .unwrap()
                .is_empty()
        );
        ui.app.composer.replace("/help");
        ui.key(KeyCode::Enter).await.unwrap();
        assert!(matches!(ui.app.overlay, Some(Overlay::Help { .. })));
    }

    #[tokio::test]
    async fn paste_never_executes_and_history_works_outside_the_picker() {
        let mut ui = TestUi::new().await;
        ui.app.composer.insert("previous prompt");
        ui.app.composer.take();
        ui.input(InputEvent::Paste("/stop\n🦀".into()))
            .await
            .unwrap();
        assert_eq!(ui.app.composer.text, "/stop\n🦀");
        assert!(ui.app.command_query().is_none());
        assert!(
            ui.harness
                .events(&ui.app.session.id, 0, 100)
                .await
                .unwrap()
                .is_empty()
        );
        ui.key(KeyCode::Up).await.unwrap();
        assert_eq!(ui.app.composer.text, "previous prompt");
        ui.key(KeyCode::Down).await.unwrap();
        assert_eq!(ui.app.composer.text, "/stop\n🦀");
    }

    #[tokio::test]
    async fn help_completes_without_execution_and_session_switch_keeps_host_state() {
        let mut ui = TestUi::new().await;
        let previous_session = ui.app.session.id.clone();
        let catalog = ui
            .app
            .commands
            .iter()
            .map(commands::CommandEntry::usage)
            .collect::<Vec<_>>();
        ui.app.widget_lines = vec!["fixture widget".into()];
        ui.app.composer.replace("/help");
        ui.key(KeyCode::Enter).await.unwrap();
        ui.key(KeyCode::Down).await.unwrap();
        ui.key(KeyCode::Down).await.unwrap();
        ui.key(KeyCode::Down).await.unwrap();
        ui.key(KeyCode::Enter).await.unwrap();
        assert_eq!(ui.app.composer.text, "/new ");
        assert!(ui.app.overlay.is_none());
        assert_eq!(ui.app.session.id, previous_session);
        assert_eq!(ui.harness.list_sessions().await.unwrap().len(), 1);

        ui.app.composer.replace("/new next session");
        ui.key(KeyCode::Enter).await.unwrap();
        assert_ne!(ui.app.session.id, previous_session);
        assert_eq!(ui.app.session.title, "next session");
        assert_eq!(ui.app.widget_lines, ["fixture widget"]);
        assert_eq!(
            ui.app
                .commands
                .iter()
                .map(commands::CommandEntry::usage)
                .collect::<Vec<_>>(),
            catalog
        );
        assert_eq!(ui.app.composer.text, "");
    }
}
