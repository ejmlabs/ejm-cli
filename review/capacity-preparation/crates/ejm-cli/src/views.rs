//! Bounded selected-session presentation. Durable state and queries stay in the harness.
use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

use anyhow::Result;
use ejm_harness::{
    Event, EventPage, SessionEvent, SessionSubscription, SessionTailOptions, SessionView,
};
use futures_util::FutureExt;
use tokio::task::{JoinError, JoinSet};

use crate::{
    app::{App, RunState},
    remote::backend::LocalBackend,
};

const BATCH_EVENTS: usize = 16;
const VIEW_BYTES: usize = 64 * 1024;
const VIEW_EVENTS: usize = 128;

#[cfg(not(test))]
type SelectedSubscription = SessionSubscription;
#[cfg(not(test))]
type Delivery = SessionEvent;

#[cfg(test)]
pub struct SelectedSubscription {
    subscription: SessionSubscription,
    trace: Option<(crate::ui::capacity_fixture::PaintTrace, u64)>,
}

#[cfg(test)]
impl From<SessionSubscription> for SelectedSubscription {
    fn from(subscription: SessionSubscription) -> Self {
        Self {
            subscription,
            trace: None,
        }
    }
}

#[cfg(test)]
pub struct Delivery {
    event: SessionEvent,
    origin: Option<(u64, Instant, Option<Instant>)>,
}

#[cfg(test)]
impl From<SessionEvent> for Delivery {
    fn from(event: SessionEvent) -> Self {
        Self {
            event,
            origin: None,
        }
    }
}

pub enum ViewReply {
    Failed {
        generation: u64,
        session: String,
        error: anyhow::Error,
    },
    Tail {
        generation: u64,
        view: SessionView,
        stream: SelectedSubscription,
    },
    Catchup {
        generation: u64,
        view: SessionView,
        stream: SelectedSubscription,
    },
    Page {
        generation: u64,
        session: String,
        page: EventPage,
    },
}

enum Completion {
    Tail {
        view: SessionView,
        stream: SelectedSubscription,
    },
    Page(EventPage),
}

struct Pending {
    events: VecDeque<Event>,
    completion: Completion,
}

struct Recovery {
    through: u64,
    cursor: u64,
    stream: SelectedSubscription,
}

#[derive(Default)]
pub struct SelectedView {
    pub jobs: JoinSet<ViewReply>,
    pub events: Option<SelectedSubscription>,
    generation: u64,
    control_revision: u64,
    session: String,
    pending: Option<Pending>,
    recovery: Option<Recovery>,
    selected_cursor: u64,
    retry_at: Option<Instant>,
    retry_recovery: bool,
    failure_reported: bool,
    #[cfg(test)]
    trace: Option<crate::ui::capacity_fixture::PaintTrace>,
}

impl SelectedView {
    #[cfg(test)]
    pub(crate) fn with_trace(trace: Option<crate::ui::capacity_fixture::PaintTrace>) -> Self {
        Self {
            trace,
            ..Self::default()
        }
    }

    pub fn joined(
        &mut self,
        reply: Option<Result<ViewReply, JoinError>>,
        harness: &LocalBackend,
        app: &mut App,
    ) {
        match reply {
            Some(Ok(loaded)) => self.loaded(loaded, harness, app),
            Some(Err(error)) if !error.is_cancelled() => {
                #[cfg(test)]
                if let Some(trace) = &self.trace {
                    trace.error("view task", &error.to_string());
                }
                app.error(format!("Load session: {error}"));
            }
            _ => {}
        }
    }

    pub fn follow(&mut self, harness: &LocalBackend, app: &App) {
        if self.session != app.session.id {
            self.failure_reported = false;
            self.snapshot(harness, app, false);
        } else if self
            .retry_at
            .is_some_and(|deadline| Instant::now() >= deadline)
        {
            self.snapshot(harness, app, self.retry_recovery);
        }
    }

    pub fn recover(&mut self, harness: &LocalBackend, app: &App) {
        self.snapshot(harness, app, true);
    }

    fn snapshot(&mut self, harness: &LocalBackend, app: &App, recovery: bool) {
        // Dropping the old set aborts tasks and releases queued stale completions.
        self.jobs = JoinSet::new();
        self.events = None;
        self.pending = None;
        self.recovery = None;
        self.retry_at = None;
        self.retry_recovery = recovery;
        self.generation = self.generation.wrapping_add(1);
        self.control_revision = app.local_control_revision();
        self.session.clone_from(&app.session.id);
        #[cfg(test)]
        if let Some(trace) = &self.trace {
            trace.select(&self.session, self.generation);
        }
        let generation = self.generation;
        let session = self.session.clone();
        let harness = harness.clone();
        #[cfg(test)]
        let trace = self.trace.clone();
        self.jobs.spawn(async move {
            let result: Result<ViewReply> = async {
                // Subscribe first so writes racing the consistent snapshot stay represented.
                #[cfg(not(test))]
                let stream = harness.subscribe_session(&session).await?;
                #[cfg(test)]
                let stream = if let Some(trace) = &trace {
                    let subscription = harness.subscribe_session_traced(&session).await?;
                    trace.subscribed();
                    SelectedSubscription {
                        subscription,
                        trace: Some((trace.clone(), generation)),
                    }
                } else {
                    harness.subscribe_session(&session).await?.into()
                };
                let view = harness
                    .session_view(
                        &session,
                        if recovery {
                            SessionTailOptions {
                                max_events: 0,
                                max_bytes: 0,
                            }
                        } else {
                            bounds()
                        },
                    )
                    .await?;
                Ok(if recovery {
                    ViewReply::Catchup {
                        generation,
                        view,
                        stream,
                    }
                } else {
                    ViewReply::Tail {
                        generation,
                        view,
                        stream,
                    }
                })
            }
            .await;
            result.unwrap_or_else(|error| ViewReply::Failed {
                generation,
                session,
                error,
            })
        });
    }

