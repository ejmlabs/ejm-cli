//! Paged session-picker state. Only the visible page is retained.
use std::{
    collections::VecDeque,
    sync::atomic::{AtomicU64, Ordering},
};

use anyhow::Result;
use ejm_harness::{Session, SessionCursor, SessionPage, SessionQuery};
use tokio::task::{JoinError, JoinSet};

use crate::{
    app::{App, Overlay},
    remote::backend::LocalBackend,
};

static GENERATION: AtomicU64 = AtomicU64::new(1);

pub struct SessionPicker {
    pub sessions: Vec<Session>,
    pub selected: usize,
    pub query: String,
    pub pending: bool,
    pub next: Option<SessionCursor>,
    generation: u64,
    cursor: Option<SessionCursor>,
    history: VecDeque<Option<SessionCursor>>,
    select_last: bool,
}

impl Default for SessionPicker {
    fn default() -> Self {
        Self {
            sessions: Vec::new(),
            selected: 0,
            query: String::new(),
            pending: true,
            next: None,
            generation: GENERATION.fetch_add(1, Ordering::Relaxed),
            cursor: None,
            history: VecDeque::new(),
            select_last: false,
        }
    }
}

impl SessionPicker {
    fn changed(&mut self) {
        self.generation = GENERATION.fetch_add(1, Ordering::Relaxed);
        self.pending = true;
    }

    pub fn query_changed(&mut self) {
        self.cursor = None;
        self.history.clear();
        self.sessions.clear();
        self.next = None;
        self.selected = 0;
        self.select_last = false;
        self.changed();
    }

    pub fn next_page(&mut self) {
        if self.pending {
            return;
        }
        if let Some(next) = self.next.clone() {
            if self.history.len() == 32 {
                self.history.pop_front();
            }
            self.history.push_back(self.cursor.take());
            self.cursor = Some(next);
            self.select_last = false;
            self.changed();
        }
    }

    pub fn previous_page(&mut self) {
        if self.pending {
            return;
        }
        if let Some(previous) = self.history.pop_back() {
            self.cursor = previous;
            self.select_last = true;
            self.changed();
        }
    }

    pub fn first_page(&mut self) {
        self.cursor = None;
        self.history.clear();
        self.select_last = false;
        self.changed();
    }
}

#[derive(Default)]
pub struct SessionPages {
    pub jobs: JoinSet<(u64, Result<SessionPage>)>,
    generation: Option<u64>,
}

impl SessionPages {
    pub fn joined(reply: Option<Result<(u64, Result<SessionPage>), JoinError>>, app: &mut App) {
        match reply {
            Some(Ok((generation, result))) => Self::loaded(generation, result, app),
            Some(Err(error)) if !error.is_cancelled() => {
                app.error(format!("List sessions: {error}"));
            }
            _ => {}
        }
    }

    pub fn follow(&mut self, harness: &LocalBackend, app: &App) {
        let Some(Overlay::Sessions { picker }) = &app.overlay else {
            self.jobs = JoinSet::new();
            self.generation = None;
            return;
        };
        if self.generation == Some(picker.generation) {
            return;
        }
        self.jobs = JoinSet::new();
        let generation = picker.generation;
        self.generation = Some(generation);
        let query = SessionQuery {
            query: picker.query.clone(),
            cursor: picker.cursor.clone(),
            limit: 50,
        };
        let harness = harness.clone();
        self.jobs
            .spawn(async move { (generation, harness.sessions_page(query).await) });
    }

    pub fn loaded(generation: u64, result: Result<SessionPage>, app: &mut App) {
        let Some(Overlay::Sessions { picker }) = &mut app.overlay else {
            return;
        };
        if picker.generation != generation {
            return;
        }
        picker.pending = false;
        let page = match result {
            Ok(page) => page,
            Err(error) => {
                app.error(format!(
                    "List sessions: {error:#}. Press Home or change the search to retry."
                ));
                return;
            }
        };
        picker.selected = if picker.select_last {
            page.sessions.len().saturating_sub(1)
        } else {
            0
        };
        picker.sessions = page.sessions;
        picker.next = page.next;
        app.dirty = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::{Context, Result};
    use ejm_harness::{Harness, HarnessConfig, ReplayProvider, SessionOptions};
    use std::sync::Arc;

    #[tokio::test]
    async fn stale_query_completion_cannot_replace_current_picker_or_draft() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let harness = Harness::new(
            HarnessConfig {
                state_dir: directory.path().join("sessions"),
                ..HarnessConfig::default()
            },
            Arc::new(ReplayProvider::echo()),
            Vec::new(),
        )?;
        let session = harness
            .create_session(SessionOptions {
                cwd: directory.path().into(),
                title: "original".into(),
            })
            .await?;
        let backend = LocalBackend::new(harness);
        let mut app = App::new(session);
        app.composer.insert("keep draft");
        app.overlay = Some(Overlay::Sessions {
            picker: SessionPicker::default(),
        });
        let mut pages = SessionPages::default();
        pages.follow(&backend, &app);
        let (generation, page) = pages
            .jobs
            .join_next()
            .await
            .context("picker task missing")??;
        let Some(Overlay::Sessions { picker }) = &mut app.overlay else {
            anyhow::bail!("picker missing");
        };
        picker.query = "different".into();
        picker.query_changed();
        SessionPages::loaded(generation, page, &mut app);
        let Some(Overlay::Sessions { picker }) = &app.overlay else {
            anyhow::bail!("picker missing");
        };
        assert!(picker.sessions.is_empty());
        assert!(picker.pending);
        assert_eq!(app.composer.text, "keep draft");
        backend.shutdown().await?;
        Ok(())
    }
}
