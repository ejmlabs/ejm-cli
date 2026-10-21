use std::{
    future::Future,
    io::{self, Stdout},
    panic,
    pin::Pin,
    time::{Duration, Instant},
};

use anyhow::{Result, ensure};
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
use ejm_harness_exec::Executor;
use ratatui::{Terminal, backend::CrosstermBackend};
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

use crate::{
    app::{App, LOGIN_PROVIDERS, Overlay, RunState},
    args::{Args, ProviderKind},
    commands,
    extensions::Extensions,
    interaction::{self, ConnectionReply, ConnectionTasks},
    remote::{backend::LocalBackend, host_ui::HostUi},
    render,
    terminal::{Modes, accepts_key},
};

pub async fn run(
    harness: Harness,
    args: &mut Args,
    extensions: &Extensions,
    executor: Executor,
    shutdown: TerminationSignal,
) -> Result<()> {
    let backend = LocalBackend::new(harness);
    let cancel = CancellationToken::new();
    let operation = async {
        let remote = HostUi::prepare(args, cancel.clone()).await?;
        let app = initial_app(&backend, args, extensions).await?;
        let connection_tasks = initial_connection(executor, args, extensions)?;
        run_prepared(
            backend.clone(),
            args,
            extensions,
            remote,
            app,
            connection_tasks,
            shutdown,
        )
        .await
    };
    with_shutdown(&backend, cancel.clone(), Box::pin(operation)).await
}

pub(crate) async fn with_shutdown(
    backend: &LocalBackend,
    cancel: CancellationToken,
    operation: impl Future<Output = Result<()>>,
) -> Result<()> {
    let guard = cancel.drop_guard();
    let result = tokio::select! {
        biased;
        () = termination_signal() => Ok(()),
        result = operation => result,
    };
    drop(guard);
    backend.stop_sharing().await;
    result
}

async fn run_prepared(
    harness: LocalBackend,
    args: &mut Args,
    extensions: &Extensions,
    remote: HostUi,
    app: App,
    connection_tasks: ConnectionTasks,
    shutdown: TerminationSignal,
) -> Result<()> {
    let terminal = TerminalGuard::enter()?;
    run_app(
        harness,
        args,
        extensions,
        remote,
        app,
        connection_tasks,
        shutdown,
        terminal,
    )
    .await
}

#[expect(
    clippy::too_many_arguments,
    reason = "The prepared terminal and owned tasks retain their existing lifetimes across the UI loop."
)]
pub(crate) async fn run_app(
    harness: LocalBackend,
    args: &mut Args,
    extensions: &Extensions,
    mut remote: HostUi,
    mut app: App,
    mut connection_tasks: ConnectionTasks,
    mut shutdown: TerminationSignal,
    mut terminal: TerminalGuard,
) -> Result<()> {
    let mut selected_view = selected_view();
    let mut session_pages = crate::sessions::SessionPages::default();
    selected_view.follow(&harness, &app);
    let mut extension_tasks = ExtensionTasks::default();
    let mut widget_tick = interval(Duration::from_secs(1));
    start_share(&mut remote, &harness, args, &app, &mut terminal).await?;
    let mut tick = interval(Duration::from_millis(16));
    let mut paint = interval(Duration::from_millis(32));
    let mut last_status_draw = Instant::now();
    'events: loop {
        tokio::select! {
            biased;
            () = &mut shutdown => break,
            _ = tick.tick() => {
                selected_view.follow(&harness, &app);
                session_pages.follow(&harness, &app);
                selected_view.apply_batch(&harness, &mut app);
                let private_panel = remote.has_panel();
                remote.poll(&mut app, &harness, crate::remote::now_ms()?).await;
                if private_panel != remote.has_panel() { terminal.reset_input()?; }
                remote.settle(&harness, crate::remote::now_ms()?).await?;
                for _ in 0..64 {
                    if !terminal.poll_input()? { break; }
                    let input = terminal.read_input()?;
                    if remote.handle_panel(&input) {
                        app.dirty = true;
                        if !remote.has_panel() { terminal.reset_input()?; break; }
                        continue;
                    }
                    if HostUi::is_command(&input, &app) {
                        match remote.command(&app.composer.text, &harness, args, &app.session.id).await {
                            Ok(message) => { app.composer.take(); if let Some(message) = message { app.notice(message); } }
                            Err(error) => app.error(format!("{error:#}")),
                        }
                        if remote.has_panel() { terminal.reset_input()?; break; }
                        continue;
                    }
                    match handle_input(input, &mut app, &harness, args, extensions, &mut extension_tasks, &mut connection_tasks).await {
                        Ok(true) => break 'events,
                        Ok(false) => {},
                        Err(error) => app.error(format!("{error:#}")),
                    }
                    if let Some(request) = connection_tasks.external_login.take() {
                        let draft = request.draft.clone();
                        let handoff = claude_handoff(&mut terminal, request, extensions, connection_tasks.executor()).await?;
                        // Discard the parent listener's copy of the child's Ctrl+C.
                        shutdown = termination_signal();
                        match handoff {
                            Handoff::Shutdown => break 'events,
                            Handoff::Finished(result) => resolve_connection(result, Some(draft), &mut app, &harness, args, true).await,
                        }
                        break;
                    }
                    if remote.session_changed(&app.session.id) { remote.stop(&harness).await; app.notice("Sharing ended because the selected session changed."); }
                    selected_view.follow(&harness, &app);
                    session_pages.follow(&harness, &app);
                }
            }
            _ = paint.tick() => {
                if app.dirty || (app.is_running() && last_status_draw.elapsed() >= Duration::from_millis(250)) {
                    draw_app(&mut terminal, &mut app, args, &remote, &harness).await?;
                    last_status_draw = Instant::now();
                }
            }
            event = crate::views::receive(&mut selected_view.events) => match event {
                Ok(event) => selected_view.event(event, &harness, &mut app),
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => selected_view.recover(&harness, &app),
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            },
            reply = selected_view.jobs.join_next(), if !selected_view.jobs.is_empty() => {
                selected_view.joined(reply, &harness, &mut app);
            },
            reply = session_pages.jobs.join_next(), if !session_pages.jobs.is_empty() => {
                crate::sessions::SessionPages::joined(reply, &mut app);
            }
            progress = connection_tasks.progress.recv() => {
                if let Some(progress) = progress { app.status = progress.message().into(); app.dirty = true; }
            }
            Some(reply) = connection_tasks.jobs.join_next(), if !connection_tasks.jobs.is_empty() => {
                let result = reply.unwrap_or_else(|error| Err(anyhow::anyhow!("Command task failed: {error}")));
                let (pending, result) = connection_tasks.finish(result);
                resolve_connection(result, pending.map(|pending| pending.draft), &mut app, &harness, args, !extension_tasks.command_active && !extension_tasks.widget_active).await;
            }
            _ = widget_tick.tick(), if extensions.has_widgets() && !extension_tasks.widget_active && !extension_tasks.widgets_paused && !connection_tasks.changing() => {
                start_widgets(&mut extension_tasks, extensions, &harness, &app.session);
            }
            reply = extension_tasks.jobs.join_next(), if !extension_tasks.jobs.is_empty() => {
                apply_extension_reply(reply, &mut extension_tasks, &mut app);
                app.dirty = true;
            }
            () = tokio::task::yield_now(), if selected_view.has_pending_batch() => {
                selected_view.apply_batch(&harness, &mut app);
            }
        }
    }
    remote.stop(&harness).await;
    connection_tasks.shutdown().await;
    Ok(())
}