    pub fn loaded(&mut self, reply: ViewReply, harness: &LocalBackend, app: &mut App) {
        match reply {
            ViewReply::Failed {
                generation,
                session,
                error,
            } => {
                if generation != self.generation || session != app.session.id {
                    return;
                }
                #[cfg(test)]
                if let Some(trace) = &self.trace {
                    trace.error("view load", &format!("{error:#}"));
                }
                self.retry_at = Some(Instant::now() + Duration::from_secs(1));
                if !self.failure_reported {
                    app.error(format!("Load session: {error:#}. Retrying."));
                    self.failure_reported = true;
                }
            }
            ViewReply::Tail {
                generation,
                view,
                stream,
            } => {
                if generation != self.generation || view.session.id != app.session.id {
                    return;
                }
                if self.control_revision != app.local_control_revision() {
                    self.snapshot(harness, app, false);
                    return;
                }
                self.load_tail(view, stream, app);
            }
            ViewReply::Catchup {
                generation,
                view,
                stream,
            } => {
                if generation != self.generation || view.session.id != app.session.id {
                    return;
                }
                if self.control_revision != app.local_control_revision() {
                    self.snapshot(harness, app, true);
                    return;
                }
                app.apply_run_snapshot(&view);
                app.session = view.session;
                app.status = app
                    .stop_status()
                    .unwrap_or("Recovering saved output…")
                    .into();
                app.dirty = true;
                self.recovery = Some(Recovery {
                    through: view.durable_cursor,
                    cursor: app.last_sequence,
                    stream,
                });
                self.page(harness);
            }
            ViewReply::Page {
                generation,
                session,
                mut page,
            } => {
                if generation != self.generation || session != app.session.id {
                    return;
                }
                if page.omitted {
                    // One event cannot fit the caller's byte limit. A fresh tail gives an
                    // explicit omission marker instead of silently advancing the cursor.
                    self.snapshot(harness, app, false);
                    return;
                }
                let events = std::mem::take(&mut page.events).into();
                self.pending = Some(Pending {
                    events,
                    completion: Completion::Page(page),
                });
            }
        }
    }

    fn load_tail(&mut self, mut view: SessionView, stream: SelectedSubscription, app: &mut App) {
        let events = std::mem::take(&mut view.events).into();
        let mut next = App::new(view.session.clone());
        next.run_state = app.run_state;
        if app.last_sequence == 0 {
            for block in app.blocks.drain(..) {
                if block.kind == crate::app::BlockKind::Error {
                    next.error(block.text);
                } else {
                    next.notice(block.text);
                }
            }
        }
        next.overlay = app.overlay.take();
        next.remote = app.remote.take();
        next.command_picker = std::mem::take(&mut app.command_picker);
        if view.omitted {
            next.notice(
                "Earlier output is outside this bounded view. Durable history remains saved.",
            );
        }
        next.status = "Loading saved output…".into();
        app.switch_session(next);
        app.apply_run_snapshot(&view);
        if let Some(status) = app.stop_status() {
            app.status = status.into();
        }
        self.pending = Some(Pending {
            events,
            completion: Completion::Tail { view, stream },
        });
    }

    fn page(&mut self, harness: &LocalBackend) {
        let Some(recovery) = &self.recovery else {
            return;
        };
        let (after, through) = (recovery.cursor, recovery.through);
        let session = self.session.clone();
        let generation = self.generation;
        let harness = harness.clone();
        self.jobs.spawn(async move {
            match harness
                .events_page(&session, after, through, bounds())
                .await
            {
                Ok(page) => ViewReply::Page {
                    generation,
                    session,
                    page,
                },
                Err(error) => ViewReply::Failed {
                    generation,
                    session,
                    error,
                },
            }
        });
    }

    pub fn has_pending_batch(&self) -> bool {
        self.pending.is_some()
    }

    /// Saved and ready live events share one short, bounded work allowance.
    pub fn apply_batch(&mut self, harness: &LocalBackend, app: &mut App) {
        if self.session != app.session.id {
            self.follow(harness, app);
            return;
        }
        let started = Instant::now();
        for _ in 0..BATCH_EVENTS {
            if let Some(pending) = &mut self.pending {
                if let Some(event) = pending.events.pop_front() {
                    #[cfg(test)]
                    if event.session_id == app.session.id
                        && event.sequence > app.last_sequence
                        && let Some(trace) = &self.trace
                    {
                        trace.saved(&event);
                    }
                    app.apply_saved(event);
                }
                if pending.events.is_empty() {
                    self.complete_batch(harness, app);
                    break;
                }
            } else {
                // Tick and paint have priority over the async receive branch.
                // Service ready output here without waiting or starving input.
                match receive(&mut self.events).now_or_never() {
                    Some(Ok(event)) => self.event(event, harness, app),
                    Some(Err(tokio::sync::broadcast::error::RecvError::Lagged(_))) => {
                        self.recover(harness, app);
                        break;
                    }
                    Some(Err(tokio::sync::broadcast::error::RecvError::Closed)) | None => break,
                }
                if self.events.is_none() {
                    break;
                }
            }
            if started.elapsed() >= Duration::from_millis(2) {
                break;
            }
        }
    }

