use std::{collections::VecDeque, fmt::Write, time::Instant};

use ejm_harness::{Event, EventKind, Session, ToolOutput};

pub use ejm_tui::ToolCallState as ToolState;
use ejm_tui::compact_count;

use crate::{
    commands::{self, CommandEntry, CommandPicker},
    render::safe_text,
};

const MAX_BLOCKS: usize = 256;
const MAX_DISPLAY_BYTES: usize = 256 * 1024;
const MAX_COMPOSER_BYTES: usize = 64 * 1024;

pub struct TranscriptBlock {
    pub kind: BlockKind,
    pub title: String,
    pub text: String,
    pub call_id: Option<String>,
    /// Set for tool calls. `text` then holds only the tool output.
    pub tool: Option<ToolView>,
}

/// What a transcript block is. Tool calls also carry a [`ToolView`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockKind {
    User,
    Assistant,
    Tool,
    Notice,
    Error,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolDetail {
    /// A few lines per call, with a count of the rest.
    Preview,
    /// Every retained line.
    Full,
}

impl ToolDetail {
    #[must_use]
    pub fn toggled(self) -> Self {
        match self {
            Self::Preview => Self::Full,
            Self::Full => Self::Preview,
        }
    }
}

pub struct ToolView {
    pub name: String,
    /// The most telling argument, such as a path, pattern or command.
    pub detail: String,
    pub state: ToolState,
}

pub enum Overlay {
    Help {
        selected: usize,
    },
    Sessions {
        sessions: Vec<Session>,
        selected: usize,
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
    /// When the active run started, for the elapsed time beside the spinner.
    pub run_started: Option<Instant>,
    /// How much tool output the transcript shows. Toggled with Ctrl+O.
    pub tool_detail: ToolDetail,
    /// Set by Ctrl+C on an empty, idle composer. A second press soon after exits.
    pub quit_armed: Option<Instant>,
    display_bytes: usize,
    received_text: bool,
}

impl App {
    pub fn new(session: Session) -> Self {
        Self {
            session,
            remote: None,
            blocks: VecDeque::new(),
            composer: Composer::default(),
            run_state: RunState::Idle,
            scroll_from_bottom: 0,
            last_sequence: 0,
            status: "Ready".into(),
            overlay: None,
            dirty: true,
            palette: None,
            widget_lines: Vec::new(),
            commands: commands::catalog(&[]),
            command_picker: CommandPicker::default(),
            connection_ready: true,
            run_started: None,
            tool_detail: ToolDetail::Preview,
            quit_armed: None,
            display_bytes: 0,
            received_text: false,
        }
    }

