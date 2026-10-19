//! A slow display must recover retained output before it accepts newer events.
#[cfg(test)]
mod contracts {
    #![expect(
        clippy::unwrap_used,
        reason = "Invalid deterministic fixtures fail the test immediately."
    )]

    use super::super::*;
    use crate::app::App;
    use crate::remote::{
        controller::{Controls, apply_update},
        invitation::ShareRole,
    };
    use ejm_harness::{Event, EventKind, Session, SessionStatus};
    use std::fmt::Write;

    struct Fixture {
        display: Display,
        receiver: mpsc::Receiver<Update>,
        app: App,
        controls: Controls,
    }

    impl Fixture {
        fn new() -> Self {
            let (updates, receiver) = mpsc::channel(16);
            let mut display = Display::new(updates);
            display.ready = true;
            Self {
                display,
                receiver,
                app: App::new(session()),
                controls: Controls {
                    role: ShareRole::Control,
                    active: None,
                    sequence: 0,
                    pending: None,
                },
            }
        }

        async fn saturate(&mut self) {
            for sequence in 1..=17 {
                self.display.publish(live(sequence)).await.unwrap();
            }
            self.drain();
            assert_eq!(self.app.last_sequence, 16);
            assert!(self.display.needs_snapshot(self.app.last_sequence));
        }

        fn drain(&mut self) {
            while let Ok(update) = self.receiver.try_recv() {
                let message = match update {
                    Update::Server(message) => Some(message),
                    _ => None,
                }
                .unwrap();
                apply_update(message, &mut self.app, &mut self.controls);
            }
        }

        fn assert_output(&self, through: u64) {
            assert_eq!(self.app.last_sequence, through);
            let mut expected = String::new();
            for sequence in 1..=through {
                write!(expected, "{sequence};").unwrap();
            }
            let actual: String = self
                .app
                .blocks
                .iter()
                .map(|block| block.text.as_str())
                .collect();
            assert_eq!(actual, expected);
        }
    }

    fn session() -> Session {
        Session {
            id: "slow-display".into(),
            title: "fixture".into(),
            cwd: "/tmp".into(),
            created_at_ms: 0,
            status: SessionStatus::Idle,
        }
    }

    fn event(sequence: u64) -> Event {
        Event {
            sequence,
            session_id: "slow-display".into(),
            run_id: None,
            timestamp_ms: 0,
            kind: EventKind::TextDelta {
                text: format!("{sequence};"),
            },
        }
    }

    fn live(sequence: u64) -> ServerMessage {
        ServerMessage::Events {
            events: vec![event(sequence)],
            through_sequence: sequence,
            gap: None,
        }
    }

    fn snapshot(first: u64, through: u64) -> ServerMessage {
        ServerMessage::Snapshot {
            session: session(),
            events: (first..=through).map(event).collect(),
            through_sequence: through,
            gap: None,
            active_run: None,
            role: ShareRole::Control,
            mutation_high_water: 0,
        }
    }

    #[tokio::test]
    async fn late_live_events_cannot_skip_a_dropped_batch_or_end_short_page_recovery() {
        let mut fixture = Fixture::new();
        fixture.saturate().await;
        fixture.display.publish(live(18)).await.unwrap();
        fixture.drain();
        fixture.assert_output(16);

        fixture.display.publish(snapshot(17, 17)).await.unwrap();
        assert!(!fixture.display.needs_snapshot(fixture.app.last_sequence));
        fixture.drain();
        fixture.assert_output(17);
        assert!(fixture.display.needs_snapshot(fixture.app.last_sequence));

        fixture.display.publish(live(19)).await.unwrap();
        fixture.display.publish(snapshot(18, 18)).await.unwrap();
        fixture.drain();
        fixture.assert_output(18);
        assert!(fixture.display.needs_snapshot(fixture.app.last_sequence));

        fixture.display.publish(snapshot(19, 19)).await.unwrap();
        fixture.drain();
        assert!(!fixture.display.needs_snapshot(fixture.app.last_sequence));
        fixture.display.publish(live(20)).await.unwrap();
        fixture.drain();
        fixture.assert_output(20);
    }

    #[tokio::test]
    async fn full_pages_and_stale_snapshots_keep_the_barrier_until_ordered_catch_up() {
        let mut fixture = Fixture::new();
        fixture.saturate().await;
        let page = u64::try_from(MAX_REPLAY_EVENTS).unwrap();
        let first_end = 16 + page;
        let second_end = first_end + page;
        fixture.display.publish(live(second_end)).await.unwrap();
        fixture
            .display
            .publish(snapshot(17, first_end))
            .await
            .unwrap();
        assert!(!fixture.display.needs_snapshot(fixture.app.last_sequence));
        fixture.drain();
        fixture.assert_output(first_end);
        assert!(fixture.display.needs_snapshot(fixture.app.last_sequence));

        // This late reply predates both the dropped batch and the acknowledged page.
        fixture.display.publish(snapshot(1, 16)).await.unwrap();
        fixture.drain();
        fixture.assert_output(first_end);
        assert!(fixture.display.needs_snapshot(fixture.app.last_sequence));

        fixture
            .display
            .publish(snapshot(first_end + 1, second_end))
            .await
            .unwrap();
        fixture.drain();
        fixture.assert_output(second_end);
        assert!(fixture.display.needs_snapshot(fixture.app.last_sequence));

        // A full page requires another read even when it covers the known watermark.
        fixture.display.publish(live(second_end + 1)).await.unwrap();
        fixture
            .display
            .publish(snapshot(second_end + 1, second_end))
            .await
            .unwrap();
        fixture.drain();
        fixture.assert_output(second_end);
        assert!(fixture.display.needs_snapshot(fixture.app.last_sequence));

        fixture
            .display
            .publish(snapshot(second_end + 1, second_end + 1))
            .await
            .unwrap();
        fixture.drain();
        assert!(!fixture.display.needs_snapshot(fixture.app.last_sequence));
        fixture.display.publish(live(second_end + 2)).await.unwrap();
        fixture.drain();
        fixture.assert_output(second_end + 2);
    }
}