    fn complete_batch(&mut self, harness: &LocalBackend, app: &mut App) {
        let Some(pending) = self.pending.take() else {
            return;
        };
        match pending.completion {
            Completion::Tail { view, stream } => {
                self.failure_reported = false;
                app.last_sequence = view.selected_cursor;
                self.selected_cursor = view.selected_cursor;
                app.session = view.session;
                show_current_status(app);
                self.events = Some(stream);
            }
            Completion::Page(page) => {
                if let Some(cursor) = page.selected_cursor {
                    self.selected_cursor = cursor;
                }
                if let Some(recovery) = &mut self.recovery {
                    recovery.cursor = page.cursor;
                }
                if page.complete {
                    self.failure_reported = false;
                    if let Some(recovery) = self.recovery.take() {
                        self.events = Some(recovery.stream);
                    }
                    show_current_status(app);
                } else {
                    self.page(harness);
                }
            }
        }
    }

    pub fn event(&mut self, event: impl Into<Delivery>, harness: &LocalBackend, app: &mut App) {
        let event = event.into();
        #[cfg(test)]
        let Delivery { event, origin } = event;
        #[cfg(test)]
        if origin.is_some_and(|(generation, _, _)| generation != self.generation) {
            if let Some(trace) = &self.trace {
                trace.omit(&event.event);
            }
            return;
        }
        if event.event.session_id != app.session.id || event.event.sequence <= self.selected_cursor
        {
            return;
        }
        if event.previous_sequence != self.selected_cursor {
            #[cfg(test)]
            if let Some(trace) = &self.trace {
                trace.omit(&event.event);
            }
            self.recover(harness, app);
            return;
        }
        // Serialization stops as soon as the byte allowance is exhausted, without
        // copying a possibly large live payload into another buffer.
        if serde_json::to_writer(ByteBudget(VIEW_BYTES), &event.event).is_err() {
            #[cfg(test)]
            if let Some(trace) = &self.trace {
                trace.omit(&event.event);
            }
            self.snapshot(harness, app, false);
            return;
        }
        self.selected_cursor = event.event.sequence;
        #[cfg(test)]
        if let Some(trace) = &self.trace {
            trace.accepted(&event.event, self.generation, origin);
        }
        app.apply(event.event);
    }
}

fn show_current_status(app: &mut App) {
    app.status = app
        .stop_status()
        .unwrap_or(match app.run_state {
            RunState::Running => "Running",
            RunState::Stopping => "Stopping…",
            RunState::Idle if app.connection_ready => "Ready",
            RunState::Idle => "Not connected",
        })
        .into();
    app.dirty = true;
}

fn bounds() -> SessionTailOptions {
    SessionTailOptions {
        max_events: VIEW_EVENTS,
        max_bytes: VIEW_BYTES,
    }
}

pub async fn receive(
    subscription: &mut Option<SelectedSubscription>,
) -> Result<Delivery, tokio::sync::broadcast::error::RecvError> {
    match subscription {
        Some(subscription) => {
            #[cfg(test)]
            {
                if let Some((trace, generation)) = &subscription.trace {
                    return match subscription.subscription.recv_traced().await {
                        Ok((event, committed)) => Ok(Delivery {
                            event,
                            origin: Some((*generation, Instant::now(), committed)),
                        }),
                        Err(error) => {
                            trace.receive_error(&error);
                            Err(error)
                        }
                    };
                }
                subscription.subscription.recv().await.map(Into::into)
            }
            #[cfg(not(test))]
            subscription.recv().await
        }
        None => std::future::pending().await,
    }
}

struct ByteBudget(usize);

impl std::io::Write for ByteBudget {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0 = self
            .0
            .checked_sub(bytes.len())
            .ok_or_else(|| std::io::Error::other("view byte budget exceeded"))?;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use anyhow::{Context, Result};
    use clap::Parser;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ejm_harness::{
        EventKind, Harness, HarnessConfig, ModelResponse, Provider, ReplayProvider, RunHandle,
        RunStatus, SessionOptions,
    };
    use ejm_harness_bench::Workload;
    use std::sync::Arc;
    use tokio::{sync::watch, time::timeout};

    async fn fixture() -> Result<(tempfile::TempDir, LocalBackend, App)> {
        fixture_with_provider(ReplayProvider::echo()).await
    }

    async fn fixture_with_provider(
        provider: impl Provider + 'static,
    ) -> Result<(tempfile::TempDir, LocalBackend, App)> {
        let directory = tempfile::tempdir()?;
        let harness = Harness::new(
            HarnessConfig {
                state_dir: directory.path().join("sessions"),
                ..HarnessConfig::default()
            },
            Arc::new(provider),
            Vec::new(),
        )?;
        let session = harness
            .create_session(SessionOptions {
                cwd: directory.path().into(),
                title: "selected view".into(),
            })
            .await?;
        Ok((directory, LocalBackend::new(harness), App::new(session)))
    }

    async fn next_reply(view: &mut SelectedView) -> Result<ViewReply> {
        timeout(Duration::from_secs(5), view.jobs.join_next())
            .await?
            .context("view task missing")?
            .map_err(Into::into)
    }

    pub(crate) fn finish_batch_page(
        view: &mut SelectedView,
        harness: &LocalBackend,
        app: &mut App,
    ) {
        for _ in 0..VIEW_EVENTS {
            if view.pending.is_none() {
                return;
            }
            view.apply_batch(harness, app);
        }
        assert!(view.pending.is_none(), "bounded page did not finish");
    }

