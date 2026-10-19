//! Native remote presentation uses the same bounded App and renderer as local sessions.
mod network;
use super::{
    account::RelayAccount,
    backend::{self, ReplayPage, SessionBackend},
    identity::EndpointIdentity,
    invitation::{Invitation, ShareRole},
    protocol::{
        ClientRequest, MAX_MUTATION_SEQUENCE, Mutation, MutationAction, MutationOutcome,
        ServerMessage, WireBody, WireMessage,
    },
    transport::{self, EncryptedSocket},
};
use crate::{
    app::{App, Overlay, RemoteBadge, RunState},
    args::Args,
    commands, render, ui,
};
use anyhow::{Context, Result, bail, ensure};
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ejm_harness::{Event as HarnessEvent, Session};
use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};
use tokio::{
    sync::{Notify, RwLock, mpsc, oneshot},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;

enum Command {
    Replay {
        after: u64,
        reply: oneshot::Sender<Result<ReplayPage>>,
    },
    Mutation(Mutation),
    Refresh,
    Ready,
}

struct CurrentSession {
    session: Session,
    active_run: Option<String>,
    mutation_high_water: u64,
}

pub struct RemoteBackend {
    commands: mpsc::Sender<Command>,
    current: Arc<RwLock<CurrentSession>>,
    cursor: Arc<AtomicU64>,
    cursor_changed: Arc<Notify>,
}

impl SessionBackend for RemoteBackend {
    async fn session(&self, id: &str) -> Result<Session> {
        let current = self.current.read().await;
        ensure!(
            current.session.id == id,
            "remote session cannot be switched"
        );
        Ok(current.session.clone())
    }
    async fn replay(&self, id: &str, after: u64) -> Result<ReplayPage> {
        ensure!(
            self.current.read().await.session.id == id,
            "remote session cannot be switched"
        );
        let (reply, receive) = oneshot::channel();
        self.commands
            .send(Command::Replay { after, reply })
            .await
            .context("remote connection ended")?;
        let page = tokio::time::timeout(Duration::from_secs(70), receive)
            .await
            .context("remote replay timed out")?
            .context("remote replay ended")??;
        self.ack_cursor(page.through_sequence);
        Ok(page)
    }
}

impl RemoteBackend {
    fn ack_cursor(&self, sequence: u64) {
        self.cursor.store(sequence, Ordering::Relaxed);
        self.cursor_changed.notify_one();
    }
}

enum Update {
    Server(ServerMessage),
    Notice(&'static str),
    Ended,
}
struct Pending {
    sequence: u64,
    draft: String,
}
struct Controls {
    role: ShareRole,
    active: Option<String>,
    sequence: u64,
    pending: Option<Pending>,
}
struct Controller {
    backend: RemoteBackend,
    output: mpsc::Receiver<Update>,
    app: App,
    controls: Controls,
    _network: NetworkGuard,
}

pub async fn run(
    args: &Args,
    account: RelayAccount,
    identity: EndpointIdentity,
    invitation: Invitation,
    cancel: CancellationToken,
) -> Result<bool> {
    let controller = Controller::connect(account, identity, invitation, cancel.clone()).await?;
    controller.present(args, cancel).await
}

impl Controller {
    async fn connect(
        account: RelayAccount,
        identity: EndpointIdentity,
        invitation: Invitation,
        cancel: CancellationToken,
    ) -> Result<Self> {
        let fingerprint = identity.fingerprint();
        println!("Controller fingerprint: {fingerprint}");
        println!("Waiting for the host to confirm this fingerprint.");
        let mut socket = tokio::select! {
            biased;
            () = cancel.cancelled() => bail!("remote connection cancelled"),
            result = transport::connect_controller(&account, &identity, &invitation) => result?,
        };
        let (session, active_run, role, mutation_high_water) =
            initial_snapshot(&mut socket, &cancel).await?;
        ensure!(role == invitation.role(), "host share grant changed");
        let (commands, command_input) = mpsc::channel(1);
        let (updates, output) = mpsc::channel(16);
        let cursor = Arc::new(AtomicU64::new(0));
        let current = Arc::new(RwLock::new(CurrentSession {
            session: session.clone(),
            active_run,
            mutation_high_water,
        }));
        let cursor_changed = Arc::new(Notify::new());
        let backend = RemoteBackend {
            commands,
            current: current.clone(),
            cursor: cursor.clone(),
            cursor_changed: cursor_changed.clone(),
        };
        let task_cancel = cancel.child_token();
        let task_token = task_cancel.clone();
        let task = tokio::spawn(network::run(
            socket,
            network::Context {
                account,
                identity,
                invitation,
                cursor,
                cursor_changed,
                current,
                cancel: task_token,
            },
            command_input,
            updates,
        ));
        let guard = NetworkGuard {
            task,
            cancel: task_cancel,
        };
        let mut app = backend::load_app(&backend, session).await?;
        backend.ack_cursor(app.last_sequence);
        let (active, sequence) = {
            let current = backend.current.read().await;
            (current.active_run.clone(), current.mutation_high_water)
        };
        backend
            .commands
            .send(Command::Ready)
            .await
            .context("remote connection ended")?;
        app.commands = commands::catalog(&[])
            .into_iter()
            .filter(|command| {
                matches!(command.name.as_str(), "help" | "status" | "quit" | "exit")
                    || (role == ShareRole::Control
                        && matches!(command.name.as_str(), "queue" | "steer" | "stop"))
            })
            .collect();
        app.remote = Some(RemoteBadge {
            controller: true,
            label: format!("Remote {role:?}"),
        });
        app.connection_ready = true;
        app.notice(format!("Encrypted native controller. Access: {role:?}. The host retains execution grants and approvals."));
        Ok(Self {
            backend,
            output,
            app,
            controls: Controls {
                role,
                active,
                sequence,
                pending: None,
            },
            _network: guard,
        })
    }

    async fn present(mut self, args: &Args, cancel: CancellationToken) -> Result<bool> {
        let mut terminal = ui::TerminalGuard::enter()?;
        let mut tick = tokio::time::interval(Duration::from_millis(32));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let shutdown = ui::termination_signal();
        tokio::pin!(shutdown);
        let mut output_open = true;
        loop {
            tokio::select! {
                biased;
                () = cancel.cancelled() => break,
                () = &mut shutdown => break,
                update = self.output.recv(), if output_open => {
                    match update {
                        Some(Update::Server(message)) => apply_update(message, &mut self.app, &mut self.controls),
                        Some(Update::Notice(message)) => { self.app.notice(message); self.app.connection_ready = false; }
                        Some(Update::Ended) | None => {
                            self.app.notice("Remote connection ended. Your draft remains here. Use /quit to leave. No uncertain action was resubmitted.");
                            self.app.connection_ready = false;
                            output_open = false;
                        }
                    }
                    self.backend.ack_cursor(self.app.last_sequence);
                }
                _ = tick.tick() => {
                    for _ in 0..64 {
                        if !terminal.poll_input()? { break; }
                        match self.controls.input(terminal.read_input()?, &mut self.app, &self.backend) {
                            Ok(true) => return Ok(true),
                            Ok(false) => {}
                            Err(error) => self.app.error(format!("{error:#}")),
                        }
                    }
                    if self.app.dirty {
                        let snapshot = ejm_harness::HarnessSnapshot { active_agents: usize::from(self.app.is_running()),
                            model_requests: 0, waiting_model_requests: 0, tools: ejm_harness_exec::GovernorSnapshot::default() };
                        terminal.terminal.draw(|frame| render::draw(frame, &mut self.app, args, &snapshot))?;
                        self.app.dirty = false;
                    }
                }
            }
        }
        Ok(true)
    }
}

async fn initial_snapshot(
    socket: &mut EncryptedSocket,
    cancel: &CancellationToken,
) -> Result<(Session, Option<String>, ShareRole, u64)> {
    socket
        .send(&WireMessage::client(ClientRequest::Snapshot {
            after_sequence: 0,
            limit: 512,
        }))
        .await?;
    let response = tokio::select! {
        biased;
        () = cancel.cancelled() => bail!("remote connection cancelled"),
        response = tokio::time::timeout(Duration::from_secs(70), socket.receive()) => response.context("host confirmation timed out")??,
    };
    let WireBody::Server(ServerMessage::Snapshot {
        session,
        role,
        active_run,
        mutation_high_water,
        ..
    }) = response.body
    else {
        bail!("host did not provide a shared session");
    };
    Ok((session, active_run, role, mutation_high_water))
}

impl Controls {
    fn input(&mut self, input: Event, app: &mut App, backend: &RemoteBackend) -> Result<bool> {
        app.dirty = true;
        match input {
            Event::Paste(text) => {
                app.composer.insert(&text);
                app.command_picker = commands::CommandPicker::default();
            }
            Event::Key(key) if crate::terminal::accepts_key(key) => {
                if app.overlay.is_some() {
                    if matches!(key.code, KeyCode::Esc | KeyCode::Enter) {
                        app.overlay = None;
                    }
                } else if !ui::handle_picker_key(key, app) {
                    if key.modifiers.contains(KeyModifiers::CONTROL) {
                        return self.control_key(key.code, app, backend);
                    }
                    if key.code == KeyCode::Enter
                        && !key
                            .modifiers
                            .intersects(KeyModifiers::ALT | KeyModifiers::SHIFT)
                    {
                        return self.enter(app, backend);
                    }
                    ui::edit_composer(key, app);
                }
            }
            Event::Mouse(mouse) => match mouse.kind {
                event::MouseEventKind::ScrollUp => {
                    app.scroll_from_bottom = app.scroll_from_bottom.saturating_add(3);
                }
                event::MouseEventKind::ScrollDown => {
                    app.scroll_from_bottom = app.scroll_from_bottom.saturating_sub(3);
                }
                _ => {}
            },
            _ => {}
        }
        Ok(false)
    }

    fn control_key(
        &mut self,
        code: KeyCode,
        app: &mut App,
        backend: &RemoteBackend,
    ) -> Result<bool> {
        match code {
            KeyCode::Char('c')
                if self.role == ShareRole::Control && app.is_running() && app.connection_ready =>
            {
                let run_id = self
                    .active
                    .clone()
                    .context("active run identity unavailable; refresh first")?;
                self.submit(
                    app,
                    backend,
                    MutationAction::Cancel { run_id },
                    String::new(),
                )?;
            }
            KeyCode::Char('c') => return Ok(true),
            KeyCode::Char('d') if app.composer.text.is_empty() => return Ok(true),
            KeyCode::Char('k') if self.role == ShareRole::Control => {
                let run_id = self
                    .active
                    .clone()
                    .context("steering needs an active run")?;
                let text = app.composer.text.clone();
                self.submit(
                    app,
                    backend,
                    MutationAction::Steer {
                        run_id,
                        text: text.clone(),
                    },
                    text,
                )?;
            }
            KeyCode::Char('j') => app.composer.insert("\n"),
            KeyCode::Char('a') => app.composer.home(),
            KeyCode::Char('e') => app.composer.end(),
            KeyCode::Char('u') => app.composer.replace(""),
            _ => {}
        }
        Ok(false)
    }

    fn enter(&mut self, app: &mut App, backend: &RemoteBackend) -> Result<bool> {
        let text = app.composer.text.clone();
        if text.trim().is_empty() {
            return Ok(false);
        }
        let action = if let Some(command) = text.strip_prefix('/') {
            let command = commands::parse(command)?;
            match command.name.as_str() {
                "quit" | "exit" => return Ok(true),
                "help" => {
                    app.overlay = Some(Overlay::Help { selected: 0 });
                    app.composer.take();
                    return Ok(false);
                }
                "status" => {
                    backend
                        .commands
                        .try_send(Command::Refresh)
                        .context("remote connection unavailable; draft retained")?;
                    app.notice("Refreshing retained output and checking the pending action result. Actions are never resubmitted.");
                    app.composer.take();
                    return Ok(false);
                }
                "stop" => MutationAction::Cancel {
                    run_id: self.active.clone().context("no active run")?,
                },
                "steer" => MutationAction::Steer {
                    run_id: self.active.clone().context("no active run")?,
                    text: command.value.into(),
                },
                "queue" => MutationAction::Queue {
                    run_id: self.active.clone().context("no active run")?,
                    text: command.value.into(),
                },
                _ => bail!("this command is unavailable to a remote controller"),
            }
        } else if let Some(run_id) = &self.active {
            MutationAction::Queue {
                run_id: run_id.clone(),
                text: text.clone(),
            }
        } else {
            MutationAction::Submit { text: text.clone() }
        };
        self.submit(app, backend, action, text)?;
        Ok(false)
    }

    fn submit(
        &mut self,
        app: &mut App,
        backend: &RemoteBackend,
        action: MutationAction,
        draft: String,
    ) -> Result<()> {
        ensure!(
            self.role == ShareRole::Control,
            "the host granted view access only"
        );
        ensure!(
            app.connection_ready,
            "remote connection unavailable; draft retained"
        );
        ensure!(
            self.pending.is_none(),
            "wait for the pending action result; it will not be resubmitted"
        );
        ensure!(
            self.sequence < MAX_MUTATION_SEQUENCE,
            "action sequence exhausted; ask the host for a new share"
        );
        self.sequence += 1;
        let mutation = Mutation {
            sequence: self.sequence,
            expected: backend::expected(&app.session, self.active.clone(), app.last_sequence),
            action,
        };
        mutation.validate()?;
        backend
            .commands
            .try_send(Command::Mutation(mutation))
            .context("remote transport is busy; draft retained")?;
        self.pending = Some(Pending {
            sequence: self.sequence,
            draft,
        });
        app.composer.take();
        app.status = "Waiting for action result".into();
        Ok(())
    }

    fn outcome(&mut self, sequence: u64, outcome: MutationOutcome, app: &mut App) {
        if matches!(outcome, MutationOutcome::Pending) {
            app.notice("The host is still resolving the action. Use /status to check. It will not be resubmitted.");
            return;
        }
        let current = if self
            .pending
            .as_ref()
            .is_some_and(|request| request.sequence == sequence)
        {
            self.pending.take()
        } else {
            None
        };
        match outcome {
            MutationOutcome::Accepted { .. } => app.status = "Action accepted".into(),
            MutationOutcome::Queued => app.notice("Remote prompt queued on the host. Stopping sharing discards pending remote prompts."),
            MutationOutcome::Rejected { message, .. } => {
                restore_draft(current, app);
                app.error(message);
            }
            MutationOutcome::Unknown => {
                restore_draft(current, app);
                app.notice("The action result is unknown. It was not resubmitted. Inspect session output before choosing another action.");
            }
            MutationOutcome::Pending => {}
        }
    }
}

fn restore_draft(pending: Option<Pending>, app: &mut App) {
    if let Some(pending) = pending
        && app.composer.text.is_empty()
    {
        app.composer.replace(&pending.draft);
    }
}

fn apply_update(message: ServerMessage, app: &mut App, controls: &mut Controls) {
    app.connection_ready = true;
    match message {
        ServerMessage::Snapshot {
            session,
            events,
            through_sequence,
            gap,
            active_run,
            role,
            mutation_high_water,
        } => {
            controls.sequence = controls.sequence.max(mutation_high_water);
            app.session = session;
            apply_events(
                events,
                through_sequence,
                gap.is_some(),
                app,
                &mut controls.active,
            );
            // Historical replay cannot replace the current run reported by the host.
            controls.active = active_run;
            app.run_state = if controls.active.is_some() {
                RunState::Running
            } else {
                RunState::Idle
            };
            controls.role = role;
            app.remote = Some(RemoteBadge {
                controller: true,
                label: format!("Remote {role:?}"),
            });
        }
        ServerMessage::Events {
            events,
            through_sequence,
            gap,
        } => apply_events(
            events,
            through_sequence,
            gap.is_some(),
            app,
            &mut controls.active,
        ),
        ServerMessage::Outcome { sequence, outcome } => controls.outcome(sequence, outcome, app),
        ServerMessage::Error { message, .. } => app.error(message),
        ServerMessage::Stopped { .. } => {
            app.notice("The host stopped sharing. Your draft remains here. Use /quit to leave.");
            app.connection_ready = false;
        }
    }
    app.dirty = true;
}

fn apply_events(
    events: Vec<HarnessEvent>,
    through: u64,
    gap: bool,
    app: &mut App,
    active: &mut Option<String>,
) {
    if gap {
        app.notice("Some remote output was omitted. The display remains bounded.");
    }
    for event in events {
        if event.sequence <= app.last_sequence {
            continue;
        }
        match &event.kind {
            ejm_harness::EventKind::RunStarted { .. } => active.clone_from(&event.run_id),
            ejm_harness::EventKind::RunFinished { summary }
                if active.as_deref() == Some(&summary.run_id) =>
            {
                *active = None;
            }
            _ => {}
        }
        app.apply(event);
    }
    if gap {
        app.last_sequence = app.last_sequence.max(through);
    }
    app.run_state = if active.is_some() {
        RunState::Running
    } else {
        RunState::Idle
    };
}

struct NetworkGuard {
    task: JoinHandle<()>,
    cancel: CancellationToken,
}
impl Drop for NetworkGuard {
    fn drop(&mut self) {
        self.cancel.cancel();
        self.task.abort();
    }
}

#[cfg(test)]
#[path = "controller_tests.rs"]
mod tests;
