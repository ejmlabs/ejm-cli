use std::{collections::VecDeque, fmt::Write};

use ejm_harness::{Event, EventKind, RunStatus, RunSummary, Session, SessionView, ToolOutput};

pub use crate::ejm_tui::BlockKind;
use crate::{
    args::ProviderKind,
    commands::{self, CommandEntry, CommandPicker},
    render::safe_text,
};

const MAX_BLOCKS: usize = 256;
const MAX_DISPLAY_BYTES: usize = 256 * 1024;
const MAX_COMPOSER_BYTES: usize = 64 * 1024;

pub const LOGIN_PROVIDERS: &[(ProviderKind, &str)] = &[
    (ProviderKind::Chatgpt, "ChatGPT"),
    (ProviderKind::Claude, "Claude"),
    (ProviderKind::Openai, "OpenAI-compatible API"),
    (ProviderKind::Responses, "OpenAI Responses API"),
    (ProviderKind::Anthropic, "Anthropic API"),
];

pub struct TranscriptBlock {
    pub kind: BlockKind,
    pub title: String,
    pub text: String,
    pub call_id: Option<String>,
}

pub enum Overlay {
    Login {
        selected: usize,
    },
    Help {
        selected: usize,
    },
    Models {
        models: Vec<ejm_harness::AvailableModel>,
        selected: usize,
    },
    Sessions {
        picker: crate::sessions::SessionPicker,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunState {
    Idle,
    Running,
    Stopping,
}

pub struct RemoteBadge {
    pub label: String,
    pub controller: bool,
}

struct StopAcknowledgement {
    run_id: String,
    outcome: Option<RunStatus>,
}

pub struct App {
    pub session: Session,
    pub remote: Option<RemoteBadge>,
    pub blocks: VecDeque<TranscriptBlock>,
    pub composer: Composer,
    pub run_state: RunState,
    pub scroll_from_bottom: usize,
    pub last_sequence: u64,
    pub status: String,
    pub overlay: Option<Overlay>,
    pub dirty: bool,
    pub palette: Option<crate::extensions::Palette>,
    pub widget_lines: Vec<String>,
    pub commands: Vec<CommandEntry>,
    pub command_picker: CommandPicker,
    pub connection_ready: bool,
    pub capacity: Option<ejm_harness::CapacitySnapshot>,
    display_bytes: usize,
    received_text: bool,
    local_control_revision: u64,
    stop_acknowledgement: Option<StopAcknowledgement>,
}

impl App {
    pub fn new(session: Session) -> Self {
        let run_state = if session.status == ejm_harness::SessionStatus::Running {
            RunState::Running
        } else {
            RunState::Idle
        };
        Self {
            session,
            remote: None,
            blocks: VecDeque::new(),
            composer: Composer::default(),
            run_state,
            scroll_from_bottom: 0,
            last_sequence: 0,
            status: "Ready. Ask a question or describe a change.".into(),
            overlay: None,
            dirty: true,
            palette: None,
            widget_lines: Vec::new(),
            commands: commands::catalog(&[]),
            command_picker: CommandPicker::default(),
            connection_ready: true,
            capacity: None,
            display_bytes: 0,
            received_text: false,
            local_control_revision: 0,
            stop_acknowledgement: None,
        }
    }

    pub fn is_running(&self) -> bool {
        self.run_state != RunState::Idle
    }

    pub(crate) fn local_control_revision(&self) -> u64 {
        self.local_control_revision
    }

    pub(crate) fn record_local_control(&mut self) {
        self.local_control_revision = self.local_control_revision.wrapping_add(1);
    }

    pub(crate) fn set_local_run_state(&mut self, state: RunState) {
        self.record_local_control();
        if state == RunState::Running {
            self.stop_acknowledgement = None;
        }
        self.run_state = state;
    }

    pub(crate) fn acknowledge_stop(&mut self, run_id: Option<String>) {
        self.record_local_control();
        self.dirty = true;
        self.stop_acknowledgement = run_id.map(|run_id| StopAcknowledgement {
            run_id,
            outcome: None,
        });
        self.run_state = if self.stop_acknowledgement.is_some() {
            RunState::Stopping
        } else {
            RunState::Idle
        };
        self.status = self
            .stop_status()
            .unwrap_or("No active run in this session.")
            .into();
    }

    pub(crate) fn apply_run_snapshot(&mut self, view: &SessionView) {
        let mut stopping = self.run_state == RunState::Stopping;
        if let Some(stop) = &mut self.stop_acknowledgement {
            stopping = view.run_id.as_deref() == Some(stop.run_id.as_str());
            let transition = view.last_transition.as_ref();
            if view
                .run_id
                .as_ref()
                .or_else(|| transition.and_then(|value| value.run_id.as_ref()))
                .is_some_and(|run| run != &stop.run_id)
            {
                self.stop_acknowledgement = None;
            } else if let Some(transition) = transition
                && transition.run_id.as_deref() == Some(stop.run_id.as_str())
                && transition.outcome.is_some()
            {
                stop.outcome.clone_from(&transition.outcome);
            }
        }
        self.run_state = if view.run_id.is_none() {
            RunState::Idle
        } else if stopping {
            RunState::Stopping
        } else {
            RunState::Running
        };
    }

    pub(crate) fn stop_status(&self) -> Option<&'static str> {
        self.stop_acknowledgement
            .as_ref()
            .map(|stop| match &stop.outcome {
                None if self.run_state == RunState::Stopping => "Stopping…",
                None => "Stop requested",
                Some(RunStatus::Cancelled) => "cancelled",
                Some(RunStatus::Completed) => "completed",
                Some(RunStatus::Failed) => "failed",
                Some(RunStatus::Interrupted) => "interrupted",
                Some(RunStatus::BudgetExhausted) => "budget_exhausted",
            })
    }

    pub fn command_query(&self) -> Option<&str> {
        let query = self.composer.text.strip_prefix('/')?;
        (self.overlay.is_none()
            && !self.command_picker.dismissed
            && self.composer.cursor == self.composer.text.len()
            && !query.contains(char::is_whitespace))
        .then_some(query)
    }

    pub fn command_matches(&self) -> Vec<&CommandEntry> {
        self.command_query().map_or_else(Vec::new, |query| {
            self.commands
                .iter()
                .filter(|entry| entry.name.starts_with(query))
                .collect()
        })
    }

    pub fn complete_command(&mut self) {
        let completion = self
            .command_matches()
            .get(self.command_picker.selected)
            .map(|command| command.completion());
        if let Some(completion) = completion {
            self.composer.replace(&completion);
            self.command_picker.dismissed = true;
        }
    }

    pub fn switch_session(&mut self, mut next: Self) {
        if self.session.id == next.session.id {
            next.local_control_revision = self.local_control_revision;
            next.stop_acknowledgement = self.stop_acknowledgement.take();
        }
        next.commands = std::mem::take(&mut self.commands);
        next.palette = self.palette;
        next.connection_ready = self.connection_ready;
        next.capacity = self.capacity.take();
        next.widget_lines = std::mem::take(&mut self.widget_lines);
        next.composer = std::mem::take(&mut self.composer);
        *self = next;
    }

    pub fn notice(&mut self, message: impl Into<String>) {
        self.push(BlockKind::Notice, "ejm-cli", &message.into(), None);
    }

    pub fn error(&mut self, message: impl Into<String>) {
        self.push(BlockKind::Error, "error", &message.into(), None);
    }

    pub fn apply(&mut self, event: Event) {
        if event.session_id != self.session.id || event.sequence <= self.last_sequence {
            return;
        }
        self.last_sequence = event.sequence;
        self.dirty = true;
        match event.kind {
            EventKind::RunStarted { prompt } => {
                if self
                    .stop_acknowledgement
                    .as_ref()
                    .is_some_and(|stop| event.run_id.as_deref() != Some(stop.run_id.as_str()))
                {
                    self.stop_acknowledgement = None;
                }
                self.run_state = RunState::Running;
                self.received_text = false;
                self.status = "Thinking".into();
                self.push(BlockKind::User, "you", &prompt, None);
            }
            EventKind::ModelStarted { step } => {
                self.status = format!("Thinking · step {step}");
            }
            EventKind::TextDelta { text } => {
                self.received_text = true;
                let text = safe_text(&text);
                if let Some(block) = self.blocks.back_mut()
                    && block.kind == BlockKind::Assistant
                {
                    block.text.push_str(&text);
                    self.display_bytes += text.len();
                    self.trim();
                } else {
                    self.push(BlockKind::Assistant, "ejm-cli", &text, None);
                }
                self.status = "Responding".into();
            }
            EventKind::ToolQueued { call } => {
                self.push(
                    BlockKind::Tool,
                    &format!("queued · {}", call.name),
                    &brief_arguments(&call.arguments),
                    Some(call.id),
                );
                self.status = "Waiting for tool capacity".into();
            }
            EventKind::ToolStarted { call } => {
                if let Some(block) = self
                    .blocks
                    .iter_mut()
                    .rev()
                    .find(|block| block.call_id.as_deref() == Some(call.id.as_str()))
                {
                    block.title = format!("running · {}", safe_text(&call.name));
                } else {
                    self.push(
                        BlockKind::Tool,
                        &format!("running · {}", call.name),
                        &brief_arguments(&call.arguments),
                        Some(call.id),
                    );
                }
                self.status = format!("Running {}", safe_text(&call.name));
            }
            EventKind::ToolFinished { call_id, output } => self.finish_tool(&call_id, output),
            EventKind::RunFinished { summary } => self.finish_run(summary),
            EventKind::Steered { text } => {
                self.push(BlockKind::Notice, "steering", &text, None);
            }
            EventKind::Queued { text } => {
                self.push(BlockKind::Notice, "queued message", &text, None);
            }
            EventKind::Progress { message, .. } => {
                self.status = safe_text(&message);
            }
            EventKind::ContextTrimmed {
                before_bytes,
                after_bytes,
            } => {
                self.notice(format!(
                    "Context trimmed: {before_bytes} → {after_bytes} bytes"
                ));
            }
            EventKind::Usage { .. } => {}
        }
        if self
            .stop_acknowledgement
            .as_ref()
            .is_some_and(|stop| stop.outcome.is_none())
        {
            self.run_state = RunState::Stopping;
            self.status = "Stopping…".into();
        }
    }

    pub(crate) fn apply_saved(&mut self, event: Event) {
        // Saved output cannot change current authority or a newer local Stop.
        let run_state = self.run_state;
        let status = std::mem::take(&mut self.status);
        let stop = self.stop_acknowledgement.take();
        self.apply(event);
        self.run_state = run_state;
        self.status = status;
        self.stop_acknowledgement = stop;
    }

    fn finish_run(&mut self, summary: RunSummary) {
        if let Some(stop) = &mut self.stop_acknowledgement
            && stop.run_id == summary.run_id
        {
            stop.outcome = Some(summary.status.clone());
        }
        self.run_state = RunState::Idle;
        if !self.received_text && !summary.text.is_empty() {
            self.push(BlockKind::Assistant, "ejm-cli", &summary.text, None);
        }
        self.status = format!(
            "{} · {} steps · {} in / {} out tokens",
            serde_json::to_value(&summary.status)
                .ok()
                .and_then(|value| value.as_str().map(str::to_owned))
                .unwrap_or_else(|| "finished".into()),
            summary.steps,
            summary.usage.input_tokens,
            summary.usage.output_tokens,
        );
        if let Some(error) = summary.error {
            self.error(error);
        }
    }

    fn finish_tool(&mut self, call_id: &str, output: ToolOutput) {
        let status = if output.is_error { "failed" } else { "done" };
        let mut content = output.content;
        if content.is_empty() && !output.data.is_null() {
            content = serde_json::to_string_pretty(&output.data).unwrap_or_default();
        }
        if let Some(artifact) = output.artifact {
            let _ = write!(content, "\nFull output: {}", artifact.display());
        }
        // Keep tool details useful while bounding a single very noisy result.
        truncate_start(&mut content, 24 * 1024);
        let content = safe_text(&content);
        if let Some(block) = self
            .blocks
            .iter_mut()
            .rev()
            .find(|block| block.call_id.as_deref() == Some(call_id))
        {
            let tool = block.title.split(" · ").last().unwrap_or("tool");
            block.title = format!("{status} · {tool}");
            if output.is_error {
                block.kind = BlockKind::Error;
            }
            self.display_bytes += content.len() + 1;
            block.text.push('\n');
            block.text.push_str(&content);
            self.trim();
        } else {
            self.push(
                if output.is_error {
                    BlockKind::Error
                } else {
                    BlockKind::Tool
                },
                status,
                &content,
                None,
            );
        }
    }

    fn push(&mut self, kind: BlockKind, title: &str, text: &str, call_id: Option<String>) {
        let mut text = safe_text(text);
        truncate_start(&mut text, MAX_DISPLAY_BYTES / 2);
        self.display_bytes += text.len();
        self.blocks.push_back(TranscriptBlock {
            kind,
            title: safe_text(title),
            text,
            call_id,
        });
        self.dirty = true;
        self.trim();
    }

    fn trim(&mut self) {
        while self.blocks.len() > MAX_BLOCKS || self.display_bytes > MAX_DISPLAY_BYTES {
            if self.blocks.len() <= 1 {
                let Some(block) = self.blocks.front_mut() else {
                    break;
                };
                truncate_start(&mut block.text, MAX_DISPLAY_BYTES / 2);
                self.display_bytes = block.text.len();
                break;
            }
            let Some(block) = self.blocks.pop_front() else {
                break;
            };
            self.display_bytes -= block.text.len();
        }
    }
}

fn brief_arguments(arguments: &serde_json::Value) -> String {
    let mut arguments = serde_json::to_string(arguments).unwrap_or_default();
    truncate_end(&mut arguments, 4 * 1024);
    arguments
}

pub fn truncate_start(text: &mut String, limit: usize) {
    if text.len() <= limit {
        return;
    }
    let mut start = text.len() - limit;
    while !text.is_char_boundary(start) {
        start += 1;
    }
    text.drain(..start);
    text.insert_str(0, "[… earlier output omitted …]\n");
}

fn truncate_end(text: &mut String, limit: usize) {
    if text.len() <= limit {
        return;
    }
    let mut end = limit;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text.truncate(end);
    text.push_str("\n[…]");
}

#[derive(Default)]
pub struct Composer {
    pub text: String,
    pub cursor: usize,
    history: VecDeque<String>,
    history_index: Option<usize>,
    draft: String,
}

impl Composer {
    pub fn replace(&mut self, input: &str) {
        self.text.clear();
        self.cursor = 0;
        self.insert(input);
    }

