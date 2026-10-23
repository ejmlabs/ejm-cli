//! Host admission contracts exercised through the harness's public replay provider.

#[cfg(test)]
mod contracts {

    use std::{
        future::{Future, poll_fn},
        sync::{
            Arc,
            atomic::{AtomicU64, Ordering},
        },
        task::Poll,
        time::Duration,
    };

    use crate::remote::test_support::Gate;
    use anyhow::{Context, Result, bail};
    use async_trait::async_trait;
    use ejm_harness::{
        EventKind, Harness, HarnessConfig, Hook, HookContext, ModelResponse, ReplayProvider,
        RunHandle, RunStatus, RunSummary, Session, SessionOptions, ToolCall,
    };
    use tokio::{
        sync::{Notify, mpsc},
        time::timeout,
    };

    use crate::remote::{
        backend::LocalBackend,
        invitation::ShareRole,
        protocol::{
            ClientRequest, ExpectedState, MAX_PENDING_PROMPTS, MAX_PROMPT_BYTES, Mutation,
            MutationAction, MutationOutcome, RemoteError, ServerMessage,
        },
    };

    const EPOCH: &str = "first-share-epoch";
    const NOW: u64 = 1000;
    const EXPIRES: u64 = 10_000;

    struct Fixture {
        directory: tempfile::TempDir,
        backend: LocalBackend,
        session: Session,
        gate: Arc<Gate>,
    }

    impl Fixture {
        async fn new(responses: Option<Vec<ModelResponse>>, role: ShareRole) -> Result<Self> {
            Self::with_hooks(responses, role, vec![]).await
        }

        async fn with_hooks(
            responses: Option<Vec<ModelResponse>>,
            role: ShareRole,
            hooks: Vec<Arc<dyn Hook>>,
        ) -> Result<Self> {
            let directory = tempfile::tempdir()?;
            let gate = Arc::new(Gate::default());
            let provider = responses.map_or_else(ReplayProvider::echo, ReplayProvider::new);
            let harness = Harness::with_hooks(
                HarnessConfig {
                    state_dir: directory.path().join("sessions"),
                    ..HarnessConfig::default()
                },
                Arc::new(provider),
                vec![gate.clone()],
                hooks,
            )?;
            let session = harness
                .create_session(SessionOptions {
                    cwd: directory.path().into(),
                    title: "remote contract fixture".into(),
                })
                .await?;
            let mut backend = LocalBackend::new(harness);
            backend.admission_clock = Some(Arc::new(AtomicU64::new(NOW)));
            backend
                .begin_share(&session.id, EPOCH.into(), role, EXPIRES)
                .await?;
            Ok(Self {
                directory,
                backend,
                session,
                gate,
            })
        }

        async fn expected(&self) -> Result<ExpectedState> {
            let response = self
                .backend
                .request(
                    EPOCH,
                    ClientRequest::Snapshot {
                        after_sequence: 0,
                        limit: 512,
                    },
                    NOW,
                )
                .await?;
            let ServerMessage::Snapshot {
                session,
                active_run,
                through_sequence,
                ..
            } = response
            else {
                bail!("snapshot response missing");
            };
            Ok(ExpectedState {
                session_id: session.id,
                run_id: active_run,
                event_sequence: through_sequence,
            })
        }

        async fn act(
            &self,
            sequence: u64,
            expected: ExpectedState,
            action: MutationAction,
        ) -> Result<MutationOutcome> {
            let response = self
                .backend
                .request(
                    EPOCH,
                    ClientRequest::Mutation {
                        mutation: Mutation {
                            sequence,
                            expected,
                            action,
                        },
                    },
                    NOW,
                )
                .await?;
            outcome(response)
        }

        async fn outcome(&self, sequence: u64) -> Result<MutationOutcome> {
            outcome(
                self.backend
                    .request(EPOCH, ClientRequest::Outcome { sequence }, NOW)
                    .await?,
            )
        }

        async fn started_prompts(&self) -> Result<Vec<String>> {
            Ok(self
                .backend
                .events(&self.session.id, 0, 512)
                .await?
                .into_iter()
                .filter_map(|event| {
                    if let EventKind::RunStarted { prompt } = event.kind {
                        Some(prompt)
                    } else {
                        None
                    }
                })
                .collect())
        }

        async fn finish_run(&self, run_id: &str) -> Result<RunSummary> {
            let mut notices = self.backend.subscribe();
            timeout(Duration::from_secs(3), async {
                let mut cursor = 0;
                loop {
                    for event in self.backend.events(&self.session.id, cursor, 512).await? {
                        cursor = event.sequence;
                        if let EventKind::RunFinished { summary } = event.kind
                            && summary.run_id == run_id
                        {
                            if let Some(mut handle) =
                                self.backend.subscribe_run(&self.session.id, run_id).await
                            {
                                handle.wait().await?;
                            }
                            return Ok(summary);
                        }
                    }
                    notices.recv().await?;
                }
            })
            .await?
        }
    }

    fn outcome(response: ServerMessage) -> Result<MutationOutcome> {
        if let ServerMessage::Outcome { outcome, .. } = response {
            Ok(outcome)
        } else {
            bail!("mutation outcome missing")
        }
    }