    pub(crate) async fn consume_through(
        view: &mut SelectedView,
        harness: &LocalBackend,
        app: &mut App,
        through: u64,
    ) -> Result<()> {
        for _ in 0..VIEW_EVENTS {
            if app.last_sequence >= through {
                return Ok(());
            }
            let event = timeout(Duration::from_secs(5), receive(&mut view.events)).await??;
            view.event(event, harness, app);
        }
        anyhow::bail!("selected stream did not reach its durable endpoint")
    }

    pub(crate) struct RecoveryFixture {
        _directory: tempfile::TempDir,
        _ready: watch::Sender<bool>,
        pub(crate) harness: LocalBackend,
        pub(crate) app: App,
        pub(crate) view: SelectedView,
        pub(crate) run: RunHandle,
        pub(crate) through: u64,
    }

    async fn saved_turns(harness: &LocalBackend, session: &str, count: usize) -> Result<usize> {
        let mut first_turn_events = 0;
        for index in 0..count {
            let summary = harness
                .start(session, format!("saved-{index}"))
                .await?
                .wait()
                .await?;
            assert_eq!(summary.status, RunStatus::Completed);
            if index == 0 {
                first_turn_events = harness.events(session, 0, VIEW_EVENTS).await?.len();
            }
        }
        Ok(first_turn_events)
    }

    /// Leave a real run live after an old completion, with durable recovery still pending.
    pub(crate) async fn recovering_fixture() -> Result<RecoveryFixture> {
        let (directory, harness, mut app) = fixture().await?;
        let mut view = SelectedView::default();
        view.follow(&harness, &app);
        let loaded = next_reply(&mut view).await?;
        view.loaded(loaded, &harness, &mut app);
        finish_batch_page(&mut view, &harness, &mut app);
        let first_turn_events = saved_turns(&harness, &app.session.id, 42).await?;
        assert!(matches!(
            receive(&mut view.events).await,
            Err(tokio::sync::broadcast::error::RecvError::Lagged(_))
        ));
        let (ready, gate) = watch::channel(false);
        harness
            .configure_provider(Some(Arc::new(Workload::new(gate, 0, 0)?)), "view fixture")
            .await?;
        let run = harness
            .start(&app.session.id, "held during recovery".into())
            .await?;
        view.recover(&harness, &app);
        let loaded = next_reply(&mut view).await?;
        let ViewReply::Catchup { view: snapshot, .. } = &loaded else {
            anyhow::bail!("expected recovery authority");
        };
        assert_eq!(snapshot.run_id.as_deref(), Some(run.id.as_str()));
        let through = snapshot.durable_cursor;
        view.loaded(loaded, &harness, &mut app);
        // A short valid page forces a historical completion at the return to input.
        // Later pages keep the normal production count and byte bounds.
        let loaded = next_reply(&mut view).await?;
        let ViewReply::Page {
            generation,
            session,
            ..
        } = loaded
        else {
            anyhow::bail!("expected recovery page");
        };
        let page = harness
            .events_page(
                &session,
                app.last_sequence,
                through,
                SessionTailOptions {
                    max_events: first_turn_events,
                    max_bytes: VIEW_BYTES,
                },
            )
            .await?;
        assert!(matches!(
            page.events.last().map(|event| &event.kind),
            Some(EventKind::RunFinished { .. })
        ));
        view.loaded(
            ViewReply::Page {
                generation,
                session,
                page,
            },
            &harness,
            &mut app,
        );
        finish_batch_page(&mut view, &harness, &mut app);
        assert!(app.last_sequence < through);
        Ok(RecoveryFixture {
            _directory: directory,
            _ready: ready,
            harness,
            app,
            view,
            run,
            through,
        })
    }

    #[tokio::test]
    async fn ticks_service_ready_live_output_within_the_shared_event_allowance() -> Result<()> {
        let (_directory, harness, mut app) = fixture().await?;
        let mut view = SelectedView::default();
        view.follow(&harness, &app);
        let loaded = next_reply(&mut view).await?;
        view.loaded(loaded, &harness, &mut app);
        finish_batch_page(&mut view, &harness, &mut app);

        saved_turns(&harness, &app.session.id, 4).await?;
        let events = harness.events(&app.session.id, 0, VIEW_EVENTS).await?;
        assert!(events.len() > BATCH_EVENTS && events.len() < 64);
        let through = events.last().context("saved live output missing")?.sequence;
        // Service only tick-side work, as when timer branches keep winning.
        // No receive await may be required to make selected output progress.
        for _ in 0..VIEW_EVENTS {
            tokio::task::yield_now().await;
            let before = app.last_sequence;
            view.apply_batch(&harness, &mut app);
            let applied = events
                .iter()
                .filter(|event| event.sequence > before && event.sequence <= app.last_sequence)
                .count();
            assert!((1..=BATCH_EVENTS).contains(&applied));
            if app.last_sequence == through {
                break;
            }
        }
        assert_eq!(app.last_sequence, through);
        assert!(view.pending.is_none() && view.events.is_some());
        view.apply_batch(&harness, &mut app);
        assert_eq!(app.last_sequence, through);
        harness.shutdown().await?;
        Ok(())
    }