    pub fn insert(&mut self, input: &str) {
        let input = safe_text(input);
        let available = MAX_COMPOSER_BYTES.saturating_sub(self.text.len());
        let mut end = available.min(input.len());
        while !input.is_char_boundary(end) {
            end -= 1;
        }
        self.text.insert_str(self.cursor, &input[..end]);
        self.cursor += end;
        self.history_index = None;
    }

    pub fn backspace(&mut self) {
        if let Some((index, _)) = self.text[..self.cursor].char_indices().next_back() {
            self.text.drain(index..self.cursor);
            self.cursor = index;
        }
    }

    pub fn delete(&mut self) {
        if let Some(character) = self.text[self.cursor..].chars().next() {
            self.text
                .drain(self.cursor..self.cursor + character.len_utf8());
        }
    }

    pub fn left(&mut self) {
        if let Some((index, _)) = self.text[..self.cursor].char_indices().next_back() {
            self.cursor = index;
        }
    }

    pub fn right(&mut self) {
        if let Some(character) = self.text[self.cursor..].chars().next() {
            self.cursor += character.len_utf8();
        }
    }

    pub fn home(&mut self) {
        self.cursor = self.text[..self.cursor].rfind('\n').map_or(0, |i| i + 1);
    }

    pub fn end(&mut self) {
        self.cursor = self.text[self.cursor..]
            .find('\n')
            .map_or(self.text.len(), |i| self.cursor + i);
    }