    fn accepted(outcome: MutationOutcome) -> Result<String> {
        if let MutationOutcome::Accepted { run_id: Some(run) } = outcome {
            Ok(run)
        } else {
            bail!("turn was not admitted")
        }
    }

    fn assert_rejected(outcome: &MutationOutcome, code: RemoteError) {
        assert!(
            matches!(outcome, MutationOutcome::Rejected { code: actual, .. } if *actual == code)
        );
    }

    fn gate_response(id: &str) -> ModelResponse {
        ModelResponse {
            tool_calls: vec![ToolCall {
                id: id.into(),
                name: "gate".into(),
                arguments: serde_json::json!({}),
            }],
            ..ModelResponse::default()
        }
    }

    fn done() -> ModelResponse {
        ModelResponse {
            content: "offline completion".into(),
            ..ModelResponse::default()
        }
    }

    async fn wait(handle: &mut RunHandle) -> Result<RunSummary> {
        timeout(Duration::from_secs(3), handle.wait()).await?
    }

    #[tokio::test]
    async fn snapshot_authority_covers_completion_journaled_after_initial_refresh() -> Result<()> {
        let mut fixture = Fixture::new(
            Some(vec![gate_response("snapshot-boundary"), done(), done()]),
            ShareRole::Control,
        )
        .await?;
        let mut target = fixture
            .backend
            .start(&fixture.session.id, "snapshot target".into())
            .await?;
        fixture.gate.wait_until_entered().await?;
        let boundary = Arc::new(super::super::RequestBoundary::default());
        fixture.backend.request_boundary = Some(boundary.clone());
        let snapshot = {
            let request = fixture.expected();
            let mut request = std::pin::pin!(request);
            tokio::select! {
                () = boundary.reached.notified() => {},
                _ = &mut request => bail!("snapshot skipped its construction boundary"),
            }
            fixture.gate.release();
            assert_eq!(wait(&mut target).await?.status, RunStatus::Completed);
            boundary.release.notify_one();
            request.await?
        };
        fixture.backend.request_boundary = None;
        let finished = fixture.backend.events(&fixture.session.id, 0, 512).await?.into_iter()
            .find(|event| matches!(&event.kind, EventKind::RunFinished { summary } if summary.run_id == target.id))
            .context("missing completed snapshot target")?;
        assert!(snapshot.event_sequence >= finished.sequence);
        assert_eq!(snapshot.run_id, None);
        let run = accepted(
            fixture
                .act(
                    1,
                    snapshot,
                    MutationAction::Submit {
                        text: "ordinary prompt after snapshot".into(),
                    },
                )
                .await?,
        )?;
        fixture.finish_run(&run).await?;
        fixture.backend.shutdown().await
    }

    #[tokio::test]
    async fn mutation_waiting_for_authority_or_refresh_cannot_cross_share_expiry() -> Result<()> {
        for after_refresh in [false, true] {
            let mut fixture = Fixture::new(None, ShareRole::Control).await?;
            let expected = fixture.expected().await?;
            let boundary = Arc::new(super::super::RequestBoundary::default());
            fixture.backend.request_boundary = after_refresh.then(|| boundary.clone());
            let guard = fixture.backend.authority.lock().await;
            let request = fixture.backend.request(
                EPOCH,
                ClientRequest::Mutation {
                    mutation: Mutation {
                        sequence: 1,
                        expected,
                        action: MutationAction::Submit {
                            text: "expired prompt".into(),
                        },
                    },
                },
                NOW,
            );
            let mut request = std::pin::pin!(request);
            assert!(futures_util::poll!(&mut request).is_pending());
            let clock = fixture
                .backend
                .admission_clock
                .as_ref()
                .context("missing fixture clock")?;
            if !after_refresh {
                clock.store(EXPIRES, Ordering::Relaxed);
            }
            drop(guard);
            if after_refresh {
                tokio::select! {
                    () = boundary.reached.notified() => {},
                    _ = &mut request => bail!("request skipped its refresh boundary"),
                }
                clock.store(EXPIRES, Ordering::Relaxed);
            }
            boundary.release.notify_one();
            assert!(request.await.is_err());
            assert_eq!(fixture.started_prompts().await?, Vec::<String>::new());
            assert_eq!(
                fixture
                    .backend
                    .authority
                    .lock()
                    .await
                    .share
                    .as_ref()
                    .context("missing share")?
                    .ledger
                    .high_water(),
                0
            );
            fixture.backend.shutdown().await?;
        }
        Ok(())
    }

    struct CompletionGates {
        started: mpsc::Sender<String>,
        start_release: Notify,
        ended: mpsc::Sender<String>,
        end_release: Notify,
    }

    impl CompletionGates {
        fn new() -> (Arc<Self>, mpsc::Receiver<String>, mpsc::Receiver<String>) {
            let (started, starts) = mpsc::channel(4);
            let (ended, ends) = mpsc::channel(4);
            (
                Arc::new(Self {
                    started,
                    start_release: Notify::new(),
                    ended,
                    end_release: Notify::new(),
                }),
                starts,
                ends,
            )
        }
    }