fn selected_view() -> crate::views::SelectedView {
    #[cfg(not(test))]
    let view = crate::views::SelectedView::default();
    #[cfg(test)]
    let view = crate::views::SelectedView::with_trace(capacity_fixture::paint_trace());
    view
}

async fn start_share(
    remote: &mut HostUi,
    harness: &LocalBackend,
    args: &Args,
    app: &App,
    terminal: &mut TerminalGuard,
) -> Result<()> {
    if let Some(role) = crate::remote::host_role(args) {
        remote.share(harness, args, &app.session.id, role).await?;
        if remote.has_panel() {
            terminal.reset_input()?;
        }
    }
    Ok(())
}

async fn draw_app(
    terminal: &mut TerminalGuard,
    app: &mut App,
    args: &Args,
    remote: &HostUi,
    harness: &LocalBackend,
) -> Result<()> {
    let snapshot = harness.status().await;
    app.capacity = harness.capacity_snapshot();
    let completed_frame = terminal.terminal.draw(|frame| {
        render::draw(frame, app, args, &snapshot);
        remote.draw(frame);
    })?;
    #[cfg(test)]
    let painted = capacity_fixture::paint_clock();
    #[cfg(test)]
    capacity_fixture::painted(app, completed_frame.buffer, painted);
    #[cfg(not(test))]
    let _ = completed_frame;
    app.dirty = false;
    Ok(())
}

fn interval(period: Duration) -> tokio::time::Interval {
    let mut interval = tokio::time::interval(period);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    interval
}

fn initial_connection(
    executor: Executor,
    args: &Args,
    extensions: &Extensions,
) -> Result<ConnectionTasks> {
    let mut tasks = ConnectionTasks::new(executor);
    interaction::connect(&mut tasks, String::new(), args.clone(), extensions.clone())?;
    Ok(tasks)
}

async fn resolve_connection(
    result: Result<ConnectionReply>,
    draft: Option<String>,
    app: &mut App,
    harness: &Harness,
    args: &mut Args,
    callbacks_idle: bool,
) {
    if let Err(error) = interaction::apply_reply(result, app, harness, args, callbacks_idle).await {
        if app.composer.text.is_empty()
            && let Some(draft) = draft
        {
            app.composer.replace(&draft);
            app.command_picker.dismissed = true;
        }
        app.error(format!("{error:#}"));
        if !app.connection_ready {
            app.notice("No model connection. Use /login to choose a provider, /account REGISTRATION or /model PROVIDER MODEL. Type / to see commands.");
        }
    }
    app.status = if app.connection_ready {
        "Ready"
    } else {
        "Not connected"
    }
    .into();
    app.dirty = true;
}

enum Handoff {
    Finished(Result<ConnectionReply>),
    Shutdown,
}

async fn claude_handoff(
    terminal: &mut TerminalGuard,
    request: interaction::ClaudeLogin,
    extensions: &Extensions,
    executor: &Executor,
) -> Result<Handoff> {
    terminal.suspend()?;
    eprintln!("Claude Code is taking over sign-in. Ctrl+C cancels and returns to ejm-cli.");
    let cancel = CancellationToken::new();
    let login = interaction::complete_claude_login(request, extensions, executor, cancel.clone());
    tokio::pin!(login);
    let outcome = tokio::select! {
        biased;
        signal = auth_signal() => {
            cancel.cancel();
            let _ = login.await;
            match signal {
                ShutdownSignal::Interrupt => Handoff::Finished(Err(anyhow::anyhow!("Claude sign-in cancelled."))),
                ShutdownSignal::Terminate => Handoff::Shutdown,
            }
        }
        result = &mut login => Handoff::Finished(result),
    };
    terminal.resume()?;
    Ok(outcome)
}

async fn initial_app(harness: &LocalBackend, args: &Args, extensions: &Extensions) -> Result<App> {
    let session = if let Some(id) = &args.session {
        find_session(harness, id).await?
    } else {
        let cwd = args.cwd()?;
        harness
            .create_session(SessionOptions {
                title: workspace_title(&cwd),
                cwd,
            })
            .await?
    };
    harness.select_session(&session.id).await?;
    let mut app = App::new(session);
    app.palette = extensions.palette;
    app.commands = commands::catalog(&extensions.commands());
    app.connection_ready = false;
    app.status = "Connecting…".into();
    app.notice("Connecting to the configured model. Ctrl+C cancels. Type / to see commands.");
    if extensions.count() > 0 {
        app.notice(format!(
            "{} trusted native packages enabled. /extensions shows grants.",
            extensions.count()
        ));
    }
    Ok(app)
}

