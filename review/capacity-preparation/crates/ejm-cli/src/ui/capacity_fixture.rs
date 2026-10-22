//! Optional real-terminal acceptance using the public offline harness workload.
use std::{
    collections::{HashSet, VecDeque},
    path::PathBuf,
    sync::{
        Arc, Mutex, MutexGuard, OnceLock, PoisonError,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

use anyhow::{Context, Result, ensure};
use clap::Parser;
use ejm_harness::{
    CapacityContext, CapacityMode, CapacityPolicy, CapacitySnapshot, EventKind, HarnessConfig,
    RunHandle, RunStatus, SessionOptions,
};
use ejm_harness_bench::{LocalCheck, Workload};
use ejm_harness_exec::ResourceLimits;
use futures_util::{StreamExt, stream::FuturesUnordered};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::{broadcast::error::RecvError, watch};

use super::*;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    root: PathBuf,
    helper: PathBuf,
    capacity: Option<CapacityMode>,
    model_requests: Option<usize>,
    jobs: Option<usize>,
    capacity_root: Option<PathBuf>,
    agents: usize,
    ui: bool,
    trace: bool,
    seconds: u64,
    journal_bytes: u64,
}

impl Config {
    fn capacity_policy(&self) -> CapacityPolicy {
        let resources = ResourceLimits::default();
        CapacityPolicy {
            mode: self.capacity.unwrap_or(CapacityMode::Fixed),
            max_sessions: Some(self.agents),
            max_model_requests: Some(self.model_requests.unwrap_or(8)),
            max_jobs: Some(self.jobs.unwrap_or(4)),
            max_memory_bytes: Some(resources.memory_bytes),
            max_cpu_millis: Some(resources.cpu_millis),
            max_heavy_jobs: Some(resources.max_heavy_jobs),
            max_processes: Some(resources.max_processes),
            max_output_bytes: Some(resources.max_output_bytes),
        }
    }

    fn capacity_root(&self) -> PathBuf {
        self.capacity_root
            .clone()
            .unwrap_or_else(|| self.root.join("capacity"))
    }
}

#[test]
fn capacity_configuration_preserves_defaults_and_explicit_shared_policy() -> Result<()> {
    let mut value = json!({
        "root": "/fixture/one", "helper": "/fixture/ejm-cli", "agents": 100,
        "ui": true, "trace": false, "seconds": 30, "journal_bytes": 2_147_483_648_u64,
    });
    let defaults: Config = serde_json::from_value(value.clone())?;
    let policy = defaults.capacity_policy();
    assert_eq!(policy.mode, CapacityMode::Fixed);
    assert_eq!(policy.max_model_requests, Some(8));
    assert_eq!(policy.max_jobs, Some(4));
    assert_eq!(defaults.capacity_root(), defaults.root.join("capacity"));

    value["capacity"] = json!("auto");
    value["model_requests"] = json!(16);
    value["jobs"] = json!(2);
    value["capacity_root"] = json!("/fixture/shared");
    let shared: Config = serde_json::from_value(value)?;
    let policy = shared.capacity_policy();
    assert_eq!(policy.mode, CapacityMode::Auto);
    assert_eq!(policy.max_sessions, Some(100));
    assert_eq!(policy.max_model_requests, Some(16));
    assert_eq!(policy.max_jobs, Some(2));
    assert_eq!(shared.capacity_root(), PathBuf::from("/fixture/shared"));
    assert_ne!(shared.capacity_root(), shared.root);
    Ok(())
}

fn resolved_capacity_limits(config: &HarnessConfig) -> serde_json::Value {
    json!({
        "max_sessions": config.max_sessions,
        "max_model_requests": config.max_model_requests,
        "resources": config.resources,
    })
}

#[test]
fn fixed_and_automatic_fixture_hard_limits_match() -> Result<()> {
    let roots = tempfile::tempdir()?;
    let root = roots.path().canonicalize()?;
    let mut value = json!({
        "root": root, "helper": root.join("ejm-cli"), "agents": 100,
        "ui": true, "trace": false, "seconds": 30, "journal_bytes": 2_147_483_648_u64,
    });
    for jobs in [1, 8] {
        value["jobs"] = json!(jobs);
        value["model_requests"] = json!(16);
        let mut resolved = Vec::new();
        for mode in [CapacityMode::Fixed, CapacityMode::Auto] {
            value["capacity"] = json!(mode);
            let config: Config = serde_json::from_value(value.clone())?;
            let mut harness_config = HarnessConfig::default();
            let capacity = CapacityContext::prepare_isolated(
                &mut harness_config,
                config.capacity_policy(),
                &root.join(format!("{mode:?}-{jobs}")),
            )?;
            assert_eq!(capacity.snapshot().mode, mode);
            assert_eq!(harness_config.max_sessions, config.agents);
            assert_eq!(harness_config.max_model_requests, 16);
            assert_eq!(harness_config.resources.max_jobs, jobs);
            resolved.push(resolved_capacity_limits(&harness_config));
        }
        assert_eq!(resolved[0], resolved[1]);
    }
    Ok(())
}

struct PendingPaint {
    session: String,
    generation: u64,
    sequence: u64,
    marker: String,
    received: Instant,
    committed: Option<Instant>,
}

const TRACE_DIAGNOSTIC_LIMIT: usize = 8;

#[derive(Serialize)]
struct PaintTraceLagSample {
    elapsed_ns: u128,
    selected_generation: Option<u64>,
    reported_lag: u64,
}

#[derive(Serialize)]
struct PaintTraceSavedMarkerSample {
    elapsed_ns: u128,
    selected_generation: Option<u64>,
    sequence: u64,
}

#[derive(Default, Serialize)]
struct PaintTraceIntegrity {
    enabled: bool,
    subscriptions: u64,
    selection_changes: u64,
    generation_resets: u64,
    uncovered_saved_markers: u64,
    lag_signals: u64,
    // Sum of public lag reports, not an exact lost-event count.
    reported_lag_total: u64,
    closed_signals: u64,
    observer_error_count: u64,
    observer_error_samples: Vec<String>,
    missing_commit_stamps: u64,
    // First bounded observations, timed from PaintTrace creation.
    lag_samples: Vec<PaintTraceLagSample>,
    saved_marker_samples: Vec<PaintTraceSavedMarkerSample>,
}

impl PaintTraceIntegrity {
    fn error(&mut self, phase: &str, error: &str) {
        self.observer_error_count += 1;
        if self.observer_error_samples.len() < 8 {
            self.observer_error_samples
                .push(format!("{phase}: {error}").chars().take(512).collect());
        }
    }
}

#[derive(Clone)]
pub(crate) struct PaintTrace(Arc<Mutex<PaintMetrics>>);

impl PaintTrace {
    fn new() -> Self {
        Self(Arc::new(Mutex::new(PaintMetrics {
            created: Instant::now(),
            selected: None,
            pending: VecDeque::new(),
            samples: Vec::new(),
            commit_samples: Vec::new(),
            omitted: 0,
            integrity: PaintTraceIntegrity {
                enabled: true,
                ..PaintTraceIntegrity::default()
            },
        })))
    }

    fn metrics(&self) -> MutexGuard<'_, PaintMetrics> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub(crate) fn select(&self, session: &str, generation: u64) {
        let mut metrics = self.metrics();
        if let Some((selected, current)) = &metrics.selected {
            if selected == session && *current == generation {
                return;
            }
            if selected == session {
                metrics.integrity.generation_resets += 1;
            } else {
                metrics.integrity.selection_changes += 1;
            }
        }
        metrics.omitted += metrics.pending.len();
        metrics.pending.clear();
        metrics.selected = Some((session.into(), generation));
    }

    pub(crate) fn subscribed(&self) {
        self.metrics().integrity.subscriptions += 1;
    }

    pub(crate) fn error(&self, phase: &str, error: &str) {
        self.metrics().integrity.error(phase, error);
    }

    pub(crate) fn receive_error(&self, error: &RecvError) {
        let mut metrics = self.metrics();
        match error {
            RecvError::Lagged(reported) => {
                metrics.integrity.lag_signals += 1;
                metrics.integrity.reported_lag_total = metrics
                    .integrity
                    .reported_lag_total
                    .saturating_add(*reported);
                if metrics.integrity.lag_samples.len() < TRACE_DIAGNOSTIC_LIMIT {
                    let sample = PaintTraceLagSample {
                        elapsed_ns: metrics.created.elapsed().as_nanos(),
                        selected_generation: metrics.selected.as_ref().map(|(_, current)| *current),
                        reported_lag: *reported,
                    };
                    metrics.integrity.lag_samples.push(sample);
                }
            }
            RecvError::Closed => metrics.integrity.closed_signals += 1,
        }
    }

    pub(crate) fn saved(&self, event: &ejm_harness::Event) {
        if marker(event).is_some() {
            let mut metrics = self.metrics();
            metrics.integrity.uncovered_saved_markers += 1;
            if metrics.integrity.saved_marker_samples.len() < TRACE_DIAGNOSTIC_LIMIT {
                let sample = PaintTraceSavedMarkerSample {
                    elapsed_ns: metrics.created.elapsed().as_nanos(),
                    selected_generation: metrics.selected.as_ref().map(|(_, current)| *current),
                    sequence: event.sequence,
                };
                metrics.integrity.saved_marker_samples.push(sample);
            }
        }
    }

    pub(crate) fn omit(&self, event: &ejm_harness::Event) {
        if marker(event).is_some() {
            self.metrics().omitted += 1;
        }
    }

    pub(crate) fn accepted(
        &self,
        event: &ejm_harness::Event,
        generation: u64,
        origin: Option<(u64, Instant, Option<Instant>)>,
    ) {
        let Some(marker) = marker(event) else {
            return;
        };
        let mut metrics = self.metrics();
        let Some((received_generation, received, committed)) = origin else {
            metrics.integrity.missing_commit_stamps += 1;
            metrics.omitted += 1;
            return;
        };
        if received_generation != generation
            || !metrics.selected.as_ref().is_some_and(|(session, current)| {
                session == &event.session_id && *current == generation
            })
        {
            metrics.omitted += 1;
            return;
        }
        if metrics.pending.iter().any(|pending| {
            pending.session == event.session_id
                && pending.generation == generation
                && pending.sequence == event.sequence
                && pending.marker == marker
        }) {
            return;
        }
        if committed.is_none() {
            metrics.integrity.missing_commit_stamps += 1;
        }
        if metrics.pending.len() == 32 {
            metrics.pending.pop_front();
            metrics.omitted += 1;
        }
        metrics.pending.push_back(PendingPaint {
            session: event.session_id.clone(),
            generation,
            sequence: event.sequence,
            marker: marker.into(),
            received,
            committed,
        });
    }

    fn painted(&self, app: &App, buffer: &ratatui::buffer::Buffer, painted: Instant) {
        let mut metrics = self.metrics();
        if metrics.pending.is_empty() {
            return;
        }
        let screen = buffer
            .content()
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect::<String>();
        let mut index = 0;
        while index < metrics.pending.len() {
            let pending = &metrics.pending[index];
            if pending.session == app.session.id
                && metrics
                    .selected
                    .as_ref()
                    .is_some_and(|(session, generation)| {
                        session == &pending.session && *generation == pending.generation
                    })
                && screen.contains(&pending.marker)
            {
                let elapsed = painted.duration_since(pending.received).as_nanos();
                let commit_elapsed = pending
                    .committed
                    .map(|committed| painted.duration_since(committed).as_nanos());
                metrics.pending.remove(index);
                if metrics.samples.len() < 1000 {
                    metrics.samples.push(elapsed);
                } else {
                    metrics.omitted += 1;
                }
                if let Some(elapsed) = commit_elapsed
                    && metrics.commit_samples.len() < 1000
                {
                    metrics.commit_samples.push(elapsed);
                }
            } else {
                index += 1;
            }
        }
    }
}