    #[tokio::test]
    async fn unselected_flood_never_enters_or_dirties_the_selected_view() -> Result<()> {
        let (ready, gate) = watch::channel(false);
        let (directory, harness, mut app) =
            fixture_with_provider(Workload::new(gate, 0, 128)?).await?;
        let mut view = SelectedView::default();
        view.follow(&harness, &app);
        let loaded = next_reply(&mut view).await?;
        view.loaded(loaded, &harness, &mut app);
        finish_batch_page(&mut view, &harness, &mut app);
        app.dirty = false;
        let mut runs = Vec::new();
        for index in 0..16 {
            let session = harness
                .create_session(SessionOptions {
                    cwd: directory.path().into(),
                    title: format!("unselected-{index}"),
                })
                .await?;
            runs.push(
                harness
                    .start(&session.id, "background flood".into())
                    .await?,
            );
        }
        assert_eq!(harness.status().await.active_agents, runs.len());
        ready.send(true)?;
        let mut events = 0;
        for mut run in runs {
            assert_eq!(run.wait().await?.status, RunStatus::Completed);
            events += harness.events(&run.session_id, 0, 1000).await?.len();
        }
        assert!(events > 2000, "fixture did not produce a background flood");
        assert!(receive(&mut view.events).now_or_never().is_none());
        assert!(!app.dirty);
        assert!(app.blocks.is_empty());
        assert_eq!(app.last_sequence, 0);
        assert_eq!(harness.status().await.active_agents, 0);
        harness.shutdown().await?;
        Ok(())
    }

    #[tokio::test]
    async fn snapshot_duplicates_and_completion_during_loading_are_applied_once() -> Result<()> {
        let (ready, gate) = watch::channel(false);
        let (_directory, harness, mut app) =
            fixture_with_provider(Workload::new(gate, 0, 0)?.with_markers(true)).await?;
        let stream = harness.subscribe_session(&app.session.id).await?;
        let mut run = harness
            .start(&app.session.id, "snapshot race".into())
            .await?;
        let snapshot = harness.session_view(&app.session.id, bounds()).await?;
        assert_eq!(snapshot.run_id.as_deref(), Some(run.id.as_str()));
        let selected_cursor = snapshot.selected_cursor;
        ready.send(true)?;
        assert_eq!(run.wait().await?.status, RunStatus::Completed);
        let through = harness
            .session_view(&app.session.id, bounds())
            .await?
            .selected_cursor;
        assert!(through > selected_cursor);
        let mut view = SelectedView {
            generation: 1,
            session: app.session.id.clone(),
            ..SelectedView::default()
        };
        view.loaded(
            ViewReply::Tail {
                generation: 1,
                view: snapshot,
                stream: stream.into(),
            },
            &harness,
            &mut app,
        );
        // A Stop accepted while this captured Running tail loads must survive
        // finalization, including when the backend has already finished the run.
        let _ = harness.cancel(&app.session.id).await?;
        app.set_local_run_state(RunState::Stopping);
        finish_batch_page(&mut view, &harness, &mut app);
        assert_eq!(app.run_state, RunState::Stopping);
        consume_through(&mut view, &harness, &mut app, through).await?;
        assert_eq!(app.run_state, RunState::Idle);
        assert_eq!(view.selected_cursor, through);
        let users: Vec<_> = app
            .blocks
            .iter()
            .filter(|block| block.kind == crate::app::BlockKind::User)
            .map(|block| block.text.as_str())
            .collect();
        assert_eq!(users, ["snapshot race"]);
        let replies: Vec<_> = app
            .blocks
            .iter()
            .filter(|block| block.kind == crate::app::BlockKind::Assistant)
            .map(|block| block.text.as_str())
            .collect();
        assert_eq!(
            replies,
            ["response:snapshot race:round-0:\nVerified all local work.\n"]
        );
        assert!(receive(&mut view.events).now_or_never().is_none());
        harness.shutdown().await?;
        Ok(())
    }

    async fn idle_snapshot_after_local_start(recovery: bool) -> Result<()> {
        let (_ready, gate) = watch::channel(false);
        let (_directory, harness, mut app) =
            fixture_with_provider(Workload::new(gate, 0, 0)?).await?;
        let mut view = SelectedView::default();
        if recovery {
            view.recover(&harness, &app);
        } else {
            view.follow(&harness, &app);
        }
        let old_reply = next_reply(&mut view).await?;
        let (ViewReply::Tail { view: snapshot, .. } | ViewReply::Catchup { view: snapshot, .. }) =
            &old_reply
        else {
            anyhow::bail!("expected captured idle authority");
        };
        assert!(snapshot.run_id.is_none());
        let old_generation = view.generation;
        let mut run = harness
            .start(&app.session.id, "newer local start".into())
            .await?;
        app.set_local_run_state(RunState::Running);
        app.status = "Starting…".into();
        app.composer.replace("draft after start");
        let revision = app.local_control_revision();
        view.loaded(old_reply, &harness, &mut app);
        assert_eq!(app.run_state, RunState::Running);
        assert_eq!(app.status, "Starting…");
        assert_eq!(app.local_control_revision(), revision);
        assert!(view.pending.is_none());
        assert!(view.recovery.is_none());
        assert_ne!(view.generation, old_generation);
        let current = next_reply(&mut view).await?;
        let (ViewReply::Tail { view: snapshot, .. } | ViewReply::Catchup { view: snapshot, .. }) =
            &current
        else {
            anyhow::bail!("expected refreshed current authority");
        };
        assert_eq!(snapshot.run_id.as_deref(), Some(run.id.as_str()));
        view.loaded(current, &harness, &mut app);
        assert_eq!(app.run_state, RunState::Running);
        assert_eq!(app.local_control_revision(), revision);
        let _ = harness.cancel(&app.session.id).await?;
        app.set_local_run_state(RunState::Stopping);
        assert_eq!(run.wait().await?.status, RunStatus::Cancelled);
        if recovery {
            let page = next_reply(&mut view).await?;
            view.loaded(page, &harness, &mut app);
        }
        finish_batch_page(&mut view, &harness, &mut app);
        assert_eq!(app.run_state, RunState::Stopping);
        let through = harness
            .session_view(&app.session.id, bounds())
            .await?
            .selected_cursor;
        consume_through(&mut view, &harness, &mut app, through).await?;
        assert_eq!(app.run_state, RunState::Idle);
        assert_eq!(app.composer.text, "draft after start");
        harness.shutdown().await?;
        Ok(())
    }

