//! Two production native transports through the ephemeral workerd service.
#[cfg(test)]
pub use contract::run;

#[cfg(test)]
mod contract {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    use anyhow::{Result, bail, ensure};
    use ejm_harness::{EventKind, ModelResponse, RunStatus, ToolCall};
    use serde::Serialize;
    use tokio::{sync::Semaphore, task::JoinHandle, time::timeout};

    use super::super::*;
    use crate::remote::{
        backend::expected,
        host_ui::HostUi,
        protocol::{
            ClientRequest, Mutation, MutationAction, MutationOutcome, ServerMessage, WireBody,
            WireMessage,
        },
        test_support::Gate,
        transport::{EncryptedSocket, connect_controller},
    };

    const FIRST: &str = "native-private-first-prompt-sentinel";
    const STEER: &str = "native-private-steering-sentinel";
    const QUEUED: &str = "native-private-queued-prompt-sentinel";
    const OUTPUT: &str = "native-private-model-output-sentinel";
    const CANCELLED: &str = "native-private-cancelled-prompt-sentinel";
    const DISCARDED: &str = "native-private-discarded-queue-sentinel";
    const REPLAY: &str = "native-private-replayed-prompt-sentinel";

    struct Notices {
        listening: Semaphore,
        disconnected: Semaphore,
        confirmations: AtomicUsize,
        connections: AtomicUsize,
    }
    impl Default for Notices {
        fn default() -> Self {
            Self {
                listening: Semaphore::new(0),
                disconnected: Semaphore::new(0),
                confirmations: AtomicUsize::new(0),
                connections: AtomicUsize::new(0),
            }
        }
    }

    struct Tasks {
        host: HostUi,
        notices: JoinHandle<Result<()>>,
    }
    impl Drop for Tasks {
        fn drop(&mut self) {
            self.notices.abort();
        }
    }

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct NativeStatus {
        paired: bool,
        runs_started: usize,
        confirmations: usize,
        reconnects: usize,
        revoked: bool,
        sentinels: Vec<&'static str>,
    }

    fn response(gate: Option<&str>) -> ModelResponse {
        ModelResponse {
            content: OUTPUT.into(),
            tool_calls: gate.map_or_else(Vec::new, |id| {
                vec![ToolCall {
                    id: id.into(),
                    name: "gate".into(),
                    arguments: serde_json::json!({}),
                }]
            }),
            ..ModelResponse::default()
        }
    }

    async fn receive(
        socket: &mut EncryptedSocket,
        mut wanted: impl FnMut(&ServerMessage) -> bool,
    ) -> Result<ServerMessage> {
        timeout(Duration::from_secs(10), async {
            loop {
                let WireBody::Server(message) = socket.receive().await?.body else {
                    bail!("fixture received client direction");
                };
                if wanted(&message) {
                    return Ok(message);
                }
                ensure!(
                    !matches!(
                        message,
                        ServerMessage::Error { .. } | ServerMessage::Stopped { .. }
                    ),
                    "host rejected fixture session"
                );
            }
        })
        .await?
    }

    async fn snapshot(socket: &mut EncryptedSocket) -> Result<ServerMessage> {
        socket
            .send(&WireMessage::client(ClientRequest::Snapshot {
                after_sequence: 0,
                limit: 512,
            }))
            .await?;
        receive(socket, |message| {
            matches!(message, ServerMessage::Snapshot { .. })
        })
        .await
    }

    fn state(snapshot: &ServerMessage) -> Result<crate::remote::protocol::ExpectedState> {
        if let ServerMessage::Snapshot {
            session,
            active_run,
            through_sequence,
            ..
        } = snapshot
        {
            Ok(expected(session, active_run.clone(), *through_sequence))
        } else {
            bail!("fixture snapshot missing");
        }
    }

    async fn action(
        socket: &mut EncryptedSocket,
        sequence: u64,
        snapshot: &ServerMessage,
        action: MutationAction,
    ) -> Result<MutationOutcome> {
        socket
            .send(&WireMessage::client(ClientRequest::Mutation {
                mutation: Mutation {
                    sequence,
                    expected: state(snapshot)?,
                    action,
                },
            }))
            .await?;
        result(socket, sequence).await
    }

