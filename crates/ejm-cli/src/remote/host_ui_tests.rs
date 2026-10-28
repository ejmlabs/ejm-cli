//! Lifecycle regressions use held tasks and an offline public harness.
use super::*;
use anyhow::{Context, Result};
use clap::Parser;
use ejm_harness::{Harness, HarnessConfig, ReplayProvider, Session, SessionOptions};
use std::{sync::Arc, time::Duration};

const NOW: u64 = 1000;
const EXPIRES: u64 = 2000;

async fn backend() -> Result<(tempfile::TempDir, LocalBackend, Session)> {
    let directory = tempfile::tempdir()?;
    let harness = Harness::new(
        HarnessConfig {
            state_dir: directory.path().join("sessions"),
            ..HarnessConfig::default()
        },
        Arc::new(ReplayProvider::echo()),
        vec![],
    )?;
    let session = harness
        .create_session(SessionOptions {
            cwd: directory.path().into(),
            title: "host lifecycle".into(),
        })
        .await?;
    Ok((directory, LocalBackend::new(harness), session))
}

fn held_task(epoch: &str) -> (HostTask, mpsc::Sender<HostNotice>, oneshot::Sender<()>) {
    let (sender, notices) = mpsc::channel(4);
    let (release, receive) = oneshot::channel();
    let join = tokio::spawn(async move {
        let _ = receive.await;
    });
    (
        HostTask {
            epoch: epoch.into(),
            expires_at_ms: EXPIRES,
            notices,
            join,
        },
        sender,
        release,
    )
}

fn host(session: &Session, task: HostTask) -> HostUi {
    HostUi {
        account: None,
        session: Some(session.id.clone()),
        role: ShareRole::Control,
        connected: true,
        panel: Some(Panel {
            title: "Private invitation".into(),
            body: Zeroizing::new("fixture invitation".into()),
            answer: None,
        }),
        shutdown: CancellationToken::new(),
        cancel: CancellationToken::new(),
        task: Some(task),
    }
}

async fn finish(host: &mut HostUi, release: oneshot::Sender<()>) -> Result<()> {
    release
        .send(())
        .map_err(|()| anyhow::anyhow!("task already ended"))?;
    tokio::time::timeout(Duration::from_secs(3), async {
        while host
            .task
            .as_ref()
            .is_some_and(|task| !task.join.is_finished())
        {
            tokio::task::yield_now().await;
        }
    })
    .await?;
    Ok(())
}