    #[async_trait]
    impl Hook for CompletionGates {
        async fn run_start(&self, context: &HookContext, prompt: String) -> Result<String> {
            self.started.send(context.run_id.clone()).await?;
            self.start_release.notified().await;
            Ok(prompt)
        }

        async fn run_end(&self, _: &HookContext, summary: &RunSummary) -> Result<()> {
            self.ended.send(summary.run_id.clone()).await?;
            self.end_release.notified().await;
            Ok(())
        }
    }

    async fn assert_waiting(
        settling: &mut (impl Future<Output = Result<()>> + Unpin),
        fixture: &Fixture,
        sequence: u64,
    ) -> Result<()> {
        settling.await?;
        assert_eq!(fixture.outcome(sequence).await?, MutationOutcome::Queued);
        Ok(())
    }

    #[tokio::test]
    async fn exact_state_duplicate_ids_and_evicted_results_cannot_repeat_a_turn() -> Result<()> {
        let fixture = Fixture::new(None, ShareRole::Control).await?;
        let expected = fixture.expected().await?;
        let mut stale = expected.clone();
        stale.event_sequence += 1;
        assert_rejected(
            &fixture
                .act(
                    1,
                    stale,
                    MutationAction::Submit {
                        text: "stale".into(),
                    },
                )
                .await?,
            RemoteError::StaleState,
        );
        assert_rejected(
            &fixture
                .act(
                    1,
                    expected.clone(),
                    MutationAction::Submit {
                        text: "reused rejected id".into(),
                    },
                )
                .await?,
            RemoteError::StaleState,
        );
        assert_eq!(fixture.started_prompts().await?, Vec::<String>::new());
        let first = fixture
            .act(
                2,
                expected.clone(),
                MutationAction::Submit {
                    text: "execute once".into(),
                },
            )
            .await?;
        let run = accepted(first.clone())?;
        assert_eq!(
            fixture
                .act(
                    2,
                    expected,
                    MutationAction::Submit {
                        text: "duplicate payload".into()
                    }
                )
                .await?,
            first
        );
        assert_eq!(fixture.finish_run(&run).await?.status, RunStatus::Completed);
        let mut stale = fixture.expected().await?;
        stale.session_id = "another-session".into();
        for sequence in 3..=131 {
            assert_rejected(
                &fixture
                    .act(
                        sequence,
                        stale.clone(),
                        MutationAction::Submit {
                            text: "never dispatched".into(),
                        },
                    )
                    .await?,
                RemoteError::StaleState,
            );
        }
        assert_eq!(fixture.outcome(2).await?, MutationOutcome::Unknown);
        assert_eq!(
            fixture
                .act(
                    2,
                    fixture.expected().await?,
                    MutationAction::Submit {
                        text: "evicted duplicate".into()
                    }
                )
                .await?,
            MutationOutcome::Unknown
        );
        assert_eq!(fixture.started_prompts().await?, ["execute once"]);
        fixture.backend.shutdown().await
    }

    #[tokio::test]
    async fn view_access_and_old_share_epochs_have_no_mutation_authority() -> Result<()> {
        let fixture = Fixture::new(None, ShareRole::View).await?;
        for (index, action) in [
            MutationAction::Submit {
                text: "view submit".into(),
            },
            MutationAction::Steer {
                run_id: "named-run".into(),
                text: "view steer".into(),
            },
            MutationAction::Queue {
                run_id: "named-run".into(),
                text: "view queue".into(),
            },
            MutationAction::Cancel {
                run_id: "named-run".into(),
            },
        ]
        .into_iter()
        .enumerate()
        {
            let mut expected = fixture.expected().await?;
            if index > 0 {
                expected.run_id = Some("named-run".into());
            }
            assert_rejected(
                &fixture
                    .act(u64::try_from(index)? + 1, expected, action)
                    .await?,
                RemoteError::AccessDenied,
            );
        }
        assert_eq!(fixture.started_prompts().await?, Vec::<String>::new());
        assert!(fixture.backend.stop_share_if(EPOCH).await);
        fixture
            .backend
            .begin_share(
                &fixture.session.id,
                "replacement-epoch".into(),
                ShareRole::Control,
                EXPIRES,
            )
            .await?;
        assert!(
            !fixture
                .backend
                .is_current_share(EPOCH, &fixture.session.id, NOW)
                .await
        );
        assert!(
            fixture
                .backend
                .request(
                    EPOCH,
                    ClientRequest::Snapshot {
                        after_sequence: 0,
                        limit: 512
                    },
                    NOW
                )
                .await
                .is_err()
        );
        let response = fixture
            .backend
            .request(
                "replacement-epoch",
                ClientRequest::Mutation {
                    mutation: Mutation {
                        sequence: 1,
                        expected: ExpectedState {
                            session_id: fixture.session.id.clone(),
                            run_id: None,
                            event_sequence: 0,
                        },
                        action: MutationAction::Submit {
                            text: "new share only".into(),
                        },
                    },
                },
                NOW,
            )
            .await?;
        let run = accepted(outcome(response)?)?;
        assert_eq!(fixture.finish_run(&run).await?.status, RunStatus::Completed);
        assert_eq!(fixture.started_prompts().await?, ["new share only"]);
        fixture.backend.shutdown().await
    }