    #[tokio::test]
    async fn idle_tail_cannot_override_a_newer_local_start() -> Result<()> {
        idle_snapshot_after_local_start(false).await
    }

    #[tokio::test]
    async fn idle_recovery_snapshot_cannot_override_a_newer_local_start() -> Result<()> {
        idle_snapshot_after_local_start(true).await
    }

    #[tokio::test]
    async fn long_saved_history_loads_only_a_bounded_tail_without_historical_run_authority()
    -> Result<()> {
        let (_directory, harness, mut app) = fixture().await?;
        saved_turns(&harness, &app.session.id, 42).await?;
        let durable_events = harness.events(&app.session.id, 0, 1000).await?.len();
        assert!(durable_events > VIEW_EVENTS);
        app.composer.replace("saved-history draft");
        let mut view = SelectedView::default();
        view.follow(&harness, &app);
        let loaded = next_reply(&mut view).await?;
        let ViewReply::Tail { view: snapshot, .. } = &loaded else {
            anyhow::bail!("expected a bounded saved tail");
        };
        assert!(snapshot.omitted);
        assert!(snapshot.events.len() <= VIEW_EVENTS);
        assert!(snapshot.events.len() < durable_events);
        let bytes = snapshot.events.iter().try_fold(0, |total, event| {
            serde_json::to_vec(event).map(|event| total + event.len())
        })?;
        assert!(bytes <= VIEW_BYTES);
        let selected_cursor = snapshot.selected_cursor;
        view.loaded(loaded, &harness, &mut app);
        for _ in 0..VIEW_EVENTS {
            if view.pending.is_none() {
                break;
            }
            let before = view.pending.as_ref().context("tail missing")?.events.len();
            view.apply_batch(&harness, &mut app);
            let after = view
                .pending
                .as_ref()
                .map_or(0, |pending| pending.events.len());
            assert!((1..=BATCH_EVENTS).contains(&(before - after)));
            assert_eq!(app.run_state, RunState::Idle);
            assert_eq!(app.composer.text, "saved-history draft");
        }
        assert!(view.pending.is_none());
        assert_eq!(app.last_sequence, selected_cursor);
        assert_eq!(
            harness.events(&app.session.id, 0, 1000).await?.len(),
            durable_events
        );
        harness.shutdown().await?;
        Ok(())
    }