    async fn result(socket: &mut EncryptedSocket, sequence: u64) -> Result<MutationOutcome> {
        let message = receive(socket, |message| matches!(message, ServerMessage::Outcome { sequence: id, .. } if *id == sequence)).await?;
        if let ServerMessage::Outcome { outcome, .. } = message {
            Ok(outcome)
        } else {
            bail!("fixture action result missing");
        }
    }

    fn accepted(outcome: MutationOutcome) -> Result<String> {
        if let MutationOutcome::Accepted {
            run_id: Some(run_id),
        } = outcome
        {
            Ok(run_id)
        } else {
            bail!("fixture action was not admitted");
        }
    }

    async fn finished(
        socket: &mut EncryptedSocket,
        run_id: &str,
        status: RunStatus,
    ) -> Result<ServerMessage> {
        timeout(Duration::from_secs(10), async {
            loop {
                let view = snapshot(socket).await?;
                if let ServerMessage::Snapshot { events, .. } = &view
                    && events.iter().any(|event| matches!(&event.kind, EventKind::RunFinished { summary } if summary.run_id == run_id && summary.status == status)) {
                    return Ok(view);
                }
                tokio::task::yield_now().await;
            }
        }).await?
    }

    async fn observe(mut input: mpsc::Receiver<HostNotice>, notices: Arc<Notices>) -> Result<()> {
        while let Some(notice) = input.recv().await {
            match notice {
                HostNotice::Listening => notices.listening.add_permits(1),
                HostNotice::Confirm { answer, .. } => {
                    notices.confirmations.fetch_add(1, Ordering::SeqCst);
                    let _ = answer.send(true);
                }
                HostNotice::Connected => {
                    notices.connections.fetch_add(1, Ordering::SeqCst);
                }
                HostNotice::Disconnected => notices.disconnected.add_permits(1),
                HostNotice::Failed => bail!("native host transport failed"),
            }
        }
        Ok(())
    }

    async fn barrier(semaphore: &Semaphore) -> Result<()> {
        timeout(Duration::from_secs(10), semaphore.acquire())
            .await??
            .forget();
        Ok(())
    }

    async fn first_turn(socket: &mut EncryptedSocket, gate: &Gate) -> Result<ServerMessage> {
        let initial = snapshot(socket).await?;
        let first = accepted(
            action(
                socket,
                1,
                &initial,
                MutationAction::Submit { text: FIRST.into() },
            )
            .await?,
        )?;
        gate.wait_until_entered().await?;
        let running = snapshot(socket).await?;
        ensure!(
            matches!(
                action(
                    socket,
                    2,
                    &running,
                    MutationAction::Steer {
                        run_id: first.clone(),
                        text: STEER.into()
                    }
                )
                .await?,
                MutationOutcome::Accepted { .. }
            ),
            "native steer denied"
        );
        let running = snapshot(socket).await?;
        ensure!(
            action(
                socket,
                3,
                &running,
                MutationAction::Queue {
                    run_id: first.clone(),
                    text: QUEUED.into()
                }
            )
            .await?
                == MutationOutcome::Queued,
            "native queue denied"
        );
        gate.release();
        finished(socket, &first, RunStatus::Completed).await?;
        socket
            .send(&WireMessage::client(ClientRequest::Outcome { sequence: 3 }))
            .await?;
        let queued = accepted(result(socket, 3).await?)?;
        finished(socket, &queued, RunStatus::Completed).await
    }