    #[tokio::test]
    async fn stop_discards_the_bounded_queue_before_the_target_can_finish() -> Result<()> {
        let fixture = Fixture::new(
            Some(vec![gate_response("gate-stop"), done()]),
            ShareRole::Control,
        )
        .await?;
        let mut target = fixture
            .backend
            .start(&fixture.session.id, "local target".into())
            .await?;
        fixture.gate.wait_until_entered().await?;
        let expected = fixture.expected().await?;
        for sequence in 1..=u64::try_from(MAX_PENDING_PROMPTS)? {
            assert_eq!(
                fixture
                    .act(
                        sequence,
                        expected.clone(),
                        MutationAction::Queue {
                            run_id: target.id.clone(),
                            text: format!("queued {sequence}")
                        }
                    )
                    .await?,
                MutationOutcome::Queued
            );
        }
        assert_rejected(
            &fixture
                .act(
                    9,
                    expected,
                    MutationAction::Queue {
                        run_id: target.id.clone(),
                        text: "queue overflow".into(),
                    },
                )
                .await?,
            RemoteError::QueueFull,
        );
        assert!(fixture.backend.stop_share_if(EPOCH).await);
        fixture.gate.release();
        assert_eq!(wait(&mut target).await?.status, RunStatus::Completed);
        fixture.backend.settle(NOW).await?;
        assert_eq!(fixture.started_prompts().await?, ["local target"]);
        assert!(fixture.backend.shared_session().await.is_none());
        fixture.backend.shutdown().await
    }

    #[tokio::test]
    async fn stale_cancel_cannot_cancel_a_replacement_local_turn() -> Result<()> {
        let fixture = Fixture::new(
            Some(vec![
                gate_response("gate-first"),
                gate_response("gate-replacement"),
                done(),
            ]),
            ShareRole::Control,
        )
        .await?;
        let mut first = fixture
            .backend
            .start(&fixture.session.id, "first local turn".into())
            .await?;
        fixture.gate.wait_until_entered().await?;
        let previous = fixture.expected().await?;
        fixture.backend.cancel(&fixture.session.id).await?;
        assert_eq!(wait(&mut first).await?.status, RunStatus::Cancelled);
        let mut replacement = fixture
            .backend
            .start(&fixture.session.id, "replacement local turn".into())
            .await?;
        fixture.gate.wait_until_entered().await?;
        assert_rejected(
            &fixture
                .act(1, previous, MutationAction::Cancel { run_id: first.id })
                .await?,
            RemoteError::StaleState,
        );
        assert_eq!(
            fixture.expected().await?.run_id.as_deref(),
            Some(replacement.id.as_str())
        );
        fixture.gate.release();
        assert_eq!(wait(&mut replacement).await?.status, RunStatus::Completed);
        fixture.backend.shutdown().await
    }

    #[tokio::test]
    async fn output_progress_keeps_exact_run_controls_valid_and_idle_aba_stays_stale() -> Result<()>
    {
        let fixture = Fixture::new(
            Some(vec![gate_response("stream-gate"), done()]),
            ShareRole::Control,
        )
        .await?;
        let idle = fixture.expected().await?;
        let mut target = fixture
            .backend
            .start(&fixture.session.id, "streaming turn".into())
            .await?;
        fixture.gate.wait_until_entered().await?;
        let events = fixture.backend.events(&fixture.session.id, 0, 512).await?;
        let start = events
            .iter()
            .find(|event| matches!(event.kind, EventKind::RunStarted { .. }))
            .ok_or_else(|| anyhow::anyhow!("run start missing"))?;
        assert!(
            events
                .last()
                .is_some_and(|event| event.sequence > start.sequence)
        );
        let expected = ExpectedState {
            session_id: fixture.session.id.clone(),
            run_id: Some(target.id.clone()),
            event_sequence: start.sequence,
        };
        assert!(matches!(
            fixture
                .act(
                    1,
                    expected.clone(),
                    MutationAction::Steer {
                        run_id: target.id.clone(),
                        text: "steer while output advances".into(),
                    }
                )
                .await?,
            MutationOutcome::Accepted { .. }
        ));
        assert!(matches!(
            fixture
                .act(
                    2,
                    expected,
                    MutationAction::Cancel {
                        run_id: target.id.clone(),
                    }
                )
                .await?,
            MutationOutcome::Accepted { .. }
        ));
        assert_eq!(wait(&mut target).await?.status, RunStatus::Cancelled);
        assert_rejected(
            &fixture
                .act(
                    3,
                    idle,
                    MutationAction::Submit {
                        text: "stale idle view before completed intervening turn".into(),
                    },
                )
                .await?,
            RemoteError::StaleState,
        );
        let snapshot = fixture
            .backend
            .request(
                EPOCH,
                ClientRequest::Snapshot {
                    after_sequence: 0,
                    limit: 512,
                },
                NOW,
            )
            .await?;
        assert!(matches!(
            snapshot,
            ServerMessage::Snapshot {
                mutation_high_water: 3,
                ..
            }
        ));
        fixture.backend.shutdown().await
    }