struct PaintMetrics {
    created: Instant,
    selected: Option<(String, u64)>,
    pending: VecDeque<PendingPaint>,
    samples: Vec<u128>,
    commit_samples: Vec<u128>,
    omitted: usize,
    integrity: PaintTraceIntegrity,
}

fn marker(event: &ejm_harness::Event) -> Option<&str> {
    let EventKind::TextDelta { text } = &event.kind else {
        return None;
    };
    text.split_whitespace()
        .next()
        .filter(|marker| marker.starts_with("response:fixture-cycle-"))
}

static PAINT: OnceLock<PaintTrace> = OnceLock::new();
static STOPPED: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

pub(crate) fn paint_trace() -> Option<PaintTrace> {
    PAINT.get().cloned()
}

fn paint_metrics() -> Option<MutexGuard<'static, PaintMetrics>> {
    PAINT.get().map(PaintTrace::metrics)
}

fn stopped_sessions() -> Option<MutexGuard<'static, HashSet<String>>> {
    STOPPED
        .get()
        .map(|stopped| stopped.lock().unwrap_or_else(PoisonError::into_inner))
}

pub(super) fn stopping(session: &str) {
    if let Some(mut stopped) = stopped_sessions() {
        stopped.insert(session.into());
    }
}

fn was_stopped(session: &str) -> bool {
    stopped_sessions().is_some_and(|stopped| stopped.contains(session))
}

