//! Native controller contracts use an offline session and bounded in-memory channels.
#[cfg(test)]
pub(in crate::remote::controller) mod contracts {
    #![expect(
        clippy::unwrap_used,
        reason = "Invalid deterministic fixtures fail the test immediately."
    )]
    use super::super::super::protocol::{GapReason, RemoteError, ReplayGap};
    use super::super::*;
    use ejm_harness::{EventKind, SessionStatus};

    pub(in crate::remote::controller) fn fixture(
        role: ShareRole,
    ) -> (App, Controls, RemoteBackend, mpsc::Receiver<Command>) {
        let session = Session {
            id: "session-fixture".into(),
            title: "fixture".into(),
            cwd: "/tmp".into(),
            created_at_ms: 0,
            status: SessionStatus::Idle,
        };
        let mut app = App::new(session.clone());
        app.connection_ready = true;
        let (commands, receiver) = mpsc::channel(1);
        let backend = RemoteBackend {
            commands,
            current: Arc::new(RwLock::new(CurrentSession {
                session,
                active_run: None,
                mutation_high_water: 0,
            })),
            cursor: Arc::new(AtomicU64::new(0)),
            cursor_changed: Arc::new(Notify::new()),
        };
        (
            app,
            Controls {
                role,
                active: None,
                sequence: 0,
                pending: None,
            },
            backend,
            receiver,
        )
    }

    #[tokio::test]
    async fn pending_and_rejected_actions_preserve_drafts_without_resubmission() {
        let (mut app, mut controls, backend, mut receiver) = fixture(ShareRole::Control);
        app.composer.replace("first private prompt");
        assert!(!controls.enter(&mut app, &backend).unwrap());
        let mutation = match receiver.recv().await {
            Some(Command::Mutation(mutation)) => Some(mutation),
            _ => None,
        }
        .unwrap();
        assert_eq!(mutation.sequence, 1);
        assert_eq!(app.composer.text, "");
        controls.outcome(1, MutationOutcome::Pending, &mut app);
        assert!(controls.pending.is_some());
        assert!(
            receiver.try_recv().is_err(),
            "pending results must never resubmit actions"
        );
        controls.outcome(
            1,
            MutationOutcome::Rejected {
                code: RemoteError::StaleState,
                message: "stale".into(),
            },
            &mut app,
        );
        assert_eq!(app.composer.text, "first private prompt");
        assert!(controls.pending.is_none());
        controls.enter(&mut app, &backend).unwrap();
        assert!(matches!(receiver.recv().await, Some(Command::Mutation(_))));
        app.composer.replace("new unsent draft");
        controls.outcome(2, MutationOutcome::Unknown, &mut app);
        assert_eq!(app.composer.text, "new unsent draft");
        assert!(receiver.try_recv().is_err());
    }

    #[tokio::test]
    async fn view_or_unavailable_transport_retains_input_and_status_only_reads() {
        let (mut app, mut controls, backend, mut receiver) = fixture(ShareRole::View);
        app.composer.replace("cannot execute");
        assert!(controls.enter(&mut app, &backend).is_err());
        assert_eq!(app.composer.text, "cannot execute");
        assert!(receiver.try_recv().is_err());
        controls.role = ShareRole::Control;
        app.connection_ready = false;
        assert!(controls.enter(&mut app, &backend).is_err());
        assert_eq!(app.composer.text, "cannot execute");
        app.composer.replace("/status");
        controls.enter(&mut app, &backend).unwrap();
        assert!(matches!(receiver.recv().await, Some(Command::Refresh)));
        assert!(controls.pending.is_none());
        drop(receiver);
        app.connection_ready = true;
        app.composer.replace("keep this after send failure");
        assert!(controls.enter(&mut app, &backend).is_err());
        assert_eq!(app.composer.text, "keep this after send failure");
        assert!(controls.pending.is_none());
    }

    #[tokio::test]
    async fn returning_controller_uses_host_high_water_without_relabeling_pending_actions() {
        let (mut app, mut controls, backend, mut receiver) = fixture(ShareRole::Control);
        let snapshot = |high_water| ServerMessage::Snapshot {
            session: app.session.clone(),
            events: vec![],
            through_sequence: 0,
            gap: None,
            active_run: None,
            role: ShareRole::Control,
            mutation_high_water: high_water,
        };
        let resumed = snapshot(41);
        let newer = snapshot(44);
        let older = snapshot(3);
        let exhausted = snapshot(MAX_MUTATION_SEQUENCE);
        apply_update(resumed, &mut app, &mut controls);
        app.composer.replace("new prompt after process restart");
        controls.enter(&mut app, &backend).unwrap();
        assert!(matches!(
            receiver.recv().await,
            Some(Command::Mutation(Mutation { sequence: 42, .. }))
        ));
        apply_update(newer, &mut app, &mut controls);
        apply_update(older, &mut app, &mut controls);
        assert_eq!(controls.sequence, 44);
        assert_eq!(
            controls.pending.as_ref().map(|pending| pending.sequence),
            Some(42)
        );
        controls.outcome(42, MutationOutcome::Unknown, &mut app);
        assert_eq!(app.composer.text, "new prompt after process restart");
        controls.enter(&mut app, &backend).unwrap();
        assert!(matches!(
            receiver.recv().await,
            Some(Command::Mutation(Mutation { sequence: 45, .. }))
        ));
        controls.outcome(45, MutationOutcome::Accepted { run_id: None }, &mut app);
        apply_update(exhausted, &mut app, &mut controls);
        app.composer.replace("retain at sequence limit");
        assert!(controls.enter(&mut app, &backend).is_err());
        assert_eq!(app.composer.text, "retain at sequence limit");
        assert!(receiver.try_recv().is_err());
    }

    #[test]
    fn replay_batches_keep_current_authority_and_allow_input_between_frames() {
        let (mut app, mut controls, _, _) = fixture(ShareRole::Control);
        let events = (1..=40)
            .map(|sequence| HarnessEvent {
                sequence,
                session_id: app.session.id.clone(),
                run_id: Some("old-run".into()),
                timestamp_ms: 0,
                kind: if sequence == 1 {
                    EventKind::RunStarted {
                        prompt: "past turn".into(),
                    }
                } else {
                    EventKind::TextDelta {
                        text: "saved".into(),
                    }
                },
            })
            .collect();
        let session = app.session.clone();
        let mut batch = DisplayBatch::begin(
            ServerMessage::Snapshot {
                session,
                events,
                through_sequence: 40,
                gap: None,
                active_run: Some("current-run".into()),
                role: ShareRole::Control,
                mutation_high_water: 0,
            },
            &mut app,
            &mut controls,
        )
        .unwrap();
        assert!(!batch.apply(&mut app, &mut controls));
        assert!(app.last_sequence > 0 && app.last_sequence < 40);
        assert_eq!(controls.active.as_deref(), Some("current-run"));
        app.composer.insert("input during recovery");
        while !batch.apply(&mut app, &mut controls) {}
        assert_eq!(app.last_sequence, 40);
        assert_eq!(app.composer.text, "input during recovery");
        assert_eq!(controls.active.as_deref(), Some("current-run"));
    }

    #[test]
    fn historical_replay_and_late_results_cannot_replace_the_current_run() {
        let (mut app, mut controls, _, _) = fixture(ShareRole::Control);
        let old = HarnessEvent {
            sequence: 1,
            session_id: app.session.id.clone(),
            run_id: Some("old-run".into()),
            timestamp_ms: 0,
            kind: EventKind::RunStarted {
                prompt: "\u{1b}]52;c;injected\u{7}".into(),
            },
        };
        let session = app.session.clone();
        apply_update(
            ServerMessage::Snapshot {
                session,
                events: vec![old.clone()],
                through_sequence: 1,
                gap: None,
                active_run: Some("current-run".into()),
                role: ShareRole::Control,
                mutation_high_water: 0,
            },
            &mut app,
            &mut controls,
        );
        assert_eq!(controls.active.as_deref(), Some("current-run"));
        apply_events(vec![old], 1, false, &mut app, &mut controls.active);
        assert_eq!(controls.active.as_deref(), Some("current-run"));
        controls.outcome(
            1,
            MutationOutcome::Accepted {
                run_id: Some("old-run".into()),
            },
            &mut app,
        );
        assert_eq!(controls.active.as_deref(), Some("current-run"));
        apply_update(
            ServerMessage::Events {
                events: vec![],
                through_sequence: 900,
                gap: Some(ReplayGap {
                    requested_after_sequence: 1,
                    available_after_sequence: 900,
                    reason: GapReason::OutputTruncated,
                }),
            },
            &mut app,
            &mut controls,
        );
        assert_eq!(app.last_sequence, 900);
        assert_eq!(controls.active.as_deref(), Some("current-run"));
    }
}