    #[tokio::test]
    async fn exact_run_steering_and_successful_remote_queue_form_an_explicit_chain() -> Result<()> {
        let fixture = Fixture::new(
            Some(vec![gate_response("gate-chain"), done(), done(), done()]),
            ShareRole::Control,
        )
        .await?;
        let mut target = fixture
            .backend
            .start(&fixture.session.id, "chain target".into())
            .await?;
        fixture.gate.wait_until_entered().await?;
        assert!(matches!(
            fixture
                .act(
                    1,
                    fixture.expected().await?,
                    MutationAction::Steer {
                        run_id: target.id.clone(),
                        text: "remote hint".into()
                    }
                )
                .await?,
            MutationOutcome::Accepted { .. }
        ));
        for (sequence, text) in [(2, "queued alpha"), (3, "queued beta")] {
            assert_eq!(
                fixture
                    .act(
                        sequence,
                        fixture.expected().await?,
                        MutationAction::Queue {
                            run_id: target.id.clone(),
                            text: text.into()
                        }
                    )
                    .await?,
                MutationOutcome::Queued
            );
        }
        fixture.gate.release();
        assert_eq!(wait(&mut target).await?.status, RunStatus::Completed);
        for sequence in [2, 3] {
            fixture.backend.settle(NOW).await?;
            let run = accepted(fixture.outcome(sequence).await?)?;
            assert_eq!(fixture.finish_run(&run).await?.status, RunStatus::Completed);
        }
        assert_eq!(
            fixture.started_prompts().await?,
            ["chain target", "queued alpha", "queued beta"]
        );
        assert!(
        fixture
            .backend
            .events(&fixture.session.id, 0, 512)
            .await?
            .iter()
            .any(
                |event| matches!(&event.kind, EventKind::Steered { text } if text == "remote hint")
            )
    );
        fixture.backend.shutdown().await
    }

    #[tokio::test]
    async fn internal_successor_and_two_remote_successors_wait_for_completion() -> Result<()> {
        timeout(Duration::from_secs(10), async {
            let (hooks, mut starts, mut ends) = CompletionGates::new();
            let fixture =
                Fixture::with_hooks(None, ShareRole::Control, vec![hooks.clone()]).await?;
            let mut first = fixture
                .backend
                .start(&fixture.session.id, "local first".into())
                .await?;
            assert_eq!(
                starts.recv().await.context("missing first start")?,
                first.id
            );
            fixture
                .backend
                .enqueue(&fixture.session.id, "internal successor".into())
                .await?;
            hooks.start_release.notify_one();
            assert_eq!(ends.recv().await.context("missing first end")?, first.id);
            hooks.end_release.notify_one();
            assert_eq!(first.wait().await?.status, RunStatus::Completed);
            let mut target = starts.recv().await.context("missing internal successor")?;
            for (sequence, text) in [(1, "remote alpha"), (2, "remote beta")] {
                assert_eq!(
                    fixture
                        .act(
                            sequence,
                            fixture.expected().await?,
                            MutationAction::Queue {
                                run_id: target.clone(),
                                text: text.into(),
                            },
                        )
                        .await?,
                    MutationOutcome::Queued
                );
            }
            for sequence in [1, 2] {
                hooks.start_release.notify_one();
                assert_eq!(ends.recv().await.context("missing target end")?, target);
                let settling = fixture.backend.settle(NOW);
                tokio::pin!(settling);
                assert_waiting(&mut settling, &fixture, sequence).await?;
                assert_eq!(fixture.outcome(2).await?, MutationOutcome::Queued);
                hooks.end_release.notify_one();
                fixture.backend.queue_ready(NOW).await?;
                fixture.backend.settle(NOW).await?;
                target = accepted(fixture.outcome(sequence).await?)?;
                assert_eq!(
                    starts.recv().await.context("missing admitted start")?,
                    target
                );
            }
            let mut last = fixture
                .backend
                .subscribe_run(&fixture.session.id, &target)
                .await
                .context("missing final completion")?;
            hooks.start_release.notify_one();
            assert_eq!(ends.recv().await.context("missing final end")?, target);
            hooks.end_release.notify_one();
            assert_eq!(last.wait().await?.status, RunStatus::Completed);
            assert_eq!(
                fixture.started_prompts().await?,
                [
                    "local first",
                    "internal successor",
                    "remote alpha",
                    "remote beta"
                ]
            );
            fixture.backend.shutdown().await
        })
        .await?
    }