pub(super) fn paint_clock() -> Option<Instant> {
    PAINT.get().map(|_| Instant::now())
}

pub(super) fn painted(app: &App, buffer: &ratatui::buffer::Buffer, painted: Option<Instant>) {
    if let Some(trace) = PAINT.get()
        && let Some(painted) = painted
    {
        trace.painted(app, buffer, painted);
    }
}

async fn trace_harness() -> Result<(tempfile::TempDir, LocalBackend, App)> {
    let root = tempfile::tempdir()?;
    let harness = Harness::with_optional_provider(
        HarnessConfig {
            state_dir: root.path().join("state"),
            ..HarnessConfig::default()
        },
        None,
        vec![],
        vec![],
    )?;
    let session = harness
        .create_session(SessionOptions {
            title: "trace".into(),
            cwd: root.path().into(),
        })
        .await?;
    Ok((root, LocalBackend::new(harness), App::new(session)))
}

async fn finish_trace_view(
    view: &mut crate::views::SelectedView,
    harness: &LocalBackend,
    app: &mut App,
) -> Result<()> {
    while view.events.is_none() {
        if !view.jobs.is_empty() {
            let reply = view.jobs.join_next().await;
            view.joined(reply, harness, app);
        }
        view.apply_batch(harness, app);
    }
    Ok(())
}