    pub fn take(&mut self) -> String {
        let text = std::mem::take(&mut self.text);
        self.cursor = 0;
        self.history_index = None;
        self.draft.clear();
        if !text.trim().is_empty() && self.history.back() != Some(&text) {
            self.history.push_back(text.clone());
            while self.history.len() > 100 {
                self.history.pop_front();
            }
        }
        text
    }

    pub fn history(&mut self, previous: bool) {
        if self.history.is_empty() {
            return;
        }
        if previous {
            let index = match self.history_index {
                None => {
                    self.draft = self.text.clone();
                    self.history.len() - 1
                }
                Some(index) => index.saturating_sub(1),
            };
            self.history_index = Some(index);
            self.text.clone_from(&self.history[index]);
        } else if let Some(index) = self.history_index {
            if index + 1 < self.history.len() {
                self.history_index = Some(index + 1);
                self.text.clone_from(&self.history[index + 1]);
            } else {
                self.history_index = None;
                self.text.clone_from(&self.draft);
            }
        }
        self.cursor = self.text.len();
    }
}

#[cfg(test)]
#[expect(
    clippy::unwrap_used,
    reason = "Fixture setup should fail this test immediately when invalid."
)]
mod tests {
    use super::*;

    fn app() -> App {
        App::new(
            serde_json::from_value(serde_json::json!({
                "id": "test", "title": "test", "cwd": "/tmp", "created_at_ms": 0,
                "status": "idle"
            }))
            .unwrap(),
        )
    }