    #[tokio::test]
    async fn revocation_during_completion_cannot_touch_a_replacement_share() -> Result<()> {
        timeout(Duration::from_secs(10), async {
            let (hooks, mut starts, mut ends) = CompletionGates::new();
            let fixture =
                Fixture::with_hooks(None, ShareRole::Control, vec![hooks.clone()]).await?;
            let mut target = fixture
                .backend
                .start(&fixture.session.id, "revoked target".into())
                .await?;
            assert_eq!(
                starts.recv().await.context("missing target start")?,
                target.id
            );
            assert_eq!(
                fixture
                    .act(
                        1,
                        fixture.expected().await?,
                        MutationAction::Queue {
                            run_id: target.id.clone(),
                            text: "must stay revoked".into(),
                        },
                    )
                    .await?,
                MutationOutcome::Queued
            );
            hooks.start_release.notify_one();
            assert_eq!(ends.recv().await.context("missing target end")?, target.id);
            let settling = fixture.backend.settle(NOW);
            tokio::pin!(settling);
            assert_waiting(&mut settling, &fixture, 1).await?;
            assert!(fixture.backend.stop_share_if(EPOCH).await);
            fixture
                .backend
                .begin_share(
                    &fixture.session.id,
                    "replacement".into(),
                    ShareRole::Control,
                    EXPIRES,
                )
                .await?;
            hooks.end_release.notify_one();
            assert_eq!(target.wait().await?.status, RunStatus::Completed);
            fixture.backend.settle(NOW).await?;
            assert!(
                fixture
                    .backend
                    .is_current_share("replacement", &fixture.session.id, NOW)
                    .await
            );
            assert_eq!(fixture.started_prompts().await?, ["revoked target"]);
            fixture.backend.shutdown().await
        })
        .await?
    }

    #[tokio::test]
    async fn local_stop_cancels_successors_during_run_end_cleanup() -> Result<()> {
        timeout(Duration::from_secs(10), async {
            let (hooks, mut starts, mut ends) = CompletionGates::new();
            let fixture =
                Fixture::with_hooks(None, ShareRole::Control, vec![hooks.clone()]).await?;
            let mut target = fixture
                .backend
                .start(&fixture.session.id, "local target".into())
                .await?;
            assert_eq!(starts.recv().await.context("missing target")?, target.id);
            fixture
                .backend
                .enqueue(&fixture.session.id, "must not start".into())
                .await?;
            hooks.start_release.notify_one();
            assert_eq!(ends.recv().await.context("missing end")?, target.id);
            assert!(fixture.expected().await?.run_id.is_none());
            fixture.backend.cancel(&fixture.session.id).await?;
            hooks.end_release.notify_one();
            let completion = timeout(Duration::from_secs(3), target.wait_completion()).await??;
            assert!(!completion.queued_successor);
            assert_eq!(fixture.started_prompts().await?, ["local target"]);
            fixture.backend.shutdown().await
        })
        .await?
    }

    #[tokio::test]
    async fn completed_local_successor_discards_the_predecessors_remote_chain() -> Result<()> {
        timeout(Duration::from_secs(10), async {
            let (hooks, mut starts, mut ends) = CompletionGates::new();
            let fixture =
                Fixture::with_hooks(None, ShareRole::Control, vec![hooks.clone()]).await?;
            let target = fixture
                .backend
                .start(&fixture.session.id, "predecessor".into())
                .await?;
            assert_eq!(
                starts.recv().await.context("missing predecessor")?,
                target.id
            );
            fixture
                .backend
                .enqueue(&fixture.session.id, "local successor".into())
                .await?;
            assert_eq!(
                fixture
                    .act(
                        1,
                        fixture.expected().await?,
                        MutationAction::Queue {
                            run_id: target.id.clone(),
                            text: "stale dependent prompt".into(),
                        },
                    )
                    .await?,
                MutationOutcome::Queued
            );
            hooks.start_release.notify_one();
            assert_eq!(
                ends.recv().await.context("missing predecessor end")?,
                target.id
            );
            let settling = fixture.backend.settle(NOW);
            tokio::pin!(settling);
            assert_waiting(&mut settling, &fixture, 1).await?;
            hooks.end_release.notify_one();
            let successor = starts.recv().await.context("missing local successor")?;
            let mut completion = fixture
                .backend
                .subscribe_run(&fixture.session.id, &successor)
                .await
                .context("missing local successor completion")?;
            hooks.start_release.notify_one();
            assert_eq!(
                ends.recv().await.context("missing local successor end")?,
                successor
            );
            hooks.end_release.notify_one();
            assert_eq!(completion.wait().await?.status, RunStatus::Completed);
            // Resume host admission only after the local successor has disappeared.
            fixture.backend.settle(NOW).await?;
            assert_rejected(&fixture.outcome(1).await?, RemoteError::StaleState);
            assert_eq!(
                fixture.started_prompts().await?,
                ["predecessor", "local successor"]
            );
            fixture.backend.shutdown().await
        })
        .await?
    }

    #[tokio::test]
    async fn utf8_prompt_limit_rejects_oversize_without_discarding_valid_queue() -> Result<()> {
        let fixture = Fixture::new(
            Some(vec![gate_response("prompt-bound"), done(), done()]),
            ShareRole::Control,
        )
        .await?;
        let mut target = fixture
            .backend
            .start(&fixture.session.id, "prompt boundary target".into())
            .await?;
        fixture.gate.wait_until_entered().await?;
        let text = format!("{}x", "界".repeat(MAX_PROMPT_BYTES / 3));
        assert_eq!(text.len(), 16_384);
        assert_eq!(
            fixture
                .act(
                    1,
                    fixture.expected().await?,
                    MutationAction::Queue {
                        run_id: target.id.clone(),
                        text: text.clone(),
                    },
                )
                .await?,
            MutationOutcome::Queued
        );
        for (sequence, action) in [
            MutationAction::Queue {
                run_id: target.id.clone(),
                text: format!("{text}x"),
            },
            MutationAction::Steer {
                run_id: target.id.clone(),
                text: format!("{text}x"),
            },
            MutationAction::Submit {
                text: format!("{text}x"),
            },
        ]
        .into_iter()
        .enumerate()
        {
            assert_rejected(
                &fixture
                    .act(
                        u64::try_from(sequence)? + 2,
                        fixture.expected().await?,
                        action,
                    )
                    .await?,
                RemoteError::InvalidRequest,
            );
        }
        assert_eq!(fixture.outcome(1).await?, MutationOutcome::Queued);
        fixture.gate.release();
        assert_eq!(target.wait().await?.status, RunStatus::Completed);
        fixture.backend.settle(NOW).await?;
        let admitted = accepted(fixture.outcome(1).await?)?;
        assert_eq!(
            fixture.finish_run(&admitted).await?.status,
            RunStatus::Completed
        );
        assert_eq!(
            fixture.started_prompts().await?,
            ["prompt boundary target", &text]
        );
        fixture.backend.shutdown().await
    }