async fn trace_turn(harness: &LocalBackend, app: &App, prompt: &str, words: usize) -> Result<u64> {
    let (_ready, gate) = watch::channel(true);
    harness
        .configure_provider(
            Some(Arc::new(Workload::new(gate, 0, words)?.with_markers(true))),
            "trace",
        )
        .await?;
    ensure!(
        harness
            .start(&app.session.id, prompt.into())
            .await?
            .wait()
            .await?
            .status
            == RunStatus::Completed,
        "trace run failed"
    );
    Ok(harness
        .session_view(&app.session.id, ejm_harness::SessionTailOptions::default())
        .await?
        .selected_cursor)
}

async fn trace_through(
    view: &mut crate::views::SelectedView,
    harness: &LocalBackend,
    app: &mut App,
    through: u64,
) -> Result<()> {
    while app.last_sequence < through {
        let delivery = crate::views::receive(&mut view.events).await?;
        view.event(delivery, harness, app);
    }
    Ok(())
}

fn paint_trace_frame(
    trace: &PaintTrace,
    app: &mut App,
    snapshot: &ejm_harness::HarnessSnapshot,
) -> Result<()> {
    let args = Args::parse_from(["ejm-cli", "--plain", "--no-color"]);
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(120, 30))?;
    let frame = terminal.draw(|frame| {
        render::draw(frame, app, &args, snapshot);
    })?;
    let painted = Instant::now();
    trace.painted(app, frame.buffer, painted);
    Ok(())
}

async fn trace_live_paint(
    view: &mut crate::views::SelectedView,
    harness: &LocalBackend,
    app: &mut App,
    trace: &PaintTrace,
    prompt: &str,
) -> Result<()> {
    let through = trace_turn(harness, app, prompt, 0).await?;
    trace_through(view, harness, app, through).await?;
    {
        let metrics = trace.metrics();
        let pending = metrics
            .pending
            .front()
            .context("later live marker missing")?;
        ensure!(
            pending.sequence <= app.last_sequence && pending.session == app.session.id,
            "marker lost its accepted event identity"
        );
        ensure!(
            pending
                .committed
                .is_some_and(|committed| committed <= pending.received),
            "public commit stamp missing"
        );
    }
    paint_trace_frame(trace, app, &harness.status().await)?;
    let metrics = trace.metrics();
    ensure!(
        metrics.samples.len() == 1 && metrics.commit_samples.len() == 1,
        "later live marker not painted once"
    );
    ensure!(
        metrics.commit_samples[0] >= metrics.samples[0],
        "clock origins reversed"
    );
    ensure!(
        metrics.integrity.missing_commit_stamps == 0,
        "live commit stamp missing"
    );
    Ok(())
}

#[tokio::test]
async fn paint_trace_actual_lag_recovery_retires_pending_before_later_live_paint() -> Result<()> {
    let (_root, backend, mut app) = trace_harness().await?;
    let mut backend = Some(backend);
    let trace = PaintTrace::new();
    let result = tokio::time::timeout(Duration::from_secs(15), async {
        let harness = backend.as_ref().context("trace backend missing")?;
        let mut view = crate::views::SelectedView::with_trace(Some(trace.clone()));
        view.follow(harness, &app);
        finish_trace_view(&mut view, harness, &mut app).await?;
        let through = trace_turn(harness, &app, "fixture-cycle-pending", 0).await?;
        trace_through(&mut view, harness, &mut app, through).await?;
        ensure!(trace.metrics().pending.len() == 1, "live marker missing");

        trace_turn(harness, &app, "fixture-cycle-lag", 128).await?;
        ensure!(
            matches!(
                crate::views::receive(&mut view.events).await,
                Err(RecvError::Lagged(_))
            ),
            "fixture did not lag the actual selected receiver"
        );
        view.recover(harness, &app);
        {
            let metrics = trace.metrics();
            ensure!(
                metrics.pending.is_empty() && metrics.omitted == 1,
                "stale marker retained"
            );
            ensure!(
                metrics.integrity.generation_resets == 1,
                "recovery generation missing"
            );
        }
        finish_trace_view(&mut view, harness, &mut app).await?;
        paint_trace_frame(&trace, &mut app, &harness.status().await)?;
        {
            let metrics = trace.metrics();
            ensure!(
                metrics.samples.is_empty(),
                "saved output timed as a live marker"
            );
            ensure!(
                metrics.integrity.uncovered_saved_markers > 0,
                "saved coverage missing"
            );
            ensure!(
                metrics.integrity.lag_signals == 1 && metrics.integrity.reported_lag_total > 0,
                "actual lag evidence missing"
            );
        }

        trace_live_paint(
            &mut view,
            harness,
            &mut app,
            &trace,
            "fixture-cycle-after-lag",
        )
        .await?;
        ensure!(
            trace.metrics().integrity.subscriptions == 2,
            "extra trace subscription created"
        );
        harness.shutdown().await?;
        drop(backend.take());
        ensure!(
            matches!(
                crate::views::receive(&mut view.events).await,
                Err(RecvError::Closed)
            ),
            "actual selected receiver did not close"
        );
        ensure!(
            trace.metrics().integrity.closed_signals == 1,
            "typed closure missing"
        );
        Ok::<(), anyhow::Error>(())
    })
    .await;
    let cleanup = match backend.take() {
        Some(backend) => tokio::time::timeout(Duration::from_secs(5), backend.shutdown()).await,
        None => Ok(Ok(())),
    };
    result.context("selected trace recovery timed out")??;
    cleanup.context("selected trace cleanup timed out")??;
    Ok(())
}