    async fn cancel_turn(
        socket: &mut EncryptedSocket,
        resumed: &ServerMessage,
        gate: &Gate,
    ) -> Result<()> {
        let target = accepted(
            action(
                socket,
                4,
                resumed,
                MutationAction::Submit {
                    text: CANCELLED.into(),
                },
            )
            .await?,
        )?;
        gate.wait_until_entered().await?;
        let running = snapshot(socket).await?;
        ensure!(
            action(
                socket,
                5,
                &running,
                MutationAction::Queue {
                    run_id: target.clone(),
                    text: DISCARDED.into()
                }
            )
            .await?
                == MutationOutcome::Queued,
            "second native queue denied"
        );
        let running = snapshot(socket).await?;
        ensure!(
            matches!(
                action(
                    socket,
                    6,
                    &running,
                    MutationAction::Cancel {
                        run_id: target.clone()
                    }
                )
                .await?,
                MutationOutcome::Accepted { .. }
            ),
            "native cancel denied"
        );
        finished(socket, &target, RunStatus::Cancelled).await?;
        socket
            .send(&WireMessage::client(ClientRequest::Outcome { sequence: 5 }))
            .await?;
        ensure!(
            matches!(result(socket, 5).await?, MutationOutcome::Rejected { .. }),
            "cancelled queue survived"
        );
        Ok(())
    }

    async fn reconnect_and_cancel(
        account: &RelayAccount,
        identity: &EndpointIdentity,
        invite: &Invitation,
        gate: &Gate,
        notices: &Notices,
        mut socket: EncryptedSocket,
    ) -> Result<(EncryptedSocket, String)> {
        socket.close().await;
        barrier(&notices.disconnected).await?;
        barrier(&notices.listening).await?;
        let mut socket = connect_controller(account, identity, invite).await?;
        let resumed = snapshot(&mut socket).await?;
        let session_id = state(&resumed)?.session_id;
        ensure!(
            matches!(
                resumed,
                ServerMessage::Snapshot {
                    mutation_high_water: 3,
                    ..
                }
            ),
            "share counter lost on native reconnect"
        );
        // Query or duplicate action IDs cannot start another prompt.
        socket
            .send(&WireMessage::client(ClientRequest::Outcome { sequence: 1 }))
            .await?;
        let old = result(&mut socket, 1).await?;
        ensure!(
            action(
                &mut socket,
                1,
                &resumed,
                MutationAction::Submit {
                    text: REPLAY.into()
                }
            )
            .await?
                == old,
            "duplicate action result changed"
        );
        cancel_turn(&mut socket, &resumed, gate).await?;
        Ok((socket, session_id))
    }

    async fn stop_and_revoke(
        account: &RelayAccount,
        identity: &EndpointIdentity,
        invite: &Invitation,
        backend: &LocalBackend,
        host: &mut HostUi,
        mut socket: EncryptedSocket,
        session_id: &str,
    ) -> Result<()> {
        host.stop(backend).await;
        let after_stop = backend
            .request(
                invite.routing_id(),
                ClientRequest::Snapshot {
                    after_sequence: 0,
                    limit: 512,
                },
                now_ms(),
            )
            .await;
        ensure!(after_stop.is_err(), "stopped share retained authority");
        let rooms = account
            .json(reqwest::Method::GET, "/v1/rooms", None)
            .await?;
        ensure!(
            rooms
                .get("rooms")
                .and_then(serde_json::Value::as_array)
                .is_some_and(Vec::is_empty),
            "native stop retained the account room reservation"
        );
        ensure!(
            account.ticket(invite.routing_id(), "host").await.is_err(),
            "native stop did not retire the original routing ID"
        );
        let replacement = Invitation::new(
            account.origin.clone(),
            identity.public_key(),
            ShareRole::Control,
            now_ms(),
        )?;
        account.ticket(replacement.routing_id(), "host").await?;
        socket.close().await;
        let events = backend.events(session_id, 0, 512).await?;
        let prompts: Vec<_> = events
            .iter()
            .filter_map(|event| match &event.kind {
                EventKind::RunStarted { prompt } => Some(prompt.as_str()),
                _ => None,
            })
            .collect();
        ensure!(
            prompts == [FIRST, QUEUED, CANCELLED],
            "native reconnect or cancelled queue repeated an action"
        );
        ensure!(
            events
                .iter()
                .any(|event| matches!(&event.kind, EventKind::Steered { text } if text == STEER)),
            "native steering event missing"
        );
        account
            .json(
                reqwest::Method::POST,
                &format!("/v1/devices/{}/revoke", account.device_id),
                Some(serde_json::json!({})),
            )
            .await?;
        ensure!(
            connect_controller(account, identity, invite).await.is_err(),
            "revoked device obtained a new relay ticket"
        );
        Ok(())
    }