    enum QueueBoundary {
        LocalStart,
        LocalCancel,
        FailedTarget,
        Expiry,
    }

    #[tokio::test]
    async fn local_control_failure_and_expiry_win_before_queued_admission() -> Result<()> {
        for boundary in [
            QueueBoundary::LocalStart,
            QueueBoundary::LocalCancel,
            QueueBoundary::FailedTarget,
            QueueBoundary::Expiry,
        ] {
            let responses = if matches!(boundary, QueueBoundary::FailedTarget) {
                vec![gate_response("gate-fail")]
            } else {
                vec![gate_response("gate-boundary"), done(), done()]
            };
            let fixture = Fixture::new(Some(responses), ShareRole::Control).await?;
            let mut target = fixture
                .backend
                .start(&fixture.session.id, "boundary target".into())
                .await?;
            fixture.gate.wait_until_entered().await?;
            assert_eq!(
                fixture
                    .act(
                        1,
                        fixture.expected().await?,
                        MutationAction::Queue {
                            run_id: target.id.clone(),
                            text: "must be discarded".into()
                        }
                    )
                    .await?,
                MutationOutcome::Queued
            );
            if matches!(boundary, QueueBoundary::Expiry) {
                fixture
                    .backend
                    .admission_clock
                    .as_ref()
                    .context("missing fixture clock")?
                    .store(EXPIRES, Ordering::Relaxed);
            }
            if matches!(boundary, QueueBoundary::LocalCancel) {
                fixture.backend.cancel(&fixture.session.id).await?;
            } else {
                fixture.gate.release();
            }
            let finished = wait(&mut target).await?;
            let expected_status = match boundary {
                QueueBoundary::LocalCancel => RunStatus::Cancelled,
                QueueBoundary::FailedTarget => RunStatus::Failed,
                _ => RunStatus::Completed,
            };
            assert_eq!(finished.status, expected_status);
            if matches!(boundary, QueueBoundary::LocalStart) {
                let mut local = fixture
                    .backend
                    .start(&fixture.session.id, "local admission wins".into())
                    .await?;
                assert_eq!(wait(&mut local).await?.status, RunStatus::Completed);
            }
            fixture.backend.settle(NOW).await?;
            if matches!(boundary, QueueBoundary::Expiry) {
                assert!(fixture.backend.shared_session().await.is_none());
            } else {
                assert_rejected(&fixture.outcome(1).await?, RemoteError::StaleState);
            }
            let prompts = fixture.started_prompts().await?;
            assert!(!prompts.iter().any(|prompt| prompt == "must be discarded"));
            assert_eq!(
                prompts.len(),
                if matches!(boundary, QueueBoundary::LocalStart) {
                    2
                } else {
                    1
                }
            );
            fixture.backend.shutdown().await?;
        }
        Ok(())
    }

    #[tokio::test]
    async fn eviction_of_a_queued_result_keeps_its_execution_high_water() -> Result<()> {
        let fixture = Fixture::new(
            Some(vec![gate_response("gate-evicted"), done(), done()]),
            ShareRole::Control,
        )
        .await?;
        let mut target = fixture
            .backend
            .start(&fixture.session.id, "eviction target".into())
            .await?;
        fixture.gate.wait_until_entered().await?;
        let expected = fixture.expected().await?;
        assert_eq!(
            fixture
                .act(
                    1,
                    expected.clone(),
                    MutationAction::Queue {
                        run_id: target.id.clone(),
                        text: "evicted queued action".into()
                    }
                )
                .await?,
            MutationOutcome::Queued
        );
        let mut stale = expected;
        stale.session_id = "another-session".into();
        for sequence in 2..=130 {
            assert_rejected(
                &fixture
                    .act(
                        sequence,
                        stale.clone(),
                        MutationAction::Submit {
                            text: "rejected".into(),
                        },
                    )
                    .await?,
                RemoteError::StaleState,
            );
        }
        fixture.gate.release();
        assert_eq!(wait(&mut target).await?.status, RunStatus::Completed);
        fixture.backend.settle(NOW).await?;
        assert_eq!(fixture.outcome(1).await?, MutationOutcome::Unknown);
        let run = fixture.expected().await?.run_id;
        if let Some(run) = run {
            fixture.finish_run(&run).await?;
        }
        assert_eq!(
            fixture
                .act(
                    1,
                    fixture.expected().await?,
                    MutationAction::Submit {
                        text: "duplicate evicted action".into()
                    }
                )
                .await?,
            MutationOutcome::Unknown
        );
        assert_eq!(
            fixture.started_prompts().await?,
            ["eviction target", "evicted queued action"]
        );
        fixture.backend.shutdown().await
    }