#[tokio::test]
async fn remote_draft_does_not_take_enter_from_an_open_overlay() -> Result<()> {
    let (_directory, backend, session) = backend().await?;
    let mut app = App::new(session);
    app.composer.replace("/remote share control");
    let enter = Event::Key(crossterm::event::KeyEvent::new(
        KeyCode::Enter,
        KeyModifiers::NONE,
    ));
    assert!(HostUi::is_command(&enter, &app));
    app.overlay = Some(crate::app::Overlay::Help { selected: 0 });
    assert!(!HostUi::is_command(&enter, &app));
    app.overlay = Some(crate::app::Overlay::Sessions {
        sessions: vec![],
        selected: 0,
    });
    assert!(!HostUi::is_command(&enter, &app));
    backend.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn failure_keeps_task_until_join_and_old_notices_cannot_hide_replacement() -> Result<()> {
    let (_directory, backend, session) = backend().await?;
    backend
        .begin_share(&session.id, "old".into(), ShareRole::Control, EXPIRES)
        .await?;
    let (task, old_sender, release) = held_task("old");
    let mut host = host(&session, task);
    let mut app = App::new(session.clone());
    old_sender.send(HostNotice::Failed).await?;
    host.poll(&mut app, &backend, NOW).await;
    assert!(
        host.task.is_some(),
        "a completion notice cannot detach its task"
    );
    assert!(host.session.is_none());
    assert!(host.panel.is_none());
    assert!(app.remote.is_none());
    let args = Args::try_parse_from(["ejm-cli"])?;
    assert!(
        host.share(&backend, &args, &session.id, ShareRole::View)
            .await
            .is_err()
    );

    finish(&mut host, release).await?;
    host.poll(&mut app, &backend, NOW).await;
    assert!(host.task.is_none());
    assert!(backend.shared_session().await.is_none());

    backend
        .begin_share(&session.id, "new".into(), ShareRole::Control, EXPIRES)
        .await?;
    let (task, _new_sender, release) = held_task("new");
    host.session = Some(session.id.clone());
    host.cancel = CancellationToken::new();
    host.task = Some(task);
    // The old receiver was dropped only after its task was joined.
    assert!(old_sender.send(HostNotice::Failed).await.is_err());
    host.poll(&mut app, &backend, NOW).await;
    assert!(app.remote.is_some());
    assert!(!backend.stop_share_if("old").await);
    assert!(backend.is_current_share("new", &session.id, NOW).await);
    finish(&mut host, release).await?;
    host.poll(&mut app, &backend, NOW).await;
    backend.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn expiry_revokes_and_clears_pairing_before_slow_cleanup_finishes() -> Result<()> {
    let (_directory, backend, session) = backend().await?;
    backend
        .begin_share(&session.id, "expiring".into(), ShareRole::View, EXPIRES)
        .await?;
    let (task, sender, release) = held_task("expiring");
    let mut host = host(&session, task);
    let mut app = App::new(session.clone());
    let (answer, mut receive) = oneshot::channel();
    host.panel.as_mut().context("pairing panel missing")?.answer = Some(answer);
    host.poll(&mut app, &backend, EXPIRES).await;
    assert!(backend.shared_session().await.is_none());
    assert!(host.cancel.is_cancelled());
    assert!(
        host.task.is_some(),
        "slow cleanup still owns its task handle"
    );
    assert!(host.session.is_none());
    assert!(host.panel.is_none());
    assert!(!host.connected);
    assert!(app.remote.is_none());
    assert!(matches!(
        receive.try_recv(),
        Err(oneshot::error::TryRecvError::Closed)
    ));

    sender.send(HostNotice::Connected).await?;
    host.poll(&mut app, &backend, EXPIRES).await;
    assert!(
        app.remote.is_none(),
        "late notices cannot restore an expired share"
    );
    finish(&mut host, release).await?;
    host.poll(&mut app, &backend, EXPIRES).await;
    assert!(host.task.is_none());
    backend.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn finished_old_task_cleanup_does_not_revoke_a_replacement_epoch() -> Result<()> {
    let (_directory, backend, session) = backend().await?;
    backend
        .begin_share(&session.id, "replacement".into(), ShareRole::View, EXPIRES)
        .await?;
    let (task, _sender, release) = held_task("old");
    let mut host = host(&session, task);
    let mut app = App::new(session.clone());
    finish(&mut host, release).await?;
    host.poll(&mut app, &backend, NOW).await;
    assert!(host.task.is_none());
    assert!(
        backend
            .is_current_share("replacement", &session.id, NOW)
            .await
    );
    assert!(backend.stop_share_if("replacement").await);
    backend.shutdown().await?;
    Ok(())
}

#[derive(serde::Deserialize, zeroize::Zeroize, zeroize::ZeroizeOnDrop)]
#[serde(deny_unknown_fields)]
struct RemoteTerminalConfig {
    input: String,
}

fn terminal_config() -> Result<RemoteTerminalConfig> {
    let path = std::env::var_os("EJM_REMOTE_TERMINAL_CONFIG")
        .context("remote terminal fixture needs a private configuration file")?;
    let bytes = Zeroizing::new(std::fs::read(path)?);
    anyhow::ensure!(
        bytes.len() <= 4096,
        "invalid remote terminal fixture config"
    );
    serde_json::from_slice(&bytes).context("invalid remote terminal fixture config")
}

async fn terminal_input(terminal: &mut crate::ui::TerminalGuard) -> Result<Event> {
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if terminal.poll_input()? {
                return terminal.read_input();
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await?
}

async fn terminal_panel(terminal: &mut crate::ui::TerminalGuard, host: &mut HostUi) -> Result<()> {
    terminal.reset_input()?;
    while host.panel.is_some() {
        terminal.terminal.draw(|frame| host.draw(frame))?;
        host.handle_panel(&terminal_input(terminal).await?);
    }
    terminal.reset_input()
}

async fn terminal_stage(
    terminal: &mut crate::ui::TerminalGuard,
    title: &str,
    body: &str,
    key: char,
) -> Result<()> {
    loop {
        terminal
            .terminal
            .draw(|frame| super::super::protected::draw(frame, title, body))?;
        if matches!(terminal_input(terminal).await?, Event::Key(event) if event.kind == KeyEventKind::Press && event.code == KeyCode::Char(key))
        {
            return Ok(());
        }
    }
}

async fn terminal_share(
    backend: &LocalBackend,
    session: &Session,
) -> Result<(HostUi, mpsc::Sender<HostNotice>, oneshot::Sender<()>)> {
    let identity = EndpointIdentity::memory_only()?;
    let invitation = Invitation::new(
        super::super::identity::RelayOrigin::parse("https://relay.example")?,
        identity.public_key(),
        ShareRole::Control,
        NOW,
    )?;
    backend
        .begin_share(
            &session.id,
            invitation.routing_id().into(),
            ShareRole::Control,
            EXPIRES,
        )
        .await?;
    let (task, sender, release) = held_task(invitation.routing_id());
    let mut sharing = host(session, task);
    sharing.panel = Some(Panel {
        title: "Private pairing fixture".into(),
        body: invitation.protected_text()?,
        answer: None,
    });
    Ok((sharing, sender, release))
}

async fn terminal_host_flow(config: &RemoteTerminalConfig) -> Result<()> {
    let (_directory, backend, session) = backend().await?;
    let (mut sharing, sender, _release) = terminal_share(&backend, &session).await?;
    let mut app = App::new(session.clone());
    let args = Args::try_parse_from(["ejm-cli", "--replay"])?;
    let mut terminal = crate::ui::TerminalGuard::enter()?;
    terminal_panel(&mut terminal, &mut sharing).await?;

    let (answer, receive) = oneshot::channel();
    sender
        .send(HostNotice::Confirm {
            fingerprint: "public controller fingerprint fixture".into(),
            answer,
        })
        .await?;
    sharing.poll(&mut app, &backend, NOW).await;
    terminal_panel(&mut terminal, &mut sharing).await?;
    anyhow::ensure!(receive.await?, "local controller confirmation was refused");
    sender.send(HostNotice::Connected).await?;
    sharing.poll(&mut app, &backend, NOW).await;
    terminal_stage(&mut terminal, "Host paired", "S stops this share.", 's').await?;
    sharing
        .command("/remote stop", &backend, &args, &session.id)
        .await?;
    anyhow::ensure!(
        backend.shared_session().await.is_none() && sharing.panel.is_none(),
        "stop left remote authority or a private panel"
    );
    terminal_stage(
        &mut terminal,
        "Sharing stopped",
        "R creates a new local share.",
        'r',
    )
    .await?;

    let (mut replacement, _sender, _release) = terminal_share(&backend, &session).await?;
    terminal_panel(&mut terminal, &mut replacement).await?;
    terminal_stage(
        &mut terminal,
        "Switch shared session",
        "N selects another local session.",
        'n',
    )
    .await?;
    let next = backend
        .create_session(SessionOptions {
            cwd: session.cwd.clone(),
            title: "selected session changed".into(),
        })
        .await?;
    app.switch_session(App::new(next));
    anyhow::ensure!(
        replacement.session_changed(&app.session.id),
        "session change was not detected"
    );
    replacement.stop(&backend).await;
    anyhow::ensure!(
        backend.shared_session().await.is_none()
            && replacement.panel.is_none()
            && replacement.session.is_none(),
        "session change left sharing active"
    );
    anyhow::ensure!(
        app.composer.text.is_empty()
            && app
                .blocks
                .iter()
                .all(|block| !block.text.contains(&config.input)
                    && !block.text.contains("ejm-remote:v1:")),
        "private pairing entered the conversation"
    );
    anyhow::ensure!(
        backend.events(&session.id, 0, 512).await?.is_empty(),
        "private pairing entered durable session events"
    );
    terminal_stage(
        &mut terminal,
        "Selected session changed",
        "Q leaves the terminal fixture.",
        'q',
    )
    .await?;
    drop(terminal);
    backend.shutdown().await
}

async fn terminal_private_boundary(config: &RemoteTerminalConfig) -> Result<()> {
    const PUBLIC_INPUT: &str = "safe boundary input";
    let (_directory, backend, session) = backend().await?;
    let mut app = App::new(session.clone());
    let args = Args::try_parse_from(["ejm-cli", "--replay"])?;
    let mut terminal = crate::ui::TerminalGuard::enter()?;
    app.notice("Ordinary input ready");
    loop {
        let snapshot = backend.status().await;
        terminal
            .terminal
            .draw(|frame| crate::render::draw(frame, &mut app, &args, &snapshot))?;
        match terminal_input(&mut terminal).await? {
            Event::Paste(text) => app.composer.insert(&Zeroizing::new(text)),
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                if key.code == KeyCode::Enter {
                    anyhow::ensure!(
                        app.composer.take() == PUBLIC_INPUT,
                        "private queued input crossed into an ordinary submission"
                    );
                    break;
                }
                crate::ui::edit_composer(key, &mut app);
            }
            _ => {}
        }
        anyhow::ensure!(
            !app.composer.text.contains(&config.input),
            "private queued input crossed into the ordinary draft"
        );
    }
    app.composer.history(true);
    app.composer.history(true);
    anyhow::ensure!(
        app.composer.text == PUBLIC_INPUT
            && app
                .blocks
                .iter()
                .all(|block| !block.text.contains(&config.input)),
        "private queued input entered ordinary history"
    );
    anyhow::ensure!(
        backend.events(&session.id, 0, 512).await?.is_empty(),
        "private queued input entered durable session events"
    );
    terminal_stage(
        &mut terminal,
        "Private input boundary passed",
        "Q leaves the terminal fixture.",
        'q',
    )
    .await?;
    drop(terminal);
    backend.shutdown().await
}

// Only the ignored PTY fixture replaces OAuth transport. Production login is unchanged.
pub(super) async fn startup_login_fixture(cancel: CancellationToken) -> Result<()> {
    if std::env::var("EJM_REMOTE_TERMINAL_CASE").as_deref() == Ok("host-signal") {
        super::super::protected::login_notice(
            "Offline host startup fixture",
            true,
            "https://relay.example",
            cancel,
        )
        .await?;
        anyhow::bail!("host fixture must stop before OAuth transport");
    }
    Ok(())
}

struct ObserverGate {
    started: tokio::sync::Notify,
    start_release: tokio::sync::Notify,
    ended: mpsc::Sender<CancellationToken>,
    end_release: tokio::sync::Notify,
}

#[async_trait::async_trait]
impl ejm_harness::Hook for ObserverGate {
    async fn run_start(&self, _: &ejm_harness::HookContext, prompt: String) -> Result<String> {
        self.started.notify_one();
        self.start_release.notified().await;
        Ok(prompt)
    }
    async fn run_end(
        &self,
        context: &ejm_harness::HookContext,
        _: &ejm_harness::RunSummary,
    ) -> Result<()> {
        self.ended.send(context.cancel.clone()).await?;
        self.end_release.notified().await;
        Ok(())
    }
}

async fn queued_observer(
    backend: &LocalBackend,
    session: &Session,
    gate: &ObserverGate,
    ends: &mut mpsc::Receiver<CancellationToken>,
) -> Result<(ejm_harness::RunHandle, CancellationToken)> {
    use super::super::protocol::{
        ClientRequest, ExpectedState, Mutation, MutationAction, MutationOutcome, ServerMessage,
    };
    let target = backend.start(&session.id, "original target".into()).await?;
    gate.started.notified().await;
    let ServerMessage::Snapshot {
        through_sequence, ..
    } = backend
        .request(
            "observer",
            ClientRequest::Snapshot {
                after_sequence: 0,
                limit: 512,
            },
            NOW,
        )
        .await?
    else {
        anyhow::bail!("missing fixture snapshot");
    };
    let response = backend
        .request(
            "observer",
            ClientRequest::Mutation {
                mutation: Mutation {
                    sequence: 1,
                    expected: ExpectedState {
                        session_id: session.id.clone(),
                        run_id: Some(target.id.clone()),
                        event_sequence: through_sequence,
                    },
                    action: MutationAction::Queue {
                        run_id: target.id.clone(),
                        text: "must stay revoked".into(),
                    },
                },
            },
            NOW,
        )
        .await?;
    anyhow::ensure!(
        matches!(
            response,
            ServerMessage::Outcome {
                outcome: MutationOutcome::Queued,
                ..
            }
        ),
        "remote queue not accepted"
    );
    gate.start_release.notify_one();
    let observer = ends.recv().await.context("missing blocked observer")?;
    Ok((target, observer))
}

async fn terminal_observer_loop(signal: bool) -> Result<()> {
    let directory = tempfile::tempdir()?;
    let (ended, mut ends) = mpsc::channel(1);
    let gate = Arc::new(ObserverGate {
        started: tokio::sync::Notify::new(),
        start_release: tokio::sync::Notify::new(),
        ended,
        end_release: tokio::sync::Notify::new(),
    });
    let harness = Harness::with_hooks(
        HarnessConfig {
            state_dir: directory.path().join("sessions"),
            ..HarnessConfig::default()
        },
        Arc::new(ReplayProvider::echo()),
        vec![],
        vec![gate.clone()],
    )?;
    let session = harness
        .create_session(SessionOptions {
            cwd: directory.path().into(),
            title: "observer fixture".into(),
        })
        .await?;
    let backend = LocalBackend::new(harness);
    backend
        .begin_share(&session.id, "observer".into(), ShareRole::Control, u64::MAX)
        .await?;
    let (mut target, observer) = queued_observer(&backend, &session, &gate, &mut ends).await?;
    let shutdown = CancellationToken::new();
    let cancel = shutdown.child_token();
    let task_cancel = cancel.clone();
    let (_sender, notices) = mpsc::channel(4);
    let task = HostTask {
        epoch: "observer".into(),
        expires_at_ms: u64::MAX,
        notices,
        join: tokio::spawn(async move { task_cancel.cancelled().await }),
    };
    let mut sharing = host(&session, task);
    sharing.shutdown = shutdown.clone();
    sharing.cancel = cancel;
    sharing.panel = None;
    let mut app = App::new(session.clone());
    app.notice("Observer blocked");
    let args = Args::try_parse_from(["ejm-cli", "--replay"])?;
    let extensions = crate::extensions::Extensions::empty()?;
    let terminal = crate::ui::TerminalGuard::enter()?;
    let mut operation_completed = false;
    crate::ui::with_shutdown(&backend, shutdown.clone(), async {
        if signal {
            backend
                .with_authority_held(async {
                    eprintln!("Observer blocked");
                    shutdown.cancelled().await;
                })
                .await;
        }
        crate::ui::run_app(&backend, &args, &extensions, sharing, app, terminal).await?;
        operation_completed = true;
        Ok(())
    })
    .await?;
    anyhow::ensure!(
        !signal || operation_completed,
        "signal dropped the suspended UI operation before cleanup"
    );
    anyhow::ensure!(
        !observer.is_cancelled(),
        "local input waited for observer release or timeout"
    );
    anyhow::ensure!(
        backend.shared_session().await.is_none(),
        "local exit retained remote authority"
    );
    gate.end_release.notify_one();
    target.wait().await?;
    let events = backend.events(&session.id, 0, 512).await?;
    anyhow::ensure!(
        events
            .iter()
            .filter(|event| matches!(event.kind, ejm_harness::EventKind::RunStarted { .. }))
            .count()
            == 1,
        "revoked remote prompt started"
    );
    backend.shutdown().await
}

async fn terminal_pairing_replacement() -> Result<()> {
    let (_directory, backend, session) = backend().await?;
    backend
        .begin_share(&session.id, "pairing".into(), ShareRole::Control, u64::MAX)
        .await?;
    let (mut task, sender, _release) = held_task("pairing");
    task.expires_at_ms = u64::MAX;
    let mut sharing = host(&session, task);
    let mut app = App::new(session);
    let (answer, abandoned) = oneshot::channel();
    sender
        .send(HostNotice::Confirm {
            fingerprint: "original controller fixture".into(),
            answer,
        })
        .await?;
    sharing.poll(&mut app, &backend, NOW).await;
    let mut terminal = crate::ui::TerminalGuard::enter()?;
    terminal.terminal.draw(|frame| sharing.draw(frame))?;
    // Parse the old approval into the real owned reader without consuming it.
    tokio::time::timeout(Duration::from_secs(10), async {
        while !terminal.poll_input()? {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        Result::<()>::Ok(())
    })
    .await??;
    drop(abandoned);
    let (answer, confirmed) = oneshot::channel();
    sender
        .send(HostNotice::Confirm {
            fingerprint: "replacement controller fixture".into(),
            answer,
        })
        .await?;
    app.notice("Pairing boundary passed");
    let args = Args::try_parse_from(["ejm-cli", "--replay"])?;
    crate::ui::run_app(
        &backend,
        &args,
        &crate::extensions::Extensions::empty()?,
        sharing,
        app,
        terminal,
    )
    .await?;
    anyhow::ensure!(confirmed.await?, "fresh confirmation was rejected");
    backend.shutdown().await
}

#[expect(
    clippy::panic,
    reason = "The PTY supervisor verifies private input is discarded before panic restores echo."
)]
fn terminal_panic_fixture() -> ! {
    panic!("private terminal panic cleanup fixture");
}

#[tokio::test]
#[ignore = "PTY supervisor drives private input, pairing panels and terminal restoration"]
async fn remote_terminal_interaction_fixture() -> Result<()> {
    let config = terminal_config()?;
    let mode = std::env::var("EJM_REMOTE_TERMINAL_CASE")
        .context("remote terminal fixture case missing")?;
    if mode == "host-signal" {
        let (directory, backend, _) = backend().await?;
        let args = Args::try_parse_from([
            "ejm-cli",
            "--replay",
            "--memory-only",
            "--relay",
            "https://relay.example",
            "--state-dir",
            directory.path().to_str().context("fixture path")?,
            "remote",
            "host",
        ])?;
        crate::ui::run(
            (*backend).clone(),
            &args,
            &crate::extensions::Extensions::empty()?,
        )
        .await?;
        return backend.shutdown().await;
    }
    if mode.starts_with("observer-") {
        return terminal_observer_loop(mode == "observer-signal").await;
    }
    if mode == "pairing-replacement" {
        return terminal_pairing_replacement().await;
    }
    #[cfg(unix)]
    let mut panic_signal =
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::user_defined1())?;
    #[cfg(windows)]
    let mut panic_signal = tokio::signal::windows::ctrl_break()?;
    let input = tokio::select! {
        result = super::super::protected::read_invitation(CancellationToken::new()) => result,
        () = crate::ui::termination_signal(), if mode != "panic" => {
            anyhow::ensure!(mode == "signal", "terminal fixture interrupted");
            return Ok(());
        }
        () = async {
            #[cfg(any(unix, windows))]
            let _ = panic_signal.recv().await;
            #[cfg(not(any(unix, windows)))]
            std::future::pending::<()>().await;
        }, if mode == "panic" => terminal_panic_fixture(),
    };
    if matches!(mode.as_str(), "flow" | "queued-complete" | "queued-partial") {
        anyhow::ensure!(
            input?.as_str() == config.input,
            "protected invitation input changed"
        );
        if mode == "flow" {
            terminal_host_flow(&config).await
        } else {
            terminal_private_boundary(&config).await
        }
    } else {
        anyhow::ensure!(
            matches!(mode.as_str(), "cancel" | "interrupt" | "error") && input.is_err(),
            "protected input did not cancel or reject the invalid paste"
        );
        Ok(())
    }
}