    async fn exercise(
        account: &RelayAccount,
        invite: &Invitation,
        backend: &LocalBackend,
        host: &mut HostUi,
        gate: &Gate,
        notices: &Notices,
    ) -> Result<NativeStatus> {
        barrier(&notices.listening).await?;
        let controller = EndpointIdentity::memory_only()?;
        let mut socket = connect_controller(account, &controller, invite).await?;
        first_turn(&mut socket, gate).await?;
        let (socket, session_id) =
            reconnect_and_cancel(account, &controller, invite, gate, notices, socket).await?;
        stop_and_revoke(
            account,
            &controller,
            invite,
            backend,
            host,
            socket,
            &session_id,
        )
        .await?;
        let confirmations = notices.confirmations.load(Ordering::SeqCst);
        ensure!(
            confirmations == 1,
            "reconnect enrolled a new native controller"
        );
        ensure!(
            notices.connections.load(Ordering::SeqCst) == 2,
            "expected two native encrypted channels"
        );
        Ok(NativeStatus {
            paired: true,
            runs_started: 3,
            confirmations,
            reconnects: 1,
            revoked: true,
            sentinels: vec![FIRST, STEER, QUEUED, OUTPUT, CANCELLED, DISCARDED, REPLAY],
        })
    }

    pub async fn run() -> Result<()> {
        let config = read_config()?;
        let origin = RelayOrigin::loopback_for_test(&config.origin)?;
        let account = RelayAccount::for_test(
            origin.clone(),
            &config.token,
            &config.account_id,
            &config.device_id,
        )?;
        let directory = tempfile::tempdir()?;
        let gate = Arc::new(Gate::default());
        let harness = Harness::new(
            HarnessConfig {
                state_dir: directory.path().join("sessions"),
                ..HarnessConfig::default()
            },
            Arc::new(ReplayProvider::new(vec![
                response(Some("first-gate")),
                response(None),
                response(None),
                response(Some("cancel-gate")),
                response(None),
            ])),
            vec![gate.clone()],
        )?;
        let session = harness
            .create_session(SessionOptions {
                cwd: directory.path().into(),
                title: "offline native roundtrip".into(),
            })
            .await?;
        let backend = LocalBackend::new(harness);
        let identity = EndpointIdentity::memory_only()?;
        let invite = Invitation::new(
            origin.clone(),
            identity.public_key(),
            ShareRole::Control,
            now_ms(),
        )?;
        backend
            .begin_share(
                &session.id,
                invite.routing_id().into(),
                ShareRole::Control,
                invite.share_expires_at_ms(),
            )
            .await?;
        let invitation = invite.protected_text()?;
        let controller_invite = Invitation::decode(&invitation, &origin, now_ms())?;
        write_private(
            &config.ready_path,
            &FixtureReady {
                invitation: invitation.as_str(),
                origin: origin.as_str(),
                session_id: &session.id,
                host_fingerprint: identity.fingerprint(),
            },
        )?;
        let notices = Arc::new(Notices::default());
        let (sender, receiver) = mpsc::channel(8);
        let cancel = CancellationToken::new();
        let hosting = serve_host(
            account.clone(),
            identity,
            Enrollment::new(invite),
            backend.clone(),
            cancel.clone(),
            sender,
        );
        let mut tasks = Tasks {
            host: HostUi::worker_fixture(
                &session.id,
                &controller_invite,
                cancel,
                tokio::spawn(async move {
                    let _ = hosting.await;
                }),
            ),
            notices: tokio::spawn(observe(receiver, notices.clone())),
        };
        let result = exercise(
            &account,
            &controller_invite,
            &backend,
            &mut tasks.host,
            &gate,
            &notices,
        )
        .await;
        tasks.host.stop(&backend).await;
        let shutdown = backend.shutdown().await;
        let status = result?;
        shutdown?;
        write_private(&config.status_path, &status)
    }
}
