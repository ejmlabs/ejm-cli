//! Explicit local integration entry point. This module never enters a production build.

use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use anyhow::{Context, Result, ensure};
use ejm_harness::{Event, EventKind, Harness, HarnessConfig, ReplayProvider, SessionOptions};
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, mpsc};
use tokio_util::sync::CancellationToken;
use zeroize::{Zeroize, Zeroizing};

use super::{
    super::{
        account::RelayAccount,
        backend::LocalBackend,
        identity::{EndpointIdentity, RelayOrigin},
        invitation::{Enrollment, Invitation, ShareRole},
    },
    HostNotice, now_ms, serve_host,
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FixtureConfig {
    origin: String,
    account_id: String,
    token: String,
    device_id: String,
    role: ShareRole,
    ready_path: PathBuf,
    stop_path: PathBuf,
    status_path: PathBuf,
}

impl Drop for FixtureConfig {
    fn drop(&mut self) {
        self.token.zeroize();
    }
}

#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
struct FixtureStatus {
    session_id: String,
    controller_fingerprint: Option<String>,
    locally_confirmed: bool,
    confirmations: u32,
    connections: u32,
    disconnections: u32,
    runs_started: u32,
    runs_finished: u32,
    steers: u32,
    last_output: String,
    through_sequence: u64,
    stopped: bool,
    failed: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FixtureReady<'a> {
    invitation: &'a str,
    origin: &'a str,
    session_id: &'a str,
    host_fingerprint: String,
}

fn read_config() -> Result<FixtureConfig> {
    let path = std::env::var_os("EJM_REMOTE_FIXTURE_CONFIG")
        .context("set EJM_REMOTE_FIXTURE_CONFIG to a private local fixture file")?;
    let path = PathBuf::from(path);
    let metadata = fs::symlink_metadata(&path)?;
    ensure!(
        metadata.is_file() && metadata.len() <= 16 * 1024,
        "invalid fixture config"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        ensure!(
            metadata.permissions().mode().trailing_zeros() >= 6,
            "fixture config must be private"
        );
    }
    let bytes = Zeroizing::new(fs::read(path)?);
    serde_json::from_slice(&bytes).context("invalid local fixture configuration")
}

fn write_private<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let parent = path.parent().context("fixture path needs a parent")?;
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    serde_json::to_writer(file.as_file_mut(), value)?;
    file.as_file_mut().flush()?;
    file.persist(path).map_err(|error| error.error)?;
    Ok(())
}

fn observe_event(status: &mut FixtureStatus, event: &Event) {
    status.through_sequence = event.sequence;
    match &event.kind {
        EventKind::RunStarted { .. } => status.runs_started += 1,
        EventKind::RunFinished { .. } => status.runs_finished += 1,
        EventKind::Steered { .. } => status.steers += 1,
        EventKind::TextDelta { text } => {
            status.last_output = text.chars().take(4096).collect();
        }
        _ => {}
    }
}

async fn observe_host(
    config: &FixtureConfig,
    backend: &LocalBackend,
    mut notices: mpsc::Receiver<HostNotice>,
    mut events: broadcast::Receiver<Event>,
    cancel: &CancellationToken,
    status: &mut FixtureStatus,
) -> Result<()> {
    let deadline = tokio::time::sleep(Duration::from_secs(90));
    tokio::pin!(deadline);
    let mut poll = tokio::time::interval(Duration::from_millis(50));
    loop {
        tokio::select! {
            () = &mut deadline => break,
            _ = poll.tick() => {
                if config.stop_path.exists() { break; }
                continue;
            }
            notice = notices.recv() => match notice {
                Some(HostNotice::Confirm { fingerprint, answer }) => {
                    status.controller_fingerprint = Some(fingerprint);
                    status.locally_confirmed = true;
                    status.confirmations += 1;
                    let _ = answer.send(true);
                }
                Some(HostNotice::Listening) => {},
                Some(HostNotice::Connected) => status.connections += 1,
                Some(HostNotice::Disconnected) => status.disconnections += 1,
                Some(HostNotice::Failed) => { status.failed = true; break; }
                None => break,
            },
            event = events.recv() => match event {
                Ok(event) if event.session_id == status.session_id => {
                    observe_event(status, &event);
                }
                Ok(_) => {},
                Err(broadcast::error::RecvError::Lagged(_)) => {
                    for event in backend.events(&status.session_id, status.through_sequence, 512).await? {
                        observe_event(status, &event);
                    }
                }
                Err(broadcast::error::RecvError::Closed) => break,
            },
        }
        write_private(&config.status_path, status)?;
    }
    cancel.cancel();
    backend.stop_share().await;
    status.stopped = true;
    write_private(&config.status_path, status)
}

#[tokio::test]
#[ignore = "local workerd/browser harness supplies a private fixture config"]
async fn browser_worker_native_host_fixture() -> Result<()> {
    let config = read_config()?;
    let origin = RelayOrigin::loopback_for_test(&config.origin)?;
    let account = RelayAccount::for_test(
        origin.clone(),
        &config.token,
        &config.account_id,
        &config.device_id,
    )?;
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
            title: "private offline browser fixture".into(),
        })
        .await?;
    let backend = LocalBackend::new(harness);
    let identity = EndpointIdentity::memory_only()?;
    let invite = Invitation::new(origin.clone(), identity.public_key(), config.role, now_ms())?;
    backend
        .begin_share(
            &session.id,
            invite.routing_id().into(),
            config.role,
            invite.share_expires_at_ms(),
        )
        .await?;
    let invitation = invite.protected_text()?;
    write_private(
        &config.ready_path,
        &FixtureReady {
            invitation: invitation.as_str(),
            origin: origin.as_str(),
            session_id: &session.id,
            host_fingerprint: identity.fingerprint(),
        },
    )?;
    let mut status = FixtureStatus {
        session_id: session.id,
        ..FixtureStatus::default()
    };
    write_private(&config.status_path, &status)?;
    let events = backend.subscribe();
    let (send, receive) = mpsc::channel(8);
    let cancel = CancellationToken::new();
    let mut task = tokio::spawn(serve_host(
        account,
        identity,
        Enrollment::new(invite),
        backend.clone(),
        cancel.clone(),
        send,
    ));
    let observed = observe_host(&config, &backend, receive, events, &cancel, &mut status).await;
    cancel.cancel();
    let served = tokio::time::timeout(Duration::from_secs(5), &mut task).await;
    if served.is_err() {
        task.abort();
    }
    let shutdown = backend.shutdown().await;
    observed?;
    shutdown?;
    served??
}

#[path = "native_roundtrip.rs"]
mod roundtrip;

#[tokio::test]
#[ignore = "ephemeral workerd runner supplies a private fixture config"]
async fn worker_native_roundtrip_fixture() -> Result<()> {
    Box::pin(roundtrip::run()).await
}
