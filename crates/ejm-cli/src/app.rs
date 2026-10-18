use std::{collections::VecDeque, fmt::Write};

use ejm_harness::{Event, EventKind, Session, ToolOutput};

use crate::render::safe_text;

const MAX_BLOCKS: usize = 256;
const MAX_DISPLAY_BYTES: usize = 256 * 1024;
const MAX_COMPOSER_BYTES: usize = 64 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockKind {
    User,
    Assistant,
    Tool,
    Error,
    Notice,
}

pub struct TranscriptBlock {
    pub kind: BlockKind,
    pub title: String,
    pub text: String,
    pub call_id: Option<String>,
}

pub enum Overlay {
    Help,
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

pub struct App {
    pub session: Session,
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
    display_bytes: usize,
    received_text: bool,
}

impl App {
    pub fn new(session: Session) -> Self {
        Self {
            session,
            blocks: VecDeque::new(),
            composer: Composer::default(),
            run_state: RunState::Idle,
            scroll_from_bottom: 0,
            last_sequence: 0,
            status: "Ready. Ask a question or describe a change.".into(),
            overlay: None,
            dirty: true,
            palette: None,
            widget_lines: Vec::new(),
            display_bytes: 0,
            received_text: false,
        }
    }

    pub fn is_running(&self) -> bool {
        self.run_state != RunState::Idle
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
            EventKind::RunFinished { summary } => {
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