#[tokio::test]
async fn paint_trace_snapshot_duplicate_stays_uncovered_then_later_live_event_is_timed()
-> Result<()> {
    let (_root, harness, mut app) = trace_harness().await?;
    let trace = PaintTrace::new();
    let result = tokio::time::timeout(Duration::from_secs(15), async {
        let mut view = crate::views::SelectedView::with_trace(Some(trace.clone()));
        view.follow(&harness, &app);
        let reply = view
            .jobs
            .join_next()
            .await
            .context("initial view task missing")??;
        let crate::views::ViewReply::Tail {
            generation, stream, ..
        } = reply
        else {
            anyhow::bail!("expected initial tail");
        };
        trace_turn(&harness, &app, "fixture-cycle-saved", 0).await?;
        // Capture a later snapshot while keeping the already registered stream.
        let snapshot = harness
            .session_view(&app.session.id, ejm_harness::SessionTailOptions::default())
            .await?;
        let buffered = snapshot.events.len();
        view.loaded(
            crate::views::ViewReply::Tail {
                generation,
                view: snapshot,
                stream,
            },
            &harness,
            &mut app,
        );
        finish_trace_view(&mut view, &harness, &mut app).await?;
        for _ in 0..buffered {
            let delivery = crate::views::receive(&mut view.events).await?;
            view.event(delivery, &harness, &mut app);
        }
        paint_trace_frame(&trace, &mut app, &harness.status().await)?;
        {
            let metrics = trace.metrics();
            ensure!(
                metrics.integrity.uncovered_saved_markers == 1,
                "saved marker not uncovered"
            );
            ensure!(
                metrics.pending.is_empty() && metrics.samples.is_empty(),
                "buffered duplicate timed as a new live event"
            );
        }
        trace_live_paint(&mut view, &harness, &mut app, &trace, "fixture-cycle-later").await?;
        let metrics = trace.metrics();
        ensure!(
            metrics.integrity.subscriptions == 1 && metrics.integrity.observer_error_count == 0,
            "trace added a receiver or failed"
        );
        Ok::<(), anyhow::Error>(())
    })
    .await;
    let cleanup = tokio::time::timeout(Duration::from_secs(5), harness.shutdown()).await;
    result.context("snapshot trace test timed out")??;
    cleanup.context("snapshot trace cleanup timed out")??;
    Ok(())
}

#[test]
fn paint_trace_bounds_selected_view_failures_without_losing_the_count() {
    let mut integrity = PaintTraceIntegrity::default();
    let error = "λ".repeat(600);
    for phase in ["subscription", "view load", "view task"] {
        integrity.error(phase, &error);
    }
    for _ in 0..9 {
        integrity.error("view task", &error);
    }
    assert_eq!(integrity.observer_error_count, 12);
    assert_eq!(integrity.observer_error_samples.len(), 8);
    assert!(
        integrity
            .observer_error_samples
            .iter()
            .all(|sample| sample.chars().count() == 512)
    );
}

async fn completion(
    mut run: RunHandle,
    submitted: Option<Instant>,
) -> (
    String,
    Result<ejm_harness::RunSummary>,
    Option<Instant>,
    Instant,
) {
    let result = run.wait().await;
    (run.session_id, result, submitted, Instant::now())
}

#[derive(Serialize)]
struct CapacityObservation {
    elapsed_ns: u128,
    snapshot: Option<CapacitySnapshot>,
    journal: ejm_harness::JournalSnapshot,
}