    #[test]
    fn command_prefix_completion_preserves_the_conversation() {
        let mut app = app();
        app.composer.insert("/mod");
        assert_eq!(app.command_query(), Some("mod"));
        assert_eq!(
            app.command_matches()
                .iter()
                .map(|entry| entry.name.as_str())
                .collect::<Vec<_>>(),
            ["model", "models"]
        );

        app.complete_command();
        assert_eq!(app.composer.text, "/model ");
        assert_eq!(app.composer.cursor, app.composer.text.len());
        assert!(app.command_query().is_none());
        assert!(app.blocks.is_empty());
        assert_eq!(app.run_state, RunState::Idle);

        app.composer.replace("/mod");
        app.command_picker = CommandPicker {
            selected: 1,
            dismissed: false,
        };
        app.complete_command();
        assert_eq!(app.composer.text, "/models");
        assert!(app.command_picker.dismissed);
    }

    #[test]
    fn command_dismissal_and_noncommand_drafts_keep_text_editable() {
        let mut app = app();
        app.composer.insert("/mod");
        app.command_picker.dismissed = true;
        assert!(app.command_query().is_none());
        assert_eq!(app.composer.text, "/mod");

        app.command_picker = CommandPicker::default();
        app.composer.left();
        assert!(app.command_query().is_none());
        app.composer.right();
        assert_eq!(app.command_query(), Some("mod"));
        for draft in [
            "ordinary text",
            " /mod",
            "/model sample",
            "/mod\nsecond line",
        ] {
            app.composer.replace(draft);
            assert!(app.command_query().is_none(), "{draft}");
            assert_eq!(app.composer.text, draft);
        }

        app.composer.replace("/does-not-exist");
        app.complete_command();
        assert_eq!(app.composer.text, "/does-not-exist");
        assert!(app.command_matches().is_empty());
    }

