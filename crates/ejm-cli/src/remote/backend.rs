//! The two session implementations share presentation without opening a second journal.
use super::{
    invitation::ShareRole,
    protocol::{
        ClientRequest, ExpectedState, MAX_PENDING_PROMPTS, MAX_REPLAY_EVENTS, Mutation,
        MutationAction, MutationAdmission, MutationLedger, MutationOutcome, RemoteError,
        ServerMessage,
    },
};
use anyhow::{Context, Result, ensure};
use ejm_harness::{Event, EventKind, Harness, RunHandle, RunStatus, Session};
use std::{
    collections::{BTreeMap, VecDeque},
    ops::Deref,
    sync::Arc,
};
use tokio::sync::Mutex;

pub struct ReplayPage {
    pub events: Vec<Event>,
    pub through_sequence: u64,
    pub gap: bool,
}

pub trait SessionBackend {
    async fn session(&self, id: &str) -> Result<Session>;
    async fn replay(&self, id: &str, after: u64) -> Result<ReplayPage>;
}

#[derive(Clone)]
pub struct LocalBackend {
    harness: Harness,
    authority: Arc<Mutex<Authority>>,
}

#[derive(Default)]
struct Authority {
    sessions: BTreeMap<String, Observed>,
    share: Option<Shared>,
}

#[derive(Default)]
struct Observed {
    sequence: u64,
    authority_sequence: u64,
    active: Option<String>,
    finished: Option<(String, RunStatus)>,
}

struct Shared {
    session: String,
    epoch: String,
    role: ShareRole,
    expires: u64,
    ledger: MutationLedger,
    queue: VecDeque<Queued>,
}

struct Queued {
    sequence: u64,
    run: String,
    text: String,
}

impl Deref for LocalBackend {
    type Target = Harness;
    fn deref(&self) -> &Harness {
        &self.harness
    }
}

impl SessionBackend for LocalBackend {
    async fn session(&self, id: &str) -> Result<Session> {
        self.harness.get_session(id).await
    }
    async fn replay(&self, id: &str, after: u64) -> Result<ReplayPage> {
        let events = self.harness.events(id, after, MAX_REPLAY_EVENTS).await?;
        let through_sequence = events.last().map_or(after, |event| event.sequence);
        Ok(ReplayPage {
            events,
            through_sequence,
            gap: false,
        })
    }
}

impl LocalBackend {
    pub fn new(harness: Harness) -> Self {
        Self {
            harness,
            authority: Arc::new(Mutex::new(Authority::default())),
        }
    }

    pub async fn start(&self, session: &str, text: String) -> Result<RunHandle> {
        let mut state = self.authority.lock().await;
        self.refresh(&mut state, session).await?;
        // Local admission wins this ordering point. Pending remote work cannot follow it.
        Self::discard_pending(&mut state, session);
        let run = self.harness.start(session, text).await?;
        state.sessions.entry(session.into()).or_default().active = Some(run.id.clone());
        Ok(run)
    }

    pub async fn enqueue(&self, session: &str, text: String) -> Result<()> {
        let mut state = self.authority.lock().await;
        self.refresh(&mut state, session).await?;
        Self::discard_pending(&mut state, session);
        self.harness.enqueue(session, text).await
    }

    pub async fn steer(&self, session: &str, text: String) -> Result<()> {
        let mut state = self.authority.lock().await;
        self.refresh(&mut state, session).await?;
        let run = state
            .sessions
            .get(session)
            .and_then(|view| view.active.clone())
            .context("session has no active turn")?;
        self.harness.steer_run(session, &run, text).await
    }

    pub async fn cancel(&self, session: &str) -> Result<()> {
        let mut state = self.authority.lock().await;
        self.refresh(&mut state, session).await?;
        Self::discard_pending(&mut state, session);
        if let Some(run) = state
            .sessions
            .get(session)
            .and_then(|view| view.active.as_deref())
        {
            self.harness.cancel_run(session, run).await?;
        }
        Ok(())
    }

    pub async fn begin_share(
        &self,
        session: &str,
        epoch: String,
        role: ShareRole,
        expires: u64,
    ) -> Result<()> {
        let mut state = self.authority.lock().await;
        ensure!(
            state.share.is_none(),
            "stop the current share before creating another"
        );
        self.refresh(&mut state, session).await?;
        state.share = Some(Shared {
            session: session.into(),
            epoch,
            role,
            expires,
            ledger: MutationLedger::new(128),
            queue: VecDeque::new(),
        });
        Ok(())
    }

    /// The same lock covers admission, pending prompts and revocation acknowledgment.
    /// Late cleanup belongs only to the share that created it.
    pub async fn stop_share_if(&self, epoch: &str) -> bool {
        let mut state = self.authority.lock().await;
        if state
            .share
            .as_ref()
            .is_some_and(|share| share.epoch == epoch)
        {
            state.share = None;
            return true;
        }
        false
    }

    pub async fn shared_session(&self) -> Option<String> {
        self.authority
            .lock()
            .await
            .share
            .as_ref()
            .map(|share| share.session.clone())
    }