    #[tokio::test]
    async fn recovery_returns_to_input_and_stop_while_appends_continue_past_its_endpoint()
    -> Result<()> {
        let mut fixture = recovering_fixture().await?;
        let RecoveryFixture {
            harness,
            app,
            view,
            run,
            through,
            ..
        } = &mut fixture;
        assert_eq!(app.run_state, RunState::Running);
        app.composer.replace("draft");
        let mut batches = 0;
        let mut stopped = false;
        for _ in 0..512 {
            if view.events.is_some() {
                break;
            }
            if view.pending.is_none() {
                let loaded = next_reply(view).await?;
                let ViewReply::Page { page, .. } = &loaded else {
                    anyhow::bail!("expected continued bounded recovery");
                };
                assert_eq!(page.through, *through);
                assert!(page.events.len() <= VIEW_EVENTS);
                view.loaded(loaded, harness, app);
            }
            let before = view.pending.as_ref().context("missing page")?.events.len();
            view.apply_batch(harness, app);
            let after = view
                .pending
                .as_ref()
                .map_or(0, |pending| pending.events.len());
            assert!((1..=BATCH_EVENTS).contains(&(before - after)));
            assert_eq!(
                view.recovery.as_ref().map(|value| value.through),
                view.events.is_none().then_some(*through)
            );
            crate::ui::edit_composer(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE), app);
            batches += 1;
            if stopped {
                assert_eq!(app.run_state, RunState::Stopping);
            } else {
                harness
                    .steer(&app.session.id, format!("live-{batches}"))
                    .await?;
                if batches == 3 {
                    assert!(app.last_sequence < *through);
                    let _ = harness.cancel(&app.session.id).await?;
                    app.set_local_run_state(RunState::Stopping);
                    assert_eq!(run.wait().await?.status, RunStatus::Cancelled);
                    assert!(view.recovery.is_some());
                    stopped = true;
                }
            }
        }
        assert!(stopped);
        assert!(view.events.is_some(), "recovery chased continued appends");
        assert_eq!(app.composer.text, format!("draft{}", "x".repeat(batches)));
        let current = harness.session_view(&app.session.id, bounds()).await?;
        assert!(current.selected_cursor > *through);
        consume_through(view, harness, app, current.selected_cursor).await?;
        assert_eq!(app.run_state, RunState::Idle);
        let users: Vec<_> = app
            .blocks
            .iter()
            .filter(|block| block.kind == crate::app::BlockKind::User)
            .map(|block| block.text.clone())
            .collect();
        let mut expected: Vec<_> = (0..42).map(|index| format!("saved-{index}")).collect();
        expected.push("held during recovery".into());
        assert_eq!(users, expected);
        for index in 0..42 {
            assert_eq!(
                app.blocks
                    .iter()
                    .filter(|block| block.kind == crate::app::BlockKind::Assistant
                        && block.text == format!("Demo: saved-{index}"))
                    .count(),
                1
            );
        }
        assert_eq!(
            app.blocks
                .iter()
                .filter(|block| block.title == "steering")
                .count(),
            3
        );
        assert!(view.jobs.is_empty());
        harness.shutdown().await?;
        Ok(())
    }

    #[tokio::test]
    async fn selected_stream_accepts_global_gaps_and_recovers_proven_loss() -> Result<()> {
        let (_directory, harness, mut app) = fixture().await?;
        let mut view = SelectedView {
            selected_cursor: 10,
            session: app.session.id.clone(),
            ..SelectedView::default()
        };
        app.last_sequence = 10;
        let event = |sequence, previous_sequence| SessionEvent {
            event: Event {
                sequence,
                session_id: app.session.id.clone(),
                run_id: None,
                timestamp_ms: 0,
                kind: EventKind::TextDelta {
                    text: "visible".into(),
                },
            },
            previous_sequence,
        };
        let continuous = event(14, 10);
        let lost = event(20, 18);
        view.event(continuous, &harness, &mut app);
        assert_eq!(app.last_sequence, 14);
        assert!(view.jobs.is_empty());
        app.composer.insert("keep draft");
        view.event(lost, &harness, &mut app);
        assert_eq!(app.last_sequence, 14);
        assert!(!view.jobs.is_empty());
        assert_eq!(app.composer.text, "keep draft");
        harness.shutdown().await?;
        Ok(())
    }

    #[tokio::test]
    async fn stale_snapshot_cannot_replace_new_selection_or_draft() -> Result<()> {
        let (directory, harness, mut app) = fixture().await?;
        let mut view = SelectedView::default();
        view.follow(&harness, &app);
        let loaded = view.jobs.join_next().await.context("view task missing")??;
        let next = harness
            .create_session(SessionOptions {
                cwd: directory.path().into(),
                title: "next selection".into(),
            })
            .await?;
        app.composer.insert("saved draft");
        app.switch_session(App::new(next.clone()));
        view.follow(&harness, &app);
        view.loaded(loaded, &harness, &mut app);
        assert_eq!(app.session.id, next.id);
        assert_eq!(app.composer.text, "saved draft");
        assert!(view.pending.is_none());
        harness.shutdown().await?;
        Ok(())
    }

    #[tokio::test]
    async fn switching_after_a_tail_is_accepted_discards_its_pending_batches() -> Result<()> {
        let (directory, harness, mut app) = fixture().await?;
        let mut view = SelectedView::default();
        view.follow(&harness, &app);
        let loaded = next_reply(&mut view).await?;
        view.loaded(loaded, &harness, &mut app);
        assert!(view.pending.is_some());
        let next = harness
            .create_session(SessionOptions {
                cwd: directory.path().into(),
                title: "switch before pending tail applies".into(),
            })
            .await?;
        app.composer.insert("keep this selection's draft");
        app.switch_session(App::new(next.clone()));
        // The view itself rejects pending work even before the loop calls follow.
        view.apply_batch(&harness, &mut app);
        assert_eq!(app.session.id, next.id);
        assert_eq!(app.last_sequence, 0);
        assert_eq!(app.composer.text, "keep this selection's draft");
        assert!(view.pending.is_none());
        assert!(view.recovery.is_none());
        assert_eq!(view.session, next.id);
        harness.shutdown().await?;
        Ok(())
    }

    #[tokio::test]
    async fn stale_recovery_completions_preserve_the_draft_selection_and_share() -> Result<()> {
        let (directory, harness, mut app) = fixture().await?;
        let mut view = SelectedView::default();
        view.recover(&harness, &app);
        let old_catchup = next_reply(&mut view).await?;
        view.recover(&harness, &app);
        let current_catchup = next_reply(&mut view).await?;
        view.loaded(current_catchup, &harness, &mut app);
        let old_page = next_reply(&mut view).await?;
        view.recover(&harness, &app);
        view.loaded(old_catchup, &harness, &mut app);
        view.loaded(old_page, &harness, &mut app);
        assert!(view.pending.is_none());
        assert!(view.recovery.is_none());
        let old_catchup = next_reply(&mut view).await?;
        let old_generation = view.generation;
        let old_session = app.session.id.clone();
        let next = harness
            .create_session(SessionOptions {
                cwd: directory.path().into(),
                title: "new shared selection".into(),
            })
            .await?;
        app.composer.replace("current draft");
        app.switch_session(App::new(next.clone()));
        app.remote = Some(crate::app::RemoteBadge {
            label: "current share".into(),
            controller: false,
        });
        harness
            .begin_share(
                &next.id,
                "current-epoch".into(),
                crate::remote::invitation::ShareRole::View,
                100,
            )
            .await?;
        view.follow(&harness, &app);
        app.dirty = false;
        view.loaded(old_catchup, &harness, &mut app);
        view.loaded(
            ViewReply::Failed {
                generation: old_generation,
                session: old_session,
                error: anyhow::anyhow!("old recovery failed"),
            },
            &harness,
            &mut app,
        );
        assert!(!app.dirty);
        assert!(view.pending.is_none());
        assert!(view.recovery.is_none());
        assert_eq!(app.session.id, next.id);
        assert_eq!(app.composer.text, "current draft");
        assert_eq!(
            app.remote.as_ref().map(|badge| badge.label.as_str()),
            Some("current share")
        );
        assert!(harness.is_current_share("current-epoch", &next.id, 1).await);
        let current = next_reply(&mut view).await?;
        view.loaded(current, &harness, &mut app);
        finish_batch_page(&mut view, &harness, &mut app);
        assert_eq!(app.session.id, next.id);
        assert_eq!(app.composer.text, "current draft");
        assert_eq!(
            app.remote.as_ref().map(|badge| badge.label.as_str()),
            Some("current share")
        );
        assert!(harness.is_current_share("current-epoch", &next.id, 1).await);
        harness.stop_share().await;
        harness.shutdown().await?;
        Ok(())
    }

    #[tokio::test]
    async fn initial_tail_preserves_host_notices_and_connection_state() -> Result<()> {
        let (_directory, harness, mut app) = fixture().await?;
        let mut view = SelectedView::default();
        view.follow(&harness, &app);
        app.notice("Model connection failed; choose another provider.");
        app.connection_ready = false;
        app.composer.insert("keep draft");
        app.overlay = Some(crate::app::Overlay::Login { selected: 2 });
        let loaded = view.jobs.join_next().await.context("view task missing")??;
        view.loaded(loaded, &harness, &mut app);
        view.apply_batch(&harness, &mut app);
        assert!(
            app.blocks
                .iter()
                .any(|block| block.text.contains("Model connection failed"))
        );
        assert!(!app.connection_ready);
        assert_eq!(app.status, "Not connected");
        assert_eq!(app.composer.text, "keep draft");
        assert!(matches!(
            app.overlay,
            Some(crate::app::Overlay::Login { selected: 2 })
        ));
        harness.shutdown().await?;
        Ok(())
    }

    #[tokio::test]
    async fn omitted_tail_draws_a_notice_without_changing_durable_history_or_authority()
    -> Result<()> {
        let (_directory, harness, mut app) =
            fixture_with_provider(ReplayProvider::new(vec![ModelResponse {
                // The stream fits, while serialized event metadata exceeds the view bound.
                content: "s".repeat(VIEW_BYTES),
                ..ModelResponse::default()
            }]))
            .await?;
        let summary = harness
            .start(&app.session.id, "read saved output".into())
            .await?
            .wait()
            .await?;
        assert_eq!(summary.status, RunStatus::Completed, "{:?}", summary.error);
        let history = harness.events(&app.session.id, 0, 1000).await?;
        let durable_before = serde_json::to_vec(&history)?;
        let mut view = SelectedView::default();
        view.follow(&harness, &app);
        let loaded = view.jobs.join_next().await.context("view task missing")??;
        let ViewReply::Tail { view: tail, .. } = &loaded else {
            anyhow::bail!("expected initial saved tail");
        };
        assert!(tail.omitted);
        assert!(tail.events.len() < history.len());
        let selected_cursor = tail.selected_cursor;
        view.loaded(loaded, &harness, &mut app);
        while view.pending.is_some() {
            view.apply_batch(&harness, &mut app);
        }
        assert_eq!(app.last_sequence, selected_cursor);
        assert!(!app.is_running());
        let notice = app
            .blocks
            .iter()
            .find(|block| block.kind == crate::app::BlockKind::Notice)
            .context("omitted saved tail has no display notice")?
            .text
            .clone();
        let args = crate::args::Args::parse_from(["ejm-cli", "--plain", "--no-color"]);
        let snapshot = harness.status().await;
        let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(120, 30))?;
        terminal.draw(|frame| crate::render::draw(frame, &mut app, &args, &snapshot))?;
        let displayed = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect::<String>();
        assert!(displayed.contains(&notice));
        assert_eq!(
            serde_json::to_vec(&harness.events(&app.session.id, 0, 1000).await?)?,
            durable_before
        );
        harness.shutdown().await?;
        Ok(())
    }

    #[tokio::test]
    async fn oversized_live_payload_reloads_a_bounded_tail_and_preserves_the_draft() -> Result<()> {
        let (_directory, harness, mut app) =
            fixture_with_provider(ReplayProvider::new(vec![ModelResponse {
                content: "x".repeat(VIEW_BYTES),
                ..ModelResponse::default()
            }]))
            .await?;
        let mut view = SelectedView::default();
        view.follow(&harness, &app);
        let loaded = next_reply(&mut view).await?;
        view.loaded(loaded, &harness, &mut app);
        finish_batch_page(&mut view, &harness, &mut app);
        app.composer.replace("keep large-output draft");
        let generation = view.generation;
        let summary = harness
            .start(&app.session.id, "large response".into())
            .await?
            .wait()
            .await?;
        assert_eq!(summary.status, RunStatus::Completed, "{:?}", summary.error);
        for _ in 0..VIEW_EVENTS {
            let event = timeout(Duration::from_secs(5), receive(&mut view.events)).await??;
            view.event(event, &harness, &mut app);
            if view.generation != generation {
                break;
            }
        }
        assert_ne!(view.generation, generation);
        assert!(view.events.is_none());
        assert!(
            app.blocks
                .iter()
                .all(|block| block.text.len() <= VIEW_BYTES)
        );
        assert_eq!(app.composer.text, "keep large-output draft");
        let loaded = next_reply(&mut view).await?;
        let ViewReply::Tail { view: snapshot, .. } = &loaded else {
            anyhow::bail!("large output did not request a bounded replacement");
        };
        assert!(snapshot.omitted);
        let selected_cursor = snapshot.selected_cursor;
        view.loaded(loaded, &harness, &mut app);
        finish_batch_page(&mut view, &harness, &mut app);
        assert_eq!(app.last_sequence, selected_cursor);
        assert_eq!(app.composer.text, "keep large-output draft");
        assert_eq!(app.run_state, RunState::Idle);
        harness.shutdown().await?;
        Ok(())
    }
}