    #[test]
    fn replacement_keeps_unicode_and_composer_bounds() {
        let mut composer = Composer::default();
        composer.insert("sent message");
        composer.take();
        composer.replace(&format!(
            "\x1b[31m{}\x1b[0m",
            "界".repeat(MAX_COMPOSER_BYTES)
        ));
        assert!(composer.text.len() <= MAX_COMPOSER_BYTES);
        assert!(composer.text.chars().all(|character| character == '界'));
        assert_eq!(composer.cursor, composer.text.len());
        let draft = composer.text.clone();
        composer.history(true);
        assert_eq!(composer.text, "sent message");
        composer.history(false);
        assert_eq!(composer.text, draft);
    }

    #[test]
    fn edits_unicode_without_splitting_codepoints_and_recovers_draft() {
        let mut composer = Composer::default();
        composer.insert("a🦀界");
        composer.left();
        composer.backspace();
        assert_eq!(composer.text, "a界");
        composer.delete();
        assert_eq!(composer.text, "a");
        assert_eq!(composer.take(), "a");
        composer.insert("draft");
        composer.history(true);
        assert_eq!(composer.text, "a");
        composer.history(false);
        assert_eq!(composer.text, "draft");
    }

    #[test]
    fn transcript_and_single_streaming_block_are_bounded() {
        let session: Session = serde_json::from_value(serde_json::json!({
            "id": "test", "title": "test", "cwd": "/tmp", "created_at_ms": 0,
            "status": "idle"
        }))
        .unwrap();
        let mut app = App::new(session);
        for _ in 0..1000 {
            app.notice("x".repeat(1024));
        }
        assert!(app.blocks.len() <= MAX_BLOCKS);
        assert!(app.display_bytes <= MAX_DISPLAY_BYTES + 64);
        let mut unicode = "🦀".repeat(MAX_DISPLAY_BYTES);
        truncate_start(&mut unicode, 100);
        assert!(unicode.len() < 150);
    }
}