    pub async fn is_current_share(&self, epoch: &str, session: &str, now: u64) -> bool {
        self.authority
            .lock()
            .await
            .share
            .as_ref()
            .is_some_and(|share| {
                share.epoch == epoch && share.session == session && now < share.expires
            })
    }

    pub async fn request(
        &self,
        epoch: &str,
        request: ClientRequest,
        now: u64,
    ) -> Result<ServerMessage> {
        let mut state = self.authority.lock().await;
        let share = state.share.as_ref().context("sharing has ended")?;
        ensure!(
            share.epoch == epoch && now < share.expires,
            "sharing has ended"
        );
        let session = share.session.clone();
        self.refresh(&mut state, &session).await?;
        match request {
            ClientRequest::Snapshot {
                after_sequence,
                limit,
            } => {
                let session_info = self.harness.get_session(&session).await?;
                let events = self
                    .harness
                    .events(
                        &session,
                        after_sequence,
                        usize::from(limit).clamp(1, MAX_REPLAY_EVENTS),
                    )
                    .await?;
                let through_sequence = events.last().map_or(after_sequence, |event| event.sequence);
                let active_run = state
                    .sessions
                    .get(&session)
                    .and_then(|view| view.active.clone());
                let share = state.share.as_ref().context("sharing has ended")?;
                let role = share.role;
                let mutation_high_water = share.ledger.high_water();
                Ok(ServerMessage::Snapshot {
                    session: session_info,
                    events,
                    through_sequence,
                    gap: None,
                    active_run,
                    role,
                    mutation_high_water,
                })
            }
            ClientRequest::Outcome { sequence } => {
                let outcome = state
                    .share
                    .as_ref()
                    .context("sharing has ended")?
                    .ledger
                    .lookup(sequence);
                Ok(ServerMessage::Outcome { sequence, outcome })
            }
            ClientRequest::Mutation { mutation } => {
                let sequence = mutation.sequence;
                let admission = state
                    .share
                    .as_mut()
                    .context("sharing has ended")?
                    .ledger
                    .admit(sequence);
                let outcome = match admission {
                    MutationAdmission::Duplicate(outcome) => {
                        return Ok(ServerMessage::Outcome { sequence, outcome });
                    }
                    MutationAdmission::Invalid => {
                        return Ok(ServerMessage::Outcome {
                            sequence,
                            outcome: rejected(
                                RemoteError::InvalidRequest,
                                "invalid mutation sequence",
                            ),
                        });
                    }
                    MutationAdmission::Fresh => self.dispatch(&mut state, &session, mutation).await,
                };
                state
                    .share
                    .as_mut()
                    .context("sharing has ended")?
                    .ledger
                    .record(sequence, outcome.clone())?;
                Ok(ServerMessage::Outcome { sequence, outcome })
            }
        }
    }

    async fn dispatch(
        &self,
        state: &mut Authority,
        session: &str,
        mutation: Mutation,
    ) -> MutationOutcome {
        if mutation.validate().is_err() {
            return rejected(RemoteError::InvalidRequest, "invalid action request");
        }
        if state
            .share
            .as_ref()
            .is_none_or(|share| share.role != ShareRole::Control)
        {
            return rejected(
                RemoteError::AccessDenied,
                "the host granted view access only",
            );
        }
        let observed = state.sessions.entry(session.into()).or_default();
        if mutation.expected.session_id != session
            || mutation.expected.run_id != observed.active
            || !(observed.authority_sequence..=observed.sequence)
                .contains(&mutation.expected.event_sequence)
        {
            return rejected(
                RemoteError::StaleState,
                "session state changed; refresh before acting",
            );
        }
        let result = match mutation.action {
            MutationAction::Submit { text } => {
                if observed.active.is_some() {
                    return rejected(RemoteError::StaleState, "the session is busy");
                }
                match self.harness.start(session, text).await {
                    Ok(run) => {
                        observed.active = Some(run.id.clone());
                        return MutationOutcome::Accepted {
                            run_id: Some(run.id),
                        };
                    }
                    Err(_) => {
                        return rejected(
                            RemoteError::StaleState,
                            "the host could not admit this turn",
                        );
                    }
                }
            }
            MutationAction::Steer { run_id, text } => {
                if observed.active.as_deref() != Some(&run_id) {
                    return stale();
                }
                self.harness.steer_run(session, &run_id, text).await
            }
            MutationAction::Cancel { run_id } => {
                if observed.active.as_deref() != Some(&run_id) {
                    return stale();
                }
                Self::discard_pending(state, session);
                self.harness.cancel_run(session, &run_id).await
            }
            MutationAction::Queue { run_id, text } => {
                if observed.active.as_deref() != Some(&run_id) {
                    return stale();
                }
                let Some(share) = state.share.as_mut() else {
                    return stale();
                };
                if share.queue.len() >= MAX_PENDING_PROMPTS {
                    return rejected(RemoteError::QueueFull, "remote prompt queue is full");
                }
                if text.trim().is_empty() || text.len() > 64 * 1024 {
                    return rejected(
                        RemoteError::InvalidRequest,
                        "prompt must contain 1 to 65536 bytes",
                    );
                }
                share.queue.push_back(Queued {
                    sequence: mutation.sequence,
                    run: run_id,
                    text,
                });
                return MutationOutcome::Queued;
            }
        };
        match result {
            Ok(()) => MutationOutcome::Accepted {
                run_id: state
                    .sessions
                    .get(session)
                    .and_then(|view| view.active.clone()),
            },
            Err(_) => stale(),
        }
    }