    #[tokio::test]
    async fn inactive_authority_views_are_evicted_without_resetting_share_order() -> Result<()> {
        let fixture = Fixture::new(None, ShareRole::Control).await?;
        let session = fixture
            .backend
            .create_session(SessionOptions {
                cwd: fixture.directory.path().into(),
                title: "new after cache filled".into(),
            })
            .await?;
        {
            let mut state = fixture.backend.authority.lock().await;
            for index in 0..999 {
                state.sessions.insert(
                    format!("inactive-{index}"),
                    super::super::Observed::default(),
                );
            }
            assert_eq!(state.sessions.len(), 1000);
        }
        fixture.backend.select_session(&session.id).await?;
        let state = fixture.backend.authority.lock().await;
        assert_eq!(state.sessions.len(), 1000);
        assert!(state.sessions.contains_key(&fixture.session.id));
        assert!(state.sessions.contains_key(&session.id));
        assert!(
            state
                .share
                .as_ref()
                .is_some_and(|share| share.epoch == EPOCH)
        );
        drop(state);
        fixture.backend.shutdown().await?;
        Ok(())
    }

    #[tokio::test]
    async fn protected_authority_cache_rejects_new_entries_and_keeps_stop_available() -> Result<()>
    {
        let mut fixture =
            Fixture::new(Some(vec![gate_response("protected")]), ShareRole::Control).await?;
        fixture.backend.view_limit = 2;
        let mut run = fixture
            .backend
            .start(&fixture.session.id, "protected run".into())
            .await?;
        fixture.gate.wait_until_entered().await?;
        let selected = fixture
            .backend
            .create_session(SessionOptions {
                cwd: fixture.directory.path().into(),
                title: "selected protected view".into(),
            })
            .await?;
        fixture.backend.select_session(&selected.id).await?;
        let session = fixture
            .backend
            .create_session(SessionOptions {
                cwd: fixture.directory.path().into(),
                title: "outside protected cache".into(),
            })
            .await?;
        let error = fixture
            .backend
            .select_session(&session.id)
            .await
            .err()
            .context("protected entries were evicted")?;
        assert!(error.to_string().contains("protected"));
        let _ = fixture.backend.cancel(&fixture.session.id).await?;
        assert_eq!(wait(&mut run).await?.status, RunStatus::Cancelled);
        assert_eq!(fixture.backend.authority.lock().await.sessions.len(), 2);
        fixture.backend.shutdown().await?;
        Ok(())
    }

    #[tokio::test]
    async fn completed_runs_cannot_leave_stale_pins_in_a_full_authority_cache() -> Result<()> {
        let fixture = Fixture::new(None, ShareRole::Control).await?;
        let session = fixture
            .backend
            .create_session(SessionOptions {
                cwd: fixture.directory.path().into(),
                title: "after stale active views".into(),
            })
            .await?;
        {
            let mut state = fixture.backend.authority.lock().await;
            for index in 0..999 {
                state.sessions.insert(
                    format!("completed-{index}"),
                    super::super::Observed {
                        active: Some(format!("past-run-{index}")),
                        ..super::super::Observed::default()
                    },
                );
            }
        }
        fixture.backend.select_session(&session.id).await?;
        assert_eq!(fixture.backend.authority.lock().await.sessions.len(), 1000);
        fixture.backend.shutdown().await?;
        Ok(())
    }

    #[tokio::test]
    async fn local_cancel_uses_live_replacement_after_waiting_for_authority() -> Result<()> {
        let fixture = Fixture::new(
            Some(vec![
                gate_response("gate-first"),
                gate_response("gate-replacement"),
            ]),
            ShareRole::Control,
        )
        .await?;
        let mut first = fixture
            .backend
            .start(&fixture.session.id, "first local turn".into())
            .await?;
        fixture.gate.wait_until_entered().await?;

        let authority = fixture.backend.authority.lock().await;
        let cancel = fixture.backend.cancel(&fixture.session.id);
        tokio::pin!(cancel);
        assert!(poll_fn(|cx| Poll::Ready(cancel.as_mut().poll(cx).is_pending())).await);

        // Replace the run through the public harness without refreshing the host's cache.
        fixture
            .backend
            .harness
            .cancel_run(&fixture.session.id, &first.id)
            .await?;
        assert_eq!(wait(&mut first).await?.status, RunStatus::Cancelled);
        let mut replacement = fixture
            .backend
            .harness
            .start(&fixture.session.id, "replacement local turn".into())
            .await?;
        fixture.gate.wait_until_entered().await?;

        drop(authority);
        assert_eq!(cancel.await?.as_deref(), Some(replacement.id.as_str()));
        assert_eq!(wait(&mut replacement).await?.status, RunStatus::Cancelled);
        fixture.backend.shutdown().await
    }
}