async fn handle_input(
    input: InputEvent,
    app: &mut App,
    harness: &LocalBackend,
    args: &mut Args,
    extensions: &Extensions,
    extension_tasks: &mut ExtensionTasks,
    connection_tasks: &mut ConnectionTasks,
) -> Result<bool> {
    app.dirty = true;
    match input {
        InputEvent::Key(key) if accepts_key(key) => {
            return handle_key(
                key,
                app,
                harness,
                args,
                extensions,
                extension_tasks,
                connection_tasks,
            )
            .await;
        }
        InputEvent::Paste(text) if !matches!(app.overlay, Some(Overlay::Login { .. })) => {
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

async fn handle_key(
    key: KeyEvent,
    app: &mut App,
    harness: &LocalBackend,
    args: &mut Args,
    extensions: &Extensions,
    extension_tasks: &mut ExtensionTasks,
    connection_tasks: &mut ConnectionTasks,
) -> Result<bool> {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        if connection_tasks.cancel() {
            app.status = "Cancelling command…".into();
            return Ok(false);
        }
        if app.run_state != RunState::Stopping {
            #[cfg(test)]
            capacity_fixture::stopping(&app.session.id);
            if let Some(run) = harness.cancel(&app.session.id).await? {
                app.acknowledge_stop(Some(run));
                return Ok(false);
            }
        }
        return Ok(true);
    }
    if app.overlay.is_some() {
        return match handle_overlay_key(key, app, harness).await? {
            Some(OverlayAction::Submit(text)) => {
                submit(
                    text,
                    app,
                    harness,
                    args,
                    extensions,
                    extension_tasks,
                    connection_tasks,
                )
                .await
            }
            Some(OverlayAction::Login(provider)) => {
                if provider == ProviderKind::Claude {
                    ensure!(
                        !extension_tasks.command_active && !extension_tasks.widget_active,
                        "wait for extension callbacks before Claude sign-in"
                    );
                }
                interaction::login(provider, app, harness, args, extensions, connection_tasks)
                    .await?;
                app.composer.take();
                Ok(false)
            }
            None => Ok(false),
        };
    }
    if handle_picker_key(key, app) {
        return Ok(false);
    }
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        return handle_control_key(key, app, harness).await;
    }
    if key.code == KeyCode::Enter
        && !key
            .modifiers
            .intersects(KeyModifiers::ALT | KeyModifiers::SHIFT)
    {
        let text = app.composer.text.clone();
        if !text.trim().is_empty() {
            if submit(
                text,
                app,
                harness,
                args,
                extensions,
                extension_tasks,
                connection_tasks,
            )
            .await?
            {
                return Ok(true);
            }
            if !matches!(app.overlay, Some(Overlay::Login { .. })) {
                app.composer.take();
            }
        }
    } else {
        edit_composer(key, app);
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

async fn handle_control_key(key: KeyEvent, app: &mut App, harness: &LocalBackend) -> Result<bool> {
    match key.code {
        KeyCode::Char('d') if app.composer.text.is_empty() => return Ok(true),
        KeyCode::Char('s') => open_sessions(app),
        KeyCode::Char('k') => {
            if app.is_running() && !app.composer.text.trim().is_empty() {
                harness
                    .steer(&app.session.id, app.composer.text.clone())
                    .await?;
                app.composer.take();
            } else {
                app.notice("Steering needs an active run and a message in the composer.");
            }
        }
        KeyCode::Char('j') => {
            app.composer.insert("\n");
            app.command_picker = commands::CommandPicker::default();
        }
        KeyCode::Char('a') => app.composer.home(),
        KeyCode::Char('e') => app.composer.end(),
        KeyCode::Char('u') => {
            app.composer.replace("");
            app.command_picker = commands::CommandPicker::default();
        }
        _ => {}
    }
    Ok(false)
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

enum OverlayAction {
    Submit(String),
    Login(ProviderKind),
}

async fn handle_overlay_key(
    key: KeyEvent,
    app: &mut App,
    harness: &LocalBackend,
) -> Result<Option<OverlayAction>> {
    if let Some(Overlay::Sessions { picker }) = &mut app.overlay
        && handle_session_picker_key(key, picker)
    {
        return Ok(None);
    }
    match key.code {
        KeyCode::Esc => app.overlay = None,
        KeyCode::Up | KeyCode::Down | KeyCode::PageUp | KeyCode::PageDown => {
            let selection = match &mut app.overlay {
                Some(Overlay::Sessions { picker }) => {
                    Some((&mut picker.selected, picker.sessions.len()))
                }
                Some(Overlay::Models { models, selected }) => Some((selected, models.len())),
                Some(Overlay::Help { selected }) => Some((selected, app.commands.len())),
                Some(Overlay::Login { selected }) => Some((selected, LOGIN_PROVIDERS.len())),
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
            if let Some(Overlay::Login { selected }) = &app.overlay {
                let action = LOGIN_PROVIDERS
                    .get(*selected)
                    .map(|(provider, _)| OverlayAction::Login(*provider));
                app.overlay = None;
                return Ok(action);
            }
            if let Some(Overlay::Models { models, selected }) = &app.overlay {
                let command = models
                    .get(*selected)
                    .map(|model| OverlayAction::Submit(format!("/model {}", model.slug)));
                app.overlay = None;
                return Ok(command);
            }
            if let Some(Overlay::Help { selected }) = &app.overlay {
                if let Some(command) = app.commands.get(*selected) {
                    app.composer.replace(&command.completion());
                    app.command_picker.dismissed = true;
                }
                app.overlay = None;
                return Ok(None);
            }
            let session = if let Some(Overlay::Sessions { picker }) = &app.overlay {
                picker.sessions.get(picker.selected).cloned()
            } else {
                None
            };
            if let Some(session) = session {
                harness.stop_share().await;
                harness.select_session(&session.id).await?;
                app.switch_session(App::new(session));
            } else {
                app.overlay = None;
            }
        }
        _ => {}
    }
    Ok(None)
}

fn handle_session_picker_key(key: KeyEvent, picker: &mut crate::sessions::SessionPicker) -> bool {
    match key.code {
        KeyCode::Char(character)
            if !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
        {
            if picker.query.len() + character.len_utf8() <= 256 {
                picker.query.push(character);
                picker.query_changed();
            }
            true
        }
        KeyCode::Backspace => {
            picker.query.pop();
            picker.query_changed();
            true
        }
        KeyCode::Home => {
            picker.first_page();
            true
        }
        KeyCode::PageDown => {
            picker.next_page();
            true
        }
        KeyCode::PageUp => {
            picker.previous_page();
            true
        }
        KeyCode::Down if picker.selected + 1 >= picker.sessions.len() => {
            picker.next_page();
            true
        }
        KeyCode::Up if picker.selected == 0 => {
            picker.previous_page();
            true
        }
        KeyCode::Enter if picker.pending => true,
        _ => false,
    }
}

async fn submit(
    text: String,
    app: &mut App,
    harness: &LocalBackend,
    args: &mut Args,
    extensions: &Extensions,
    extension_tasks: &mut ExtensionTasks,
    connection_tasks: &mut ConnectionTasks,
) -> Result<bool> {
    if let Some(command) = text.strip_prefix('/') {
        let command = commands::parse(command)?;
        let name = command.name.as_str();
        let value = command.value;
        match name {
            "help" => app.overlay = Some(Overlay::Help { selected: 0 }),
            "quit" | "exit" => return Ok(true),
            "sessions" | "agents" => open_sessions(app),
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
                harness.stop_share().await;
                harness.select_session(&session.id).await?;
                app.switch_session(App::new(session));
            }
            "resume" if !value.is_empty() => {
                let session = find_session(harness, value).await?;
                harness.stop_share().await;
                harness.select_session(&session.id).await?;
                app.switch_session(App::new(session));
            }
            "stop" => {
                #[cfg(test)]
                capacity_fixture::stopping(&app.session.id);
                let run = harness.cancel(&app.session.id).await?;
                let idle = run.is_none();
                app.acknowledge_stop(run);
                if idle {
                    app.notice("No active run in this session.");
                }
            }
            "queue" if !value.is_empty() => {
                ensure_connection(app, connection_tasks)?;
                if app.is_running() {
                    harness.enqueue(&app.session.id, value.into()).await?;
                    app.record_local_control();
                } else {
                    harness.start(&app.session.id, value.into()).await?;
                    app.set_local_run_state(RunState::Running);
                }
            }
            "steer" if !value.is_empty() => {
                harness.steer(&app.session.id, value.into()).await?;
            }
            "status" => show_status(app, harness, args).await,
            "model" | "models" | "login" | "accounts" | "account" | "logout" => {
                interaction::command(
                    name,
                    value,
                    app,
                    harness,
                    args,
                    extensions,
                    connection_tasks,
                )
                .await?;
            }
            _ => {
                if matches!(name, "prompt" | "skill") {
                    ensure_connection(app, connection_tasks)?;
                }
                ensure!(
                    !connection_tasks.changing(),
                    "finish or cancel the account or model command first"
                );
                extension_command(name, value, app, harness, extensions, extension_tasks).await?;
            }
        }
    } else if app.is_running() {
        ensure_connection(app, connection_tasks)?;
        harness.enqueue(&app.session.id, text).await?;
        app.record_local_control();
    } else {
        ensure_connection(app, connection_tasks)?;
        harness.start(&app.session.id, text).await?;
        app.set_local_run_state(RunState::Running);
        app.status = "Starting…".into();
        app.scroll_from_bottom = 0;
    }
    Ok(false)
}

fn ensure_connection(app: &App, tasks: &ConnectionTasks) -> Result<()> {
    ensure!(
        !tasks.changing(),
        "finish or cancel the account or model command before sending a turn"
    );
    ensure!(
        app.connection_ready,
        "No model connection. Use /login, /account REGISTRATION or /model PROVIDER MODEL."
    );
    Ok(())
}

async fn show_status(app: &mut App, harness: &LocalBackend, args: &Args) {
    let status = harness.status().await;
    let claude = args.uses_builtin(ProviderKind::Claude);
    let model = if claude {
        format!(
            "claude {} (official client subscription)",
            args.provider_label()
        )
    } else {
        args.provider_label().into()
    };
    let account = if claude {
        "official Claude client's current account"
    } else {
        args.account.as_deref().unwrap_or("environment or none")
    };
    app.capacity = harness.capacity_snapshot();
    let capacity = app.capacity.as_ref().map(crate::capacity::describe);
    app.notice(format!(
        "Connection: {}\nModel: {}\nAccount: {}\nSession: {}\nGrants: {}\nManaged agents: {} running or waiting\nModels: {} active, {} queued\nTools: {} active, {} queued",
        if app.connection_ready { "ready" } else { "not connected" },
        model, account,
        app.session.id, args.grants(), status.active_agents, status.model_requests,
        status.waiting_model_requests, status.tools.running, status.tools.queued,
    ));
    if let Some(capacity) = capacity {
        app.notice(capacity);
    }
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

fn start_widgets(
    tasks: &mut ExtensionTasks,
    extensions: &Extensions,
    harness: &LocalBackend,
    session: &Session,
) {
    tasks.widget_active = true;
    let extensions = extensions.clone();
    let context = extensions.context(harness, session);
    tasks
        .jobs
        .spawn(async move { ExtensionReply::Widgets(extensions.widgets(context).await) });
}

fn apply_extension_reply(
    reply: Option<Result<ExtensionReply, tokio::task::JoinError>>,
    tasks: &mut ExtensionTasks,
    app: &mut App,
) {
    match reply {
        Some(Ok(ExtensionReply::Widgets(result))) => {
            tasks.widget_active = false;
            match result {
                Ok(lines) => app.widget_lines = lines,
                Err(error) => {
                    tasks.widgets_paused = true;
                    app.widget_lines = vec![format!(
                        "Widget paused: {}. /widgets resume to retry.",
                        render::safe_text(&error.to_string())
                    )];
                }
            }
        }
        Some(Ok(ExtensionReply::Command(result))) => {
            tasks.command_active = false;
            match result {
                Ok(text) => app.notice(text),
                Err(error) => app.error(format!("{error:#}")),
            }
        }
        Some(Err(error)) => {
            tasks.widget_active = false;
            tasks.command_active = false;
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
                app.record_local_control();
            } else {
                harness.start(&app.session.id, prompt).await?;
                app.set_local_run_state(RunState::Running);
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

fn open_sessions(app: &mut App) {
    app.overlay = Some(Overlay::Sessions {
        picker: crate::sessions::SessionPicker::default(),
    });
}

async fn find_session(harness: &LocalBackend, id: &str) -> Result<Session> {
    harness.resolve_session(id).await
}

fn workspace_title(cwd: &std::path::Path) -> String {
    cwd.file_name()
        .map(|name| name.to_string_lossy().chars().take(60).collect::<String>())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "Workspace".into())
}

enum ShutdownSignal {
    Interrupt,
    Terminate,
}

pub(crate) type TerminationSignal = Pin<Box<dyn Future<Output = ()> + Send>>;

pub(crate) fn termination_signal() -> TerminationSignal {
    Box::pin(async {
        let _ = auth_signal().await;
    })
}

async fn auth_signal() -> ShutdownSignal {
    #[cfg(windows)]
    {
        use tokio::signal::windows::{ctrl_break, ctrl_close, ctrl_logoff, ctrl_shutdown};
        if let (Ok(mut interrupt), Ok(mut close), Ok(mut logoff), Ok(mut shutdown)) =
            (ctrl_break(), ctrl_close(), ctrl_logoff(), ctrl_shutdown())
        {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => return ShutdownSignal::Interrupt,
                _ = interrupt.recv() => return ShutdownSignal::Terminate,
                _ = close.recv() => return ShutdownSignal::Terminate,
                _ = logoff.recv() => return ShutdownSignal::Terminate,
                _ = shutdown.recv() => return ShutdownSignal::Terminate,
            }
        }
    }
    #[cfg(unix)]
    {
        if let Ok(mut terminate) =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => return ShutdownSignal::Interrupt,
                _ = terminate.recv() => return ShutdownSignal::Terminate,
            }
        }
    }
    let _ = tokio::signal::ctrl_c().await;
    ShutdownSignal::Interrupt
}

static PANIC_HOOK: std::sync::Once = std::sync::Once::new();
static PANIC_BASELINE: std::sync::Mutex<Option<Modes>> = std::sync::Mutex::new(None);

pub(crate) struct TerminalGuard {
    pub(crate) terminal: Terminal<CrosstermBackend<Stdout>>,
    baseline: Modes,
    input: crate::terminal_input::Input,
}

impl TerminalGuard {
    fn suspend(&mut self) -> Result<()> {
        self.reset_input()?;
        restore_terminal();
        self.baseline.restore()
    }

    fn resume(&mut self) -> Result<()> {
        self.baseline.restore()?;
        enable_raw_mode()?;
        self.input.resume()?;
        execute!(
            io::stdout(),
            EnterAlternateScreen,
            EnableMouseCapture,
            EnableBracketedPaste
        )?;
        self.terminal.clear()?;
        Ok(())
    }

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
pub(crate) mod capacity_fixture;

#[cfg(test)]
mod terminal_fixture;

#[cfg(test)]
#[expect(
    clippy::unwrap_used,
    reason = "Invalid offline terminal fixtures must fail the test immediately."
)]
mod tests {
    use super::*;
    use crate::views;
    use clap::Parser;
    use ejm_harness::{HarnessConfig, ReplayProvider, SessionTailOptions};
    use std::sync::Arc;

    struct TestUi {
        state: tempfile::TempDir,
        harness: LocalBackend,
        app: App,
        args: Args,
        extensions: Extensions,
        extension_tasks: ExtensionTasks,
        connection_tasks: ConnectionTasks,
    }

    impl TestUi {
        async fn new() -> Self {
            let state = tempfile::tempdir().unwrap();
            let harness = Harness::with_optional_provider(
                HarnessConfig {
                    state_dir: state.path().into(),
                    ..HarnessConfig::default()
                },
                None,
                Vec::new(),
                Vec::new(),
            )
            .unwrap();
            harness
                .configure_provider(Some(Arc::new(ReplayProvider::echo())), "fixture")
                .await
                .unwrap();
            let harness = LocalBackend::new(harness);
            let session = harness
                .create_session(SessionOptions {
                    cwd: state.path().into(),
                    title: "fixture".into(),
                })
                .await
                .unwrap();
            let executor =
                Executor::new(harness.governor(), state.path().join("artifacts")).unwrap();
            let mut args =
                Args::try_parse_from(["ejm-cli", "--state-dir", state.path().to_str().unwrap()])
                    .unwrap();
            args.package_provider = Some("@fixture/terminal:provider".into());
            args.claude_path = Some(state.path().join("missing-claude-client"));
            Self {
                args,
                state,
                harness,
                app: App::new(session),
                extensions: Extensions::empty().unwrap(),
                extension_tasks: ExtensionTasks::default(),
                connection_tasks: ConnectionTasks::new(executor),
            }
        }

        async fn input(&mut self, input: InputEvent) -> Result<bool> {
            handle_input(
                input,
                &mut self.app,
                &self.harness,
                &mut self.args,
                &self.extensions,
                &mut self.extension_tasks,
                &mut self.connection_tasks,
            )
            .await
        }

        async fn key(&mut self, code: KeyCode) -> Result<bool> {
            self.input(InputEvent::Key(KeyEvent::new(code, KeyModifiers::NONE)))
                .await
        }
    }

    async fn saved_sessions(ui: &TestUi, count: usize) -> Vec<Session> {
        let mut sessions = vec![ui.app.session.clone()];
        for index in 0..count {
            sessions.push(
                ui.harness
                    .create_session(SessionOptions {
                        cwd: ui.state.path().into(),
                        title: format!("Saved session {index}"),
                    })
                    .await
                    .unwrap(),
            );
        }
        sessions
    }

    #[tokio::test]
    async fn session_resume_searches_beyond_recent_history_and_preserves_failed_drafts() {
        let mut ui = TestUi::new().await;
        let sessions = saved_sessions(&ui, 1002).await;
        let recent = ui.harness.list_sessions().await.unwrap();
        assert_eq!(recent.len(), 1000);
        let hidden = sessions
            .iter()
            .find(|session| !recent.iter().any(|item| item.id == session.id))
            .unwrap();

        ui.args.session = Some(hidden.id.clone());
        let initial = initial_app(&ui.harness, &ui.args, &ui.extensions)
            .await
            .unwrap();
        assert_eq!(initial.session.id, hidden.id);
        ui.app.composer.replace(&format!("/resume {}", hidden.id));
        ui.key(KeyCode::Enter).await.unwrap();
        assert_eq!(ui.app.session.id, hidden.id);

        let unique = (1..hidden.id.len())
            .map(|length| &hidden.id[..length])
            .find(|prefix| {
                sessions
                    .iter()
                    .filter(|session| session.id.starts_with(*prefix))
                    .count()
                    == 1
            })
            .unwrap();
        assert_eq!(
            find_session(&ui.harness, unique).await.unwrap().id,
            hidden.id
        );

        let ambiguous = sessions
            .iter()
            .map(|session| &session.id[..1])
            .max_by_key(|prefix| {
                sessions
                    .iter()
                    .filter(|session| session.id.starts_with(*prefix))
                    .count()
            })
            .unwrap();
        let page = ui
            .harness
            .sessions_page(ejm_harness::SessionQuery {
                query: ambiguous.into(),
                limit: 50,
                ..ejm_harness::SessionQuery::default()
            })
            .await
            .unwrap();
        assert_eq!(page.sessions.len(), 50);
        assert!(page.next.is_some());
        for id in [ambiguous, "not-a-saved-session"] {
            let draft = format!("/resume {id}");
            ui.app.composer.replace(&draft);
            assert!(ui.key(KeyCode::Enter).await.is_err());
            assert_eq!(ui.app.session.id, hidden.id);
            assert_eq!(ui.app.composer.text, draft);
        }
        ui.harness.shutdown().await.unwrap();
    }

    async fn finish_picker_page(ui: &mut TestUi, pages: &mut crate::sessions::SessionPages) {
        pages.follow(&ui.harness, &ui.app);
        let reply = pages.jobs.join_next().await;
        crate::sessions::SessionPages::joined(reply, &mut ui.app);
    }

    #[tokio::test]
    async fn session_picker_arrow_keys_cross_pages_without_losing_selection_or_draft() -> Result<()>
    {
        let mut ui = TestUi::new().await;
        saved_sessions(&ui, 52).await;
        open_sessions(&mut ui.app);
        ui.app.composer.replace("keep this draft");
        let mut pages = crate::sessions::SessionPages::default();
        finish_picker_page(&mut ui, &mut pages).await;
        for _ in 1..50 {
            ui.key(KeyCode::Down).await.unwrap();
        }
        let Some(Overlay::Sessions { picker }) = &ui.app.overlay else {
            anyhow::bail!("session picker missing");
        };
        let previous = picker.sessions[49].id.clone();
        assert_eq!(picker.selected, 49);
        ui.key(KeyCode::Down).await.unwrap();
        ui.key(KeyCode::Enter).await.unwrap();
        assert!(ui.app.overlay.is_some());
        finish_picker_page(&mut ui, &mut pages).await;
        let Some(Overlay::Sessions { picker }) = &ui.app.overlay else {
            anyhow::bail!("session picker missing");
        };
        assert_eq!(picker.sessions.len(), 3);
        assert_eq!(picker.selected, 0);
        ui.key(KeyCode::Up).await.unwrap();
        finish_picker_page(&mut ui, &mut pages).await;
        let Some(Overlay::Sessions { picker }) = &ui.app.overlay else {
            anyhow::bail!("session picker missing");
        };
        assert_eq!(picker.sessions[picker.selected].id, previous);
        ui.key(KeyCode::Esc).await.unwrap();
        assert!(ui.app.overlay.is_none());
        assert_eq!(ui.app.composer.text, "keep this draft");
        ui.harness.shutdown().await.unwrap();
        Ok(())
    }

    #[tokio::test]
    async fn historical_completion_during_recovery_keeps_ctrl_c_on_the_live_run() -> Result<()> {
        let mut fixture = crate::views::tests::recovering_fixture().await?;
        let mut ui = TestUi::new().await;
        ui.harness.shutdown().await?;
        ui.harness = fixture.harness.clone();
        ui.app = fixture.app;
        assert!(ui.app.last_sequence < fixture.through);
        ui.app.composer.replace("recovery draft");
        ui.key(KeyCode::Char('x')).await?;
        assert_eq!(ui.app.composer.text, "recovery draftx");

        let should_exit = tokio::time::timeout(
            Duration::from_secs(5),
            ui.input(InputEvent::Key(KeyEvent::new(
                KeyCode::Char('c'),
                KeyModifiers::CONTROL,
            ))),
        )
        .await??;
        assert!(
            !should_exit,
            "Ctrl+C must stop the live run during recovery"
        );
        assert_eq!(ui.app.run_state, RunState::Stopping);
        assert_eq!(ui.app.composer.text, "recovery draftx");
        assert!(ui.app.last_sequence < fixture.through);
        assert_eq!(
            fixture.run.wait().await?.status,
            ejm_harness::RunStatus::Cancelled
        );
        ui.harness.shutdown().await?;
        Ok(())
    }

    #[tokio::test]
    async fn ctrl_c_stops_a_local_start_accepted_after_the_loading_snapshot() -> Result<()> {
        for recovery in [false, true] {
            let mut ui = TestUi::new().await;
            let (_ready, gate) = tokio::sync::watch::channel(false);
            ui.harness
                .configure_provider(
                    Some(Arc::new(ejm_harness_bench::Workload::new(gate, 0, 0)?)),
                    "held local start",
                )
                .await?;
            let mut view = views::SelectedView::default();
            if recovery {
                view.recover(&ui.harness, &ui.app);
            } else {
                view.follow(&ui.harness, &ui.app);
            }
            let reply = tokio::time::timeout(Duration::from_secs(5), view.jobs.join_next()).await?;
            match reply.as_ref() {
                Some(Ok(
                    views::ViewReply::Tail { view, .. } | views::ViewReply::Catchup { view, .. },
                )) => {
                    assert!(view.run_id.is_none());
                }
                _ => anyhow::bail!("expected an idle selected-session snapshot"),
            }
            ui.app.composer.replace("accepted while loading");
            assert!(!ui.key(KeyCode::Enter).await?);
            assert_eq!(ui.app.run_state, RunState::Running);
            ui.app.composer.replace("next turn draft");
            view.joined(reply, &ui.harness, &mut ui.app);
            assert_eq!(ui.app.run_state, RunState::Running);
            let should_exit = ui
                .input(InputEvent::Key(KeyEvent::new(
                    KeyCode::Char('c'),
                    KeyModifiers::CONTROL,
                )))
                .await?;
            assert!(
                !should_exit,
                "a stale snapshot must not turn Stop into exit"
            );
            assert_eq!(ui.app.run_state, RunState::Stopping);
            assert_eq!(ui.app.composer.text, "next turn draft");
            ui.harness.shutdown().await?;
        }
        Ok(())
    }

    async fn finish_selected_snapshot(
        ui: &mut TestUi,
        view: &mut views::SelectedView,
        recovery: bool,
    ) -> Result<()> {
        let reply = tokio::time::timeout(Duration::from_secs(5), view.jobs.join_next()).await?;
        view.joined(reply, &ui.harness, &mut ui.app);
        if recovery {
            let reply = tokio::time::timeout(Duration::from_secs(5), view.jobs.join_next()).await?;
            view.joined(reply, &ui.harness, &mut ui.app);
        }
        views::tests::finish_batch_page(view, &ui.harness, &mut ui.app);
        assert!(view.events.is_some(), "selected snapshot did not finish");
        Ok(())
    }

    #[tokio::test]
    async fn stop_uses_live_authority_and_keeps_exact_outcome_through_loading() -> Result<()> {
        for recovery in [false, true] {
            for ctrl_c in [false, true] {
                let mut ui = TestUi::new().await;
                let (ready, gate) = tokio::sync::watch::channel(false);
                ui.harness
                    .configure_provider(
                        Some(Arc::new(ejm_harness_bench::Workload::new(gate, 0, 0)?)),
                        "held while loading",
                    )
                    .await?;
                let mut run = ui
                    .harness
                    .start(&ui.app.session.id, "stop before snapshot loads".into())
                    .await?;
                let mut view = views::SelectedView::default();
                if recovery {
                    view.recover(&ui.harness, &ui.app);
                } else {
                    view.follow(&ui.harness, &ui.app);
                }
                let stale =
                    tokio::time::timeout(Duration::from_secs(5), view.jobs.join_next()).await?;
                match stale.as_ref() {
                    Some(Ok(
                        views::ViewReply::Tail { view, .. }
                        | views::ViewReply::Catchup { view, .. },
                    )) => assert_eq!(view.run_id.as_deref(), Some(run.id.as_str())),
                    _ => anyhow::bail!("expected the live run's loading snapshot"),
                }
                assert_eq!(ui.app.run_state, RunState::Idle);
                let should_exit = if ctrl_c {
                    ui.input(InputEvent::Key(KeyEvent::new(
                        KeyCode::Char('c'),
                        KeyModifiers::CONTROL,
                    )))
                    .await?
                } else {
                    ui.app.composer.replace("/stop");
                    ui.key(KeyCode::Enter).await?
                };
                assert!(!should_exit, "Stop must use live authority while loading");
                assert_eq!(ui.app.run_state, RunState::Stopping);
                assert_eq!(ui.app.stop_status(), Some("Stopping…"));
                assert_eq!(run.wait().await?.status, ejm_harness::RunStatus::Cancelled);
                ui.app.composer.replace("draft after Stop");
                view.joined(stale, &ui.harness, &mut ui.app);
                finish_selected_snapshot(&mut ui, &mut view, recovery).await?;
                assert_eq!(ui.app.run_state, RunState::Idle);
                assert_eq!(ui.app.stop_status(), Some("cancelled"));
                assert_eq!(ui.app.status, "cancelled");
                assert_eq!(ui.app.composer.text, "draft after Stop");

                let mut replacement = ui
                    .harness
                    .start(&ui.app.session.id, "completed replacement".into())
                    .await?;
                if !recovery {
                    let through = ui
                        .harness
                        .session_view(&ui.app.session.id, SessionTailOptions::default())
                        .await?
                        .selected_cursor;
                    views::tests::consume_through(&mut view, &ui.harness, &mut ui.app, through)
                        .await?;
                    assert_eq!(ui.app.run_state, RunState::Running);
                    assert!(ui.app.stop_status().is_none());
                }
                ready.send(true)?;
                assert_eq!(
                    replacement.wait().await?.status,
                    ejm_harness::RunStatus::Completed
                );
                if recovery {
                    view.recover(&ui.harness, &ui.app);
                    finish_selected_snapshot(&mut ui, &mut view, true).await?;
                } else {
                    let through = ui
                        .harness
                        .session_view(&ui.app.session.id, SessionTailOptions::default())
                        .await?
                        .selected_cursor;
                    views::tests::consume_through(&mut view, &ui.harness, &mut ui.app, through)
                        .await?;
                }
                assert_eq!(ui.app.run_state, RunState::Idle);
                assert!(ui.app.stop_status().is_none());
                assert!(!ui.app.status.contains("cancelled"));

                ui.app.composer.replace("/stop");
                ui.key(KeyCode::Enter).await?;
                assert_eq!(ui.app.run_state, RunState::Idle);
                assert!(ui.app.stop_status().is_none());
                assert_eq!(ui.app.status, "No active run in this session.");
                ui.harness.shutdown().await?;
            }
        }
        Ok(())
    }

    #[tokio::test]
    async fn slash_picker_completes_without_running_and_escape_preserves_draft() {
        let mut ui = TestUi::new().await;
        ui.app.composer.insert("/mod");
        ui.key(KeyCode::Down).await.unwrap();
        ui.key(KeyCode::Tab).await.unwrap();
        assert_eq!(ui.app.composer.text, "/models");
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
    async fn repeated_enter_cannot_choose_login_but_navigation_and_edits_repeat() {
        use crossterm::event::KeyEventKind;

        let mut ui = TestUi::new().await;
        ui.app.composer.replace("/login");
        ui.key(KeyCode::Enter).await.unwrap();
        for kind in [KeyEventKind::Repeat, KeyEventKind::Release] {
            ui.input(InputEvent::Key(KeyEvent::new_with_kind(
                KeyCode::Enter,
                KeyModifiers::NONE,
                kind,
            )))
            .await
            .unwrap();
            assert!(matches!(
                ui.app.overlay,
                Some(Overlay::Login { selected: 0 })
            ));
            assert!(!ui.connection_tasks.changing());
            assert!(ui.connection_tasks.external_login.is_none());
        }
        ui.input(InputEvent::Key(KeyEvent::new_with_kind(
            KeyCode::Down,
            KeyModifiers::NONE,
            KeyEventKind::Repeat,
        )))
        .await
        .unwrap();
        assert!(matches!(
            ui.app.overlay,
            Some(Overlay::Login { selected: 1 })
        ));
        ui.key(KeyCode::Esc).await.unwrap();
        ui.input(InputEvent::Key(KeyEvent::new_with_kind(
            KeyCode::Backspace,
            KeyModifiers::NONE,
            KeyEventKind::Repeat,
        )))
        .await
        .unwrap();
        assert_eq!(ui.app.composer.text, "/logi");
    }

    #[tokio::test]
    async fn invalid_commands_and_unconfigured_prompts_keep_the_draft() {
        let mut ui = TestUi::new().await;
        ui.app.composer.insert("/resume");
        let error = ui.key(KeyCode::Enter).await.unwrap_err();
        assert!(error.to_string().contains("Usage: /resume ID"));
        assert_eq!(ui.app.composer.text, "/resume");
        ui.harness
            .configure_provider(None, "fixture")
            .await
            .unwrap();
        ui.app.connection_ready = false;
        ui.app.composer.replace("keep this prompt");
        assert!(ui.key(KeyCode::Enter).await.is_err());
        assert_eq!(ui.app.composer.text, "keep this prompt");
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
    async fn pending_commands_leave_input_responsive_and_ctrl_c_cancels() {
        let mut ui = TestUi::new().await;
        ui.connection_tasks
            .spawn("/login".into(), true, |cancel| async move {
                cancel.cancelled().await;
                anyhow::bail!("Command cancelled.")
            })
            .unwrap();
        ui.key(KeyCode::Char('x')).await.unwrap();
        assert_eq!(ui.app.composer.text, "x");
        assert!(ui.key(KeyCode::Enter).await.is_err());
        assert_eq!(ui.app.composer.text, "x");
        let exit = ui
            .input(InputEvent::Key(KeyEvent::new(
                KeyCode::Char('c'),
                KeyModifiers::CONTROL,
            )))
            .await
            .unwrap();
        assert!(!exit);
        let result = ui.connection_tasks.jobs.join_next().await.unwrap().unwrap();
        assert!(
            result
                .err()
                .is_some_and(|error| error.to_string().contains("cancelled"))
        );
    }

    #[tokio::test]
    async fn login_picker_navigation_and_dismissal_preserve_the_draft_without_tasks() {
        let mut ui = TestUi::new().await;
        ui.app.composer.replace("/login");
        ui.key(KeyCode::Enter).await.unwrap();
        for (key, expected) in [
            (KeyCode::Up, 0),
            (KeyCode::Down, 1),
            (KeyCode::PageDown, 4),
            (KeyCode::Down, 4),
            (KeyCode::PageUp, 0),
        ] {
            ui.key(key).await.unwrap();
            assert!(matches!(
                ui.app.overlay,
                Some(Overlay::Login { selected }) if selected == expected
            ));
        }
        ui.input(InputEvent::Paste("unused input".into()))
            .await
            .unwrap();
        ui.key(KeyCode::Esc).await.unwrap();
        assert!(ui.app.overlay.is_none());
        assert_eq!(ui.app.composer.text, "/login");
        assert!(ui.connection_tasks.jobs.is_empty());
        assert!(ui.connection_tasks.pending.is_none());
        assert!(ui.connection_tasks.external_login.is_none());
        assert!(ui.app.connection_ready);
        assert_eq!(
            ui.args.package_provider.as_deref(),
            Some("@fixture/terminal:provider")
        );
        assert!(ui.app.blocks.is_empty());
        assert!(
            ui.harness
                .events(&ui.app.session.id, 0, 100)
                .await
                .unwrap()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn login_picker_dispatches_chatgpt_only_after_enter() {
        let mut ui = TestUi::new().await;
        ui.app.composer.replace("/login");
        ui.key(KeyCode::Enter).await.unwrap();
        assert!(ui.connection_tasks.jobs.is_empty());
        ui.key(KeyCode::Enter).await.unwrap();
        // Abort before yielding so this dispatch check cannot start real sign-in.
        ui.connection_tasks.jobs.abort_all();
        assert!(ui.app.overlay.is_none());
        assert_eq!(ui.app.composer.text, "");
        assert_eq!(
            ui.connection_tasks.pending.as_ref().unwrap().draft,
            "/login"
        );
        assert!(ui.connection_tasks.external_login.is_none());
        assert!(ui.app.status.contains("ChatGPT"));
        assert_eq!(
            ui.args.package_provider.as_deref(),
            Some("@fixture/terminal:provider")
        );
        assert!(ui.app.connection_ready);
        ui.connection_tasks.shutdown().await;
    }

    #[tokio::test]
    async fn background_model_catalog_keeps_the_login_choice_and_dispatch() {
        let mut ui = TestUi::new().await;
        ui.connection_tasks
            .spawn("/models".into(), false, |_| async {
                Ok(ConnectionReply::Catalog(vec![
                    ejm_harness::AvailableModel {
                        slug: "fixture-model".into(),
                        display_name: "Fixture model".into(),
                    },
                ]))
            })
            .unwrap();
        ui.app.composer.replace("/login");
        ui.key(KeyCode::Enter).await.unwrap();
        ui.key(KeyCode::Down).await.unwrap();
        let reply = ui.connection_tasks.jobs.join_next().await.unwrap().unwrap();
        let (pending, reply) = ui.connection_tasks.finish(reply);
        resolve_connection(
            reply,
            pending.map(|command| command.draft),
            &mut ui.app,
            &ui.harness,
            &mut ui.args,
            true,
        )
        .await;
        assert!(matches!(
            ui.app.overlay,
            Some(Overlay::Login { selected: 1 })
        ));
        assert_eq!(ui.app.composer.text, "/login");
        ui.key(KeyCode::Enter).await.unwrap();
        let request = ui.connection_tasks.external_login.as_ref().unwrap();
        assert_eq!(request.candidate.provider, ProviderKind::Claude);
        assert_eq!(request.draft, "/login");
        assert!(ui.connection_tasks.jobs.is_empty());
        assert_eq!(
            ui.args.package_provider.as_deref(),
            Some("@fixture/terminal:provider")
        );
        assert!(ui.app.connection_ready);
    }

    #[tokio::test]
    async fn provider_guidance_and_accounts_work_offline_inside_the_tui() {
        let mut ui = TestUi::new().await;
        for (selected, guidance) in [
            (2, "/model openai MODEL"),
            (3, "/model responses MODEL"),
            (4, "/model anthropic MODEL"),
        ] {
            ui.app.composer.replace("/login");
            ui.key(KeyCode::Enter).await.unwrap();
            for _ in 0..selected {
                ui.key(KeyCode::Down).await.unwrap();
            }
            ui.key(KeyCode::Enter).await.unwrap();
            assert!(ui.app.overlay.is_none());
            assert!(
                ui.app
                    .blocks
                    .back()
                    .is_some_and(|block| block.text.contains(guidance))
            );
            assert!(ui.connection_tasks.jobs.is_empty());
            assert!(ui.connection_tasks.pending.is_none());
            assert!(ui.connection_tasks.external_login.is_none());
            assert!(ui.app.connection_ready);
            assert_eq!(
                ui.args.package_provider.as_deref(),
                Some("@fixture/terminal:provider")
            );
        }
        ui.app.composer.insert("/accounts");
        ui.key(KeyCode::Enter).await.unwrap();
        let reply = ui.connection_tasks.jobs.join_next().await.unwrap().unwrap();
        let (_, reply) = ui.connection_tasks.finish(reply);
        interaction::apply_reply(reply, &mut ui.app, &ui.harness, &mut ui.args, true)
            .await
            .unwrap();
        assert!(
            ui.app
                .blocks
                .iter()
                .any(|block| block.text.contains("No saved accounts"))
        );
    }

    #[tokio::test]
    async fn failed_claude_handoff_keeps_the_connection_and_command_draft() {
        let mut ui = TestUi::new().await;
        ui.app.composer.replace("/login");
        ui.key(KeyCode::Enter).await.unwrap();
        assert!(ui.connection_tasks.external_login.is_none());
        ui.key(KeyCode::Down).await.unwrap();
        ui.key(KeyCode::Enter).await.unwrap();
        let request = ui.connection_tasks.external_login.take().unwrap();
        assert_eq!(request.candidate.provider, ProviderKind::Claude);
        assert_eq!(
            ui.args.package_provider.as_deref(),
            Some("@fixture/terminal:provider")
        );
        assert!(ui.app.connection_ready);
        resolve_connection(
            Err(anyhow::anyhow!("Claude sign-in cancelled.")),
            Some(request.draft),
            &mut ui.app,
            &ui.harness,
            &mut ui.args,
            true,
        )
        .await;
        assert_eq!(ui.app.composer.text, "/login");
        assert_eq!(
            ui.args.package_provider.as_deref(),
            Some("@fixture/terminal:provider")
        );
        assert!(ui.app.connection_ready);
        assert!(
            ui.harness
                .events(&ui.app.session.id, 0, 100)
                .await
                .unwrap()
                .is_empty()
        );
        let mut run = ui
            .harness
            .start(&ui.app.session.id, "still connected".into())
            .await
            .unwrap();
        assert!(run.wait().await.unwrap().text.contains("still connected"));
    }

    #[tokio::test]
    async fn claude_handoff_waits_for_callbacks_and_logout_clears_readiness() {
        let mut ui = TestUi::new().await;
        for widget in [true, false] {
            ui.extension_tasks.widget_active = widget;
            ui.extension_tasks.command_active = !widget;
            ui.app.composer.replace("/login");
            ui.key(KeyCode::Enter).await.unwrap();
            ui.key(KeyCode::Down).await.unwrap();
            assert!(ui.key(KeyCode::Enter).await.is_err());
            assert_eq!(ui.app.composer.text, "/login");
            assert!(ui.connection_tasks.external_login.is_none());
            assert!(ui.connection_tasks.jobs.is_empty());
        }
        ui.args.provider = ProviderKind::Claude;
        interaction::apply_reply(
            Ok(ConnectionReply::ClaudeLoggedOut),
            &mut ui.app,
            &ui.harness,
            &mut ui.args,
            true,
        )
        .await
        .unwrap();
        assert!(
            ui.app.connection_ready,
            "sign-out must preserve the effective package connection"
        );
        ui.args.package_provider = None;
        ui.args.base_url = None;
        ui.args.model = Some("sonnet".into());
        interaction::apply_reply(
            Ok(ConnectionReply::ClaudeLoggedOut),
            &mut ui.app,
            &ui.harness,
            &mut ui.args,
            true,
        )
        .await
        .unwrap();
        assert!(!ui.app.connection_ready);
        assert!(
            ui.harness
                .start(&ui.app.session.id, "must not run".into())
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn login_picker_keeps_pending_command_guards_on_subscription_selection() {
        let mut ui = TestUi::new().await;
        ui.connection_tasks
            .spawn("/models".into(), false, |cancel| async move {
                cancel.cancelled().await;
                anyhow::bail!("Command cancelled.")
            })
            .unwrap();
        for selected in 0..2 {
            ui.app.composer.replace("/login");
            ui.key(KeyCode::Enter).await.unwrap();
            for _ in 0..selected {
                ui.key(KeyCode::Down).await.unwrap();
            }
            let error = ui.key(KeyCode::Enter).await.unwrap_err();
            assert!(error.to_string().contains("command is running"));
            assert_eq!(ui.app.composer.text, "/login");
            assert!(ui.connection_tasks.external_login.is_none());
            assert_eq!(ui.connection_tasks.jobs.len(), 1);
            assert_eq!(
                ui.connection_tasks.pending.as_ref().unwrap().draft,
                "/models"
            );
        }
        ui.connection_tasks.shutdown().await;
    }

    #[tokio::test]
    async fn model_activation_changes_the_backend_and_rejects_a_busy_change() {
        let mut ui = TestUi::new().await;
        let mut candidate = ui.args.clone();
        candidate.package_provider = None;
        candidate.model = Some("fixture-model".into());
        let response =
            serde_json::from_value(serde_json::json!({"content":"selected backend"})).unwrap();
        let reply = interaction::ConnectionReply::Connected {
            args: Box::new(candidate),
            provider: Arc::new(ReplayProvider::new(vec![response])),
            message: "Changed model".into(),
        };
        interaction::apply_reply(Ok(reply), &mut ui.app, &ui.harness, &mut ui.args, true)
            .await
            .unwrap();
        assert_eq!(ui.args.model.as_deref(), Some("fixture-model"));
        let mut run = ui
            .harness
            .start(&ui.app.session.id, "test".into())
            .await
            .unwrap();
        assert!(
            interaction::command(
                "model",
                "other-model",
                &mut ui.app,
                &ui.harness,
                &ui.args,
                &ui.extensions,
                &mut ui.connection_tasks
            )
            .await
            .is_err()
        );
        for selected in 0..2 {
            ui.app.composer.replace("/login");
            ui.key(KeyCode::Enter).await.unwrap();
            for _ in 0..selected {
                ui.key(KeyCode::Down).await.unwrap();
            }
            let error = ui.key(KeyCode::Enter).await.unwrap_err();
            assert!(error.to_string().contains("managed runs"));
            assert_eq!(ui.app.composer.text, "/login");
            assert!(ui.connection_tasks.pending.is_none());
            assert!(ui.connection_tasks.external_login.is_none());
        }
        assert_eq!(ui.args.model.as_deref(), Some("fixture-model"));
        assert_eq!(run.wait().await.unwrap().text, "selected backend");
    }
}