    pub async fn settle(&self, now: u64) -> Result<()> {
        let mut state = self.authority.lock().await;
        let Some(share) = state.share.as_ref() else {
            return Ok(());
        };
        if now >= share.expires {
            state.share = None;
            return Ok(());
        }
        let session = share.session.clone();
        self.refresh(&mut state, &session).await?;
        let observed = state
            .sessions
            .get(&session)
            .context("missing session state")?;
        let active = observed.active.clone();
        let finished = observed.finished.clone();
        let Some(share) = state.share.as_mut() else {
            return Ok(());
        };
        let Some(front) = share.queue.front() else {
            return Ok(());
        };
        let Some((run_id, status)) = finished else {
            return Ok(());
        };
        if front.run != run_id {
            return Ok(());
        }
        if status != RunStatus::Completed || active.is_some() {
            Self::discard_pending(&mut state, &session);
            return Ok(());
        }
        let Some(queued) = share.queue.pop_front() else {
            return Ok(());
        };
        if let Ok(run) = self.harness.start(&session, queued.text).await {
            let _ = share.ledger.record(
                queued.sequence,
                MutationOutcome::Accepted {
                    run_id: Some(run.id.clone()),
                },
            );
            // Remaining queued prompts follow only our admitted chain, never another local run.
            for pending in &mut share.queue {
                pending.run.clone_from(&run.id);
            }
            state.sessions.entry(session).or_default().active = Some(run.id);
        } else {
            let _ = share.ledger.record(queued.sequence, stale());
            Self::discard_pending(&mut state, &session);
        }
        Ok(())
    }

    async fn refresh(&self, state: &mut Authority, session: &str) -> Result<()> {
        if !state.sessions.contains_key(session) {
            ensure!(state.sessions.len() < 1000, "session view limit reached");
        }
        let observed = state.sessions.entry(session.into()).or_default();
        loop {
            let page = self
                .harness
                .events(session, observed.sequence, MAX_REPLAY_EVENTS)
                .await?;
            if page.is_empty() {
                break;
            }
            for event in page {
                observed.sequence = event.sequence;
                match event.kind {
                    EventKind::RunStarted { .. } => {
                        observed.active = event.run_id;
                        observed.authority_sequence = event.sequence;
                    }
                    EventKind::RunFinished { summary } => {
                        observed.authority_sequence = event.sequence;
                        if observed.active.as_deref() == Some(&summary.run_id) {
                            observed.active = None;
                        }
                        observed.finished = Some((summary.run_id, summary.status));
                    }
                    _ => {}
                }
            }
            tokio::task::yield_now().await;
        }
        Ok(())
    }

    fn discard_pending(state: &mut Authority, session: &str) {
        if let Some(share) = &mut state.share
            && share.session == session
        {
            for pending in share.queue.drain(..) {
                let _ = share.ledger.record(
                    pending.sequence,
                    rejected(
                        RemoteError::StaleState,
                        "pending remote prompt was discarded by host control",
                    ),
                );
            }
        }
    }
}

pub async fn load_app(backend: &impl SessionBackend, session: Session) -> Result<crate::app::App> {
    let mut app = crate::app::App::new(session);
    loop {
        let page = backend.replay(&app.session.id, app.last_sequence).await?;
        let previous = app.last_sequence;
        let empty = page.events.is_empty();
        for event in page.events {
            app.apply(event);
        }
        if page.gap {
            app.notice("Some remote output was omitted. The display remains bounded.");
            app.last_sequence = app.last_sequence.max(page.through_sequence);
        }
        if empty || app.last_sequence <= previous {
            break;
        }
        tokio::task::yield_now().await;
    }
    let current = backend.session(&app.session.id).await?;
    app.run_state = if current.status == ejm_harness::SessionStatus::Running {
        crate::app::RunState::Running
    } else {
        crate::app::RunState::Idle
    };
    app.session = current;
    Ok(app)
}

fn rejected(code: RemoteError, message: &str) -> MutationOutcome {
    MutationOutcome::Rejected {
        code,
        message: message.into(),
    }
}
fn stale() -> MutationOutcome {
    rejected(
        RemoteError::StaleState,
        "the named run changed; no action was applied",
    )
}

pub fn expected(session: &Session, run_id: Option<String>, event_sequence: u64) -> ExpectedState {
    ExpectedState {
        session_id: session.id.clone(),
        run_id,
        event_sequence,
    }
}

#[cfg(test)]
#[path = "backend_tests.rs"]
mod tests;