    pub fn is_running(&self) -> bool {
        self.run_state != RunState::Idle
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
        next.commands = std::mem::take(&mut self.commands);
        next.palette = self.palette;
        next.connection_ready = self.connection_ready;
        next.tool_detail = self.tool_detail;
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
                self.run_state = RunState::Running;
                self.run_started = Some(Instant::now());
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
                self.push_tool(&call, ToolState::Queued);
                self.status = "Waiting for tool capacity".into();
            }
            EventKind::ToolStarted { call } => {
                if let Some(view) = self
                    .blocks
                    .iter_mut()
                    .rev()
                    .find(|block| block.call_id.as_deref() == Some(call.id.as_str()))
                    .and_then(|block| block.tool.as_mut())
                {
                    view.state = ToolState::Running;
                } else {
                    self.push_tool(&call, ToolState::Running);
                }
                self.status = format!("Running {}", safe_text(&call.name));
            }
            EventKind::ToolFinished { call_id, output } => self.finish_tool(&call_id, output),
            EventKind::RunFinished { summary } => {
                self.run_state = RunState::Idle;
                let elapsed = self
                    .run_started
                    .take()
                    .map(|started| started.elapsed().as_secs())
                    .filter(|seconds| *seconds > 0)
                    .map(|seconds| format!(" in {}", ejm_tui::elapsed_text(seconds)))
                    .unwrap_or_default();
                if !self.received_text && !summary.text.is_empty() {
                    self.push(BlockKind::Assistant, "ejm-cli", &summary.text, None);
                }
                self.status = format!(
                    "{}{elapsed} · {} step{} · {} in / {} out tokens",
                    serde_json::to_value(&summary.status)
                        .ok()
                        .and_then(|value| value.as_str().map(str::to_owned))
                        .unwrap_or_else(|| "finished".into()),
                    summary.steps,
                    if summary.steps == 1 { "" } else { "s" },
                    compact_count(summary.usage.input_tokens),
                    compact_count(summary.usage.output_tokens),
                );
                if let Some(error) = summary.error {
                    self.error(error);
                }
            }
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
            if let Some(view) = &mut block.tool {
                view.state = if output.is_error {
                    ToolState::Failed
                } else {
                    ToolState::Done
                };
            }
            // Consumers that read the block kind still see the failure.
            if output.is_error {
                block.kind = BlockKind::Error;
            }
            if !block.text.is_empty() {
                block.text.push('\n');
                self.display_bytes += 1;
            }
            self.display_bytes += content.len();
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
            tool: None,
        });
        self.dirty = true;
        self.trim();
    }

    fn push_tool(&mut self, call: &ejm_harness::ToolCall, state: ToolState) {
        self.push(BlockKind::Tool, &call.name, "", Some(call.id.clone()));
        if let Some(block) = self.blocks.back_mut() {
            block.tool = Some(ToolView {
                name: safe_text(&call.name),
                detail: safe_text(&tool_detail(&call.arguments)),
                state,
            });
        }
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

/// The argument a reader needs to recognize a call, such as `src/main.rs` or `"fn main" in src`.
/// Falls back to compact JSON for tools without a familiar argument.
fn tool_detail(arguments: &serde_json::Value) -> String {
    let field = |name: &str| arguments.get(name).and_then(serde_json::Value::as_str);
    let mut detail = match (field("pattern"), field("path")) {
        (Some(pattern), Some(path)) => format!("\"{pattern}\" in {path}"),
        (Some(pattern), None) => format!("\"{pattern}\""),
        (None, Some(path)) => path.to_owned(),
        (None, None) => ["command", "cmd", "file", "url", "query", "name"]
            .into_iter()
            .find_map(field)
            .map_or_else(
                || match arguments {
                    serde_json::Value::Object(map) if map.is_empty() => String::new(),
                    serde_json::Value::Null => String::new(),
                    _ => serde_json::to_string(arguments).unwrap_or_default(),
                },
                str::to_owned,
            ),
    };
    // One line keeps the call readable; the full arguments stay in the event store.
    detail = detail.replace('\n', " ");
    truncate_end(&mut detail, 512);
    detail
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
    text.push('…');
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
        app.composer.insert("/s");
        assert_eq!(app.command_query(), Some("s"));
        assert_eq!(
            app.command_matches()
                .iter()
                .map(|entry| entry.name.as_str())
                .collect::<Vec<_>>(),
            ["status", "sessions", "steer", "stop", "skill"]
        );

        app.complete_command();
        assert_eq!(app.composer.text, "/status");
        assert_eq!(app.composer.cursor, app.composer.text.len());
        assert!(app.command_query().is_none());
        assert!(app.blocks.is_empty());
        assert_eq!(app.run_state, RunState::Idle);

        app.composer.replace("/s");
        app.command_picker = CommandPicker {
            selected: 2,
            dismissed: false,
        };
        app.complete_command();
        assert_eq!(app.composer.text, "/steer ");
        assert!(app.command_picker.dismissed);
    }

    #[test]
    fn command_dismissal_and_noncommand_drafts_keep_text_editable() {
        let mut app = app();
        app.composer.insert("/s");
        app.command_picker.dismissed = true;
        assert!(app.command_query().is_none());
        assert_eq!(app.composer.text, "/s");

        app.command_picker = CommandPicker::default();
        app.composer.left();
        assert!(app.command_query().is_none());
        app.composer.right();
        assert_eq!(app.command_query(), Some("s"));
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
    fn failed_tool_marks_both_the_tool_state_and_the_block_kind() {
        let mut app = app();
        let event = |sequence: u64, mut kind: serde_json::Value| {
            let fields = kind.as_object_mut().unwrap();
            fields.insert("session_id".into(), "test".into());
            fields.insert("sequence".into(), sequence.into());
            fields.insert("timestamp_ms".into(), 0.into());
            serde_json::from_value::<Event>(kind).unwrap()
        };
        app.apply(event(
            1,
            serde_json::json!({"type": "tool_started", "call": {
                "id": "c1", "name": "shell", "arguments": {"command": "false"}
            }}),
        ));
        app.apply(event(
            2,
            serde_json::json!({"type": "tool_finished", "call_id": "c1",
            "output": {"content": "exit 1", "is_error": true}}),
        ));
        let block = app.blocks.back().unwrap();
        assert_eq!(block.kind, BlockKind::Error);
        assert_eq!(block.tool.as_ref().unwrap().state, ToolState::Failed);
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