impl CapacityObservation {
    fn capture(harness: &Harness, start: Instant) -> Self {
        Self {
            elapsed_ns: start.elapsed().as_nanos(),
            snapshot: harness.capacity_snapshot(),
            journal: harness.journal_status(),
        }
    }
}

#[derive(Default)]
struct CapacityObservations {
    samples: Vec<CapacityObservation>,
    next: Duration,
    omitted: usize,
}

impl CapacityObservations {
    const INTERVAL: Duration = Duration::from_millis(500);
    const LIMIT: usize = 400;

    fn sample(&mut self, harness: &Harness, start: Instant) {
        let elapsed = start.elapsed();
        if elapsed < self.next {
            return;
        }
        // Resume with the current snapshot, never a burst of stale catch-up samples.
        self.next = elapsed.saturating_add(Self::INTERVAL);
        if self.samples.len() < Self::LIMIT {
            self.samples
                .push(CapacityObservation::capture(harness, start));
        } else {
            self.omitted += 1;
        }
    }
}

fn record_failure(
    samples: &mut Vec<serde_json::Value>,
    session: &str,
    phase: &str,
    result: &Result<ejm_harness::RunSummary>,
) {
    if samples.len() >= 8 {
        return;
    }
    let bounded = |text: &str| text.chars().take(512).collect::<String>();
    samples.push(match result {
        Ok(summary) => json!({
            "session_id": session,
            "phase": phase,
            "kind": "run_summary",
            "run_id": summary.run_id,
            "status": summary.status,
            "error": summary.error.as_deref().map(bounded),
        }),
        Err(error) => json!({
            "session_id": session,
            "phase": phase,
            "kind": "completion_error",
            "error": bounded(&format!("{error:#}")),
        }),
    });
}

#[test]
fn failure_diagnostics_keep_both_result_kinds_with_bounded_text_and_count() {
    let mut samples = Vec::new();
    let summary = Ok(ejm_harness::RunSummary {
        run_id: "failed-run".into(),
        status: RunStatus::Failed,
        steps: 1,
        text: String::new(),
        usage: ejm_harness::Usage::default(),
        error: Some("λ".repeat(600)),
    });
    record_failure(&mut samples, "session", "active", &summary);
    let completion_error = Err(anyhow::anyhow!("run driver stopped without a summary"));
    for _ in 0..12 {
        record_failure(&mut samples, "session", "drain", &completion_error);
    }
    assert_eq!(samples.len(), 8);
    assert_eq!(samples[0]["kind"], "run_summary");
    assert_eq!(
        samples[0]["error"]
            .as_str()
            .map(|text| text.chars().count()),
        Some(512)
    );
    assert_eq!(samples[1]["kind"], "completion_error");
    assert_eq!(samples[1]["error"], "run driver stopped without a summary");
}

async fn drive(
    harness: Harness,
    handles: Vec<RunHandle>,
    gate: watch::Sender<bool>,
    root: PathBuf,
    seconds: u64,
    stop: CancellationToken,
) -> Result<serde_json::Value> {
    let mut waiting = FuturesUnordered::new();
    for handle in handles {
        waiting.push(completion(handle, None));
    }
    let mut poll = interval(Duration::from_millis(10));
    let mut opened = None;
    let mut completed = 0_u64;
    let mut cancelled = 0_u64;
    let mut failed = 0_u64;
    let mut failure_samples = Vec::new();
    let mut latencies = Vec::new();
    let mut initial_latencies = Vec::new();
    let mut capacity_observations = CapacityObservations::default();
    loop {
        tokio::select! {
            () = stop.cancelled() => break,
            _ = poll.tick() => {
                if opened.is_none() && root.join("start").try_exists()? {
                    opened = Some(Instant::now());
                    gate.send(true)?;
                }
                if let Some(start) = opened {
                    capacity_observations.sample(&harness, start);
                }
                if root.join("stop").try_exists()?
                    || opened.is_some_and(|start| start.elapsed() >= Duration::from_secs(seconds))
                { break; }
            }
            Some((id, result, submitted, finished)) = waiting.next() => {
                match result {
                    Ok(summary) if summary.status == RunStatus::Completed => {
                        completed += 1;
                        if let Some(submitted) = submitted {
                            if latencies.len() < 8192 {
                                latencies.push(finished.duration_since(submitted).as_nanos());
                            }
                        } else if let Some(opened) = opened {
                            initial_latencies.push(finished.duration_since(opened).as_nanos());
                        }
                        if !was_stopped(&id) {
                            let marker = format!("fixture-cycle-{completed:08}");
                            let submitted = Instant::now();
                            let handle = harness.start(&id, marker).await?;
                            // Stop can arrive while start awaits its ordering point.
                            if was_stopped(&id) { harness.cancel(&id).await?; }
                            waiting.push(completion(handle, Some(submitted)));
                        }
                    }
                    Ok(summary) if summary.status == RunStatus::Cancelled => cancelled += 1,
                    result => {
                        failed += 1;
                        record_failure(&mut failure_samples, &id, "active", &result);
                    }
                }
            }
        }
    }
    let elapsed = opened.map_or(0, |start| start.elapsed().as_nanos());
    let capacity_at_cutoff = opened.map(|start| CapacityObservation::capture(&harness, start));
    let cutoff_pending = waiting.len();
    let draining = Instant::now();
    stop.cancel();
    harness.request_shutdown();
    harness.shutdown().await?;
    let (mut drain_completed, mut drain_cancelled, mut drain_failed) = (0_u64, 0_u64, 0_u64);
    while let Some((id, result, _, _)) = waiting.next().await {
        match result {
            Ok(summary) if summary.status == RunStatus::Completed => drain_completed += 1,
            Ok(summary) if summary.status == RunStatus::Cancelled => drain_cancelled += 1,
            result => {
                drain_failed += 1;
                record_failure(&mut failure_samples, &id, "drain", &result);
            }
        }
    }
    let capacity_after_shutdown = opened.map(|start| CapacityObservation::capture(&harness, start));
    let failure_samples_omitted = failed + drain_failed - u64::try_from(failure_samples.len())?;
    Ok(
        json!({"completed":completed,"cancelled":cancelled,"failed":failed,
              "elapsed_ns":elapsed,"completion_ns":latencies,"initial_completion_ns":initial_latencies,
              "completion_samples_limit":8192,"cutoff_pending":cutoff_pending,
              "drain_completed":drain_completed,"drain_cancelled":drain_cancelled,"drain_failed":drain_failed,
              "drain_elapsed_ns":draining.elapsed().as_nanos(),
              "failure_samples":failure_samples,"failure_samples_omitted":failure_samples_omitted,
              "failure_samples_limit":8,"failure_text_max_chars":512,
              "capacity_observations":capacity_observations.samples,
              "capacity_observation_interval_ms":CapacityObservations::INTERVAL.as_millis(),
              "capacity_observations_limit":CapacityObservations::LIMIT,
              "capacity_observations_omitted":capacity_observations.omitted,
              "capacity_at_cutoff":capacity_at_cutoff,"capacity_after_shutdown":capacity_after_shutdown,
              "operator_stopped_sessions":stopped_sessions().map_or(0, |stopped| stopped.len())}),
    )
}

struct Prepared {
    harness: Harness,
    executor: Executor,
    handles: Vec<RunHandle>,
    gate: watch::Sender<bool>,
    completed_tools: Arc<AtomicUsize>,
    first: String,
    resolved_capacity_limits: serde_json::Value,
    capacity_initial: CapacitySnapshot,
}

async fn prepare(config: &Config) -> Result<Prepared> {
    ensure!(
        STOPPED.set(Mutex::new(HashSet::new())).is_ok(),
        "fixture Stop tracking already started"
    );
    std::fs::create_dir_all(&config.root)?;
    let mut harness_config = HarnessConfig {
        state_dir: config.root.join("state"),
        max_sessions: config.agents,
        max_store_bytes: config.journal_bytes,
        max_input_tokens: 100_000_000,
        max_output_tokens: 100_000_000,
        ..HarnessConfig::default()
    };
    let capacity = CapacityContext::prepare_isolated_with_native_helper(
        &mut harness_config,
        config.capacity_policy(),
        &config.capacity_root(),
        &config.helper,
    )?;
    let capacity_initial = capacity.snapshot();
    let resolved_capacity_limits = resolved_capacity_limits(&harness_config);
    let completed_tools = Arc::new(AtomicUsize::new(0));
    let tool = LocalCheck::new(config.root.join("artifacts"), completed_tools.clone())
        .with_native_owner(capacity.native_owner());
    let executor = Executor::new(capacity.governor(), config.root.join("artifacts"))?
        .with_native_owner(capacity.native_owner());
    let (gate, ready) = watch::channel(false);
    let harness = Harness::with_capacity(
        harness_config,
        Some(Arc::new(Workload::new(ready, 1, 128)?.with_markers(true))),
        vec![Arc::new(tool)],
        vec![],
        capacity,
    )?;
    let mut sessions = Vec::with_capacity(config.agents);
    let mut handles = Vec::with_capacity(config.agents);
    for index in 0..config.agents {
        let session = harness
            .create_session(SessionOptions {
                title: format!("agent-{index:04}"),
                cwd: config.root.clone(),
            })
            .await?;
        handles.push(
            harness
                .start(&session.id, format!("fixture-cycle-initial-{index:04}"))
                .await?,
        );
        sessions.push(session);
    }
    let first = sessions.first().context("empty fixture")?.id.clone();
    let ready_path = config.root.join("ready.tmp");
    std::fs::write(
        &ready_path,
        serde_json::to_vec(&json!({"sessions":sessions,"first_session_id":first}))?,
    )?;
    std::fs::rename(ready_path, config.root.join("ready.json"))?;
    Ok(Prepared {
        harness,
        executor,
        handles,
        gate,
        completed_tools,
        first,
        resolved_capacity_limits,
        capacity_initial,
    })
}

fn journal_file_bytes(config: &Config, name: &str) -> Option<u64> {
    std::fs::metadata(config.root.join("state").join(name))
        .ok()
        .map(|metadata| metadata.len())
}

fn read_config() -> Result<Config> {
    let path = PathBuf::from(
        std::env::var_os("EJM_CAPACITY_TERMINAL_CONFIG").context("fixture config missing")?,
    );
    ensure!(
        std::fs::metadata(&path)?.len() <= 8192,
        "fixture config exceeds bound"
    );
    let config: Config = serde_json::from_slice(&std::fs::read(path)?)?;
    ensure!(
        (100..=4096).contains(&config.agents) && (1..=180).contains(&config.seconds),
        "fixture exceeds envelope"
    );
    ensure!(
        (512 * 1024 * 1024..=2 * 1024 * 1024 * 1024).contains(&config.journal_bytes),
        "fixture journal must be between 512 MiB and 2 GiB"
    );
    ensure!(
        config.root.is_absolute()
            && config.helper.is_absolute()
            && config
                .capacity_root
                .as_ref()
                .is_none_or(|root| root.is_absolute()),
        "fixture paths must be absolute"
    );
    Ok(config)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "offline live-agent PTY and headless acceptance driven by check_terminal.py"]
async fn capacity_terminal_fixture() -> Result<()> {
    let config = read_config()?;
    let Prepared {
        harness,
        executor,
        handles,
        gate,
        completed_tools,
        first,
        resolved_capacity_limits,
        capacity_initial,
    } = prepare(&config).await?;
    let stop = CancellationToken::new();
    if config.trace {
        ensure!(
            PAINT.set(PaintTrace::new()).is_ok(),
            "fixture trace already started"
        );
    }
    let mut args = Args::parse_from(["ejm-cli", "--plain", "--no-color"]);
    args.state_dir = Some(config.root.join("state"));
    args.session = Some(first);
    let extensions = Extensions::empty()?;
    let mut controller = tokio::spawn(drive(
        harness.clone(),
        handles,
        gate,
        config.root.clone(),
        config.seconds,
        stop.clone(),
    ));
    let ui = async {
        if !config.ui {
            return std::future::pending::<Result<()>>().await;
        }
        let backend = LocalBackend::new(harness.clone());
        let remote = HostUi::prepare(&args, CancellationToken::new()).await?;
        let mut app = initial_app(&backend, &args, &extensions).await?;
        app.connection_ready = true;
        app.status = "Offline capacity fixture".into();
        run_prepared(
            backend,
            &mut args,
            &extensions,
            remote,
            app,
            ConnectionTasks::new(executor),
            termination_signal(),
        )
        .await
    };
    tokio::pin!(ui);
    let result = tokio::select! {
        result = &mut controller => result,
        result = &mut ui => {
            stop.cancel();
            let controller_result = controller.await;
            result?;
            controller_result
        },
    };
    stop.cancel();
    let mut report = result.context("fixture workload task failed")??;
    report["capacity_policy"] = json!(config.capacity_policy());
    report["capacity_initial"] = json!(capacity_initial);
    report["resolved_capacity_limits"] = resolved_capacity_limits;
    report["capacity_root"] = json!(config.capacity_root());
    report["journal_limit_bytes"] = json!(config.journal_bytes);
    report["journal_main_bytes"] = json!(journal_file_bytes(&config, "sessions.sqlite3"));
    report["journal_wal_bytes"] = json!(journal_file_bytes(&config, "sessions.sqlite3-wal"));
    report["completed_tools"] = json!(completed_tools.load(Ordering::Relaxed));
    report["selected_event_receive_to_terminal_write_ns"] =
        paint_metrics().map_or(json!([]), |metrics| json!(metrics.samples));
    report["selected_event_commit_to_terminal_write_ns"] =
        paint_metrics().map_or(json!([]), |metrics| json!(metrics.commit_samples));
    report["paint_trace_omitted"] = paint_metrics().map_or(json!(0), |metrics| {
        json!(metrics.omitted + metrics.pending.len())
    });
    report["paint_trace_integrity"] = paint_metrics()
        .map_or(json!(PaintTraceIntegrity::default()), |metrics| {
            json!(metrics.integrity)
        });
    std::fs::write(
        config.root.join("result.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    Ok(())
}
