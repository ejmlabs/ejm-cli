//! Version one product messages. Every byte in this module goes inside Noise encryption.

use std::{collections::VecDeque, io::Write};

use anyhow::{Result, anyhow, ensure};
use ejm_harness::{Event, Session};
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use super::{
    channel::MAX_PLAINTEXT_BYTES,
    invitation::{PROTOCOL_VERSION, ShareRole},
};

pub const MAX_REPLAY_EVENTS: usize = 512;
pub const MAX_MESSAGE_BYTES: usize = 1024 * 1024;
pub const MAX_PROMPT_BYTES: usize = 16 * 1024;
pub const MAX_PENDING_PROMPTS: usize = 8;
/// Native and browser peers share exactly represented action numbers.
pub const MAX_MUTATION_SEQUENCE: u64 = 9_007_199_254_740_991;
pub const OUTPUT_COALESCE_MS: u64 = 20;
const MAX_RESULT_CACHE: usize = 128;
const FRAME_MAGIC: &[u8; 4] = b"EJMR";
const HEADER_BYTES: usize = 22;
const CHUNK_BYTES: usize = MAX_PLAINTEXT_BYTES - HEADER_BYTES;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WireMessage {
    pub version: u16,
    pub body: WireBody,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(
    tag = "direction",
    content = "message",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum WireBody {
    Client(ClientRequest),
    Server(ServerMessage),
}

impl WireMessage {
    pub fn client(request: ClientRequest) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            body: WireBody::Client(request),
        }
    }

    pub fn server(response: ServerMessage) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            body: WireBody::Server(response),
        }
    }

    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.version == PROTOCOL_VERSION,
            "unsupported remote wire version"
        );
        match &self.body {
            WireBody::Client(ClientRequest::Snapshot { limit, .. }) => {
                ensure!(
                    *limit > 0 && usize::from(*limit) <= MAX_REPLAY_EVENTS,
                    "invalid replay page limit"
                );
            }
            WireBody::Client(ClientRequest::Mutation { mutation }) => mutation.validate()?,
            WireBody::Client(ClientRequest::Outcome { sequence }) => {
                validate_mutation_sequence(*sequence)?;
            }
            WireBody::Server(ServerMessage::Snapshot {
                session,
                events,
                through_sequence,
                active_run,
                mutation_high_water,
                ..
            }) => {
                ensure!(
                    *mutation_high_water <= MAX_MUTATION_SEQUENCE,
                    "invalid mutation high-water mark"
                );
                validate_id(&session.id)?;
                if let Some(run) = active_run {
                    validate_id(run)?;
                }
                validate_events(events, *through_sequence)?;
            }
            WireBody::Server(ServerMessage::Events {
                events,
                through_sequence,
                ..
            }) => {
                validate_events(events, *through_sequence)?;
            }
            WireBody::Server(ServerMessage::Outcome { sequence, outcome }) => {
                validate_mutation_sequence(*sequence)?;
                outcome.validate()?;
            }
            WireBody::Server(ServerMessage::Error { message, .. }) => {
                ensure!(message.len() <= 4096, "remote error is too large");
            }
            WireBody::Server(ServerMessage::Stopped { .. }) => {}
        }
        Ok(())
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum ClientRequest {
    Snapshot { after_sequence: u64, limit: u16 },
    Mutation { mutation: Mutation },
    Outcome { sequence: u64 },
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mutation {
    pub sequence: u64,
    pub expected: ExpectedState,
    pub action: MutationAction,
}

impl Mutation {
    pub fn validate(&self) -> Result<()> {
        validate_mutation_sequence(self.sequence)?;
        validate_id(&self.expected.session_id)?;
        if let Some(run) = &self.expected.run_id {
            validate_id(run)?;
        }
        match &self.action {
            MutationAction::Submit { text } => validate_prompt(text)?,
            MutationAction::Steer { run_id, text } | MutationAction::Queue { run_id, text } => {
                validate_id(run_id)?;
                validate_prompt(text)?;
                ensure!(
                    self.expected.run_id.as_ref() == Some(run_id),
                    "mutation targets a different expected run"
                );
            }
            MutationAction::Cancel { run_id } => {
                validate_id(run_id)?;
                ensure!(
                    self.expected.run_id.as_ref() == Some(run_id),
                    "mutation targets a different expected run"
                );
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExpectedState {
    pub session_id: String,
    pub run_id: Option<String>,
    pub event_sequence: u64,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum MutationAction {
    Submit { text: String },
    Steer { run_id: String, text: String },
    Queue { run_id: String, text: String },
    Cancel { run_id: String },
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum ServerMessage {
    Snapshot {
        session: Session,
        events: Vec<Event>,
        through_sequence: u64,
        gap: Option<ReplayGap>,
        active_run: Option<String>,
        role: ShareRole,
        mutation_high_water: u64,
    },
    Events {
        events: Vec<Event>,
        through_sequence: u64,
        gap: Option<ReplayGap>,
    },
    Outcome {
        sequence: u64,
        outcome: MutationOutcome,
    },
    Stopped {
        reason: StopReason,
    },
    Error {
        code: RemoteError,
        message: String,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReplayGap {
    pub requested_after_sequence: u64,
    pub available_after_sequence: u64,
    pub reason: GapReason,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GapReason {
    Retention,
    OutputTruncated,
    SlowConsumer,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    LocalStop,
    HostExit,
    SessionChanged,
    Expired,
    TransportClosed,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RemoteError {
    AccessDenied,
    StaleState,
    QueueFull,
    InvalidRequest,
    Unavailable,
    Expired,
    UnknownOutcome,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum MutationOutcome {
    Accepted { run_id: Option<String> },
    Queued,
    Rejected { code: RemoteError, message: String },
    Unknown,
    Pending,
}

impl MutationOutcome {
    fn validate(&self) -> Result<()> {
        match self {
            Self::Accepted {
                run_id: Some(run_id),
            } => validate_id(run_id)?,
            Self::Rejected { message, .. } => {
                ensure!(message.len() <= 4096, "remote error is too large");
            }
            _ => {}
        }
        Ok(())
    }
}

fn validate_mutation_sequence(sequence: u64) -> Result<()> {
    ensure!(
        (1..=MAX_MUTATION_SEQUENCE).contains(&sequence),
        "invalid mutation sequence"
    );
    Ok(())
}

fn validate_id(id: &str) -> Result<()> {
    ensure!(
        !id.is_empty() && id.len() <= 256,
        "invalid remote session or run identifier"
    );
    Ok(())
}

fn validate_prompt(text: &str) -> Result<()> {
    ensure!(
        !text.trim().is_empty() && text.len() <= MAX_PROMPT_BYTES,
        "prompt is empty or exceeds the remote limit"
    );
    Ok(())
}

fn validate_events(events: &[Event], through_sequence: u64) -> Result<()> {
    ensure!(
        events.len() <= MAX_REPLAY_EVENTS,
        "replay page exceeds the remote limit"
    );
    let mut previous = None;
    for event in events {
        validate_id(&event.session_id)?;
        if let Some(run) = &event.run_id {
            validate_id(run)?;
        }
        ensure!(
            event.sequence <= through_sequence,
            "event exceeds replay cursor"
        );
        if let Some(previous) = previous {
            ensure!(event.sequence > previous, "replay events are out of order");
        }
        previous = Some(event.sequence);
    }
    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
pub enum MutationAdmission {
    Fresh,
    Duplicate(MutationOutcome),
    Invalid,
}

/// Keep one instance for the share epoch, across every network reconnect.
/// Advancing the high-water mark precedes dispatch, including requests later denied.
pub struct MutationLedger {
    high_water: u64,
    capacity: usize,
    results: VecDeque<(u64, MutationOutcome)>,
}

impl MutationLedger {
    pub fn new(capacity: usize) -> Self {
        Self {
            high_water: 0,
            capacity: capacity.clamp(1, MAX_RESULT_CACHE),
            results: VecDeque::new(),
        }
    }

    pub fn admit(&mut self, sequence: u64) -> MutationAdmission {
        if !(1..=MAX_MUTATION_SEQUENCE).contains(&sequence) {
            return MutationAdmission::Invalid;
        }
        if sequence <= self.high_water {
            return MutationAdmission::Duplicate(self.lookup(sequence));
        }
        self.high_water = sequence;
        self.results.push_back((sequence, MutationOutcome::Pending));
        while self.results.len() > self.capacity {
            self.results.pop_front();
        }
        MutationAdmission::Fresh
    }

    pub fn record(&mut self, sequence: u64, outcome: MutationOutcome) -> Result<()> {
        ensure!(
            outcome != MutationOutcome::Pending && outcome != MutationOutcome::Unknown,
            "only a known completed outcome can be recorded"
        );
        outcome.validate()?;
        let cached = self
            .results
            .iter_mut()
            .find(|(id, _)| *id == sequence)
            .ok_or_else(|| anyhow!("mutation outcome is no longer retained"))?;
        ensure!(
            cached.1 == MutationOutcome::Pending
                || (cached.1 == MutationOutcome::Queued
                    && matches!(
                        outcome,
                        MutationOutcome::Accepted { .. } | MutationOutcome::Rejected { .. }
                    )),
            "mutation outcome has already been recorded"
        );
        cached.1 = outcome;
        Ok(())
    }

    pub fn lookup(&self, sequence: u64) -> MutationOutcome {
        self.results
            .iter()
            .find(|(id, _)| *id == sequence)
            .map_or(MutationOutcome::Unknown, |(_, result)| result.clone())
    }

    pub fn high_water(&self) -> u64 {
        self.high_water
    }
}

pub struct FrameEncoder {
    next_message: u64,
}

impl FrameEncoder {
    pub fn new() -> Self {
        Self { next_message: 1 }
    }

    /// Each returned plaintext chunk must be encrypted as one ordered Noise frame.
    pub fn encode(&mut self, message: &WireMessage) -> Result<Vec<Vec<u8>>> {
        message.validate()?;
        let mut buffer = BoundedJson(Zeroizing::new(Vec::new()));
        serde_json::to_writer(&mut buffer, message).map_err(|_| {
            anyhow!("remote message exceeds the reassembly limit or cannot be encoded")
        })?;
        let count = u16::try_from(buffer.0.len().div_ceil(CHUNK_BYTES))?;
        let total = u32::try_from(buffer.0.len())?;
        let id = self.next_message;
        self.next_message = id
            .checked_add(1)
            .ok_or_else(|| anyhow!("remote frame sequence exhausted; reconnect"))?;
        let mut frames = Vec::with_capacity(usize::from(count));
        for (index, chunk) in buffer.0.chunks(CHUNK_BYTES).enumerate() {
            let mut frame = Vec::with_capacity(HEADER_BYTES + chunk.len());
            frame.extend_from_slice(FRAME_MAGIC);
            frame.extend_from_slice(&PROTOCOL_VERSION.to_be_bytes());
            frame.extend_from_slice(&id.to_be_bytes());
            frame.extend_from_slice(&total.to_be_bytes());
            frame.extend_from_slice(&u16::try_from(index)?.to_be_bytes());
            frame.extend_from_slice(&count.to_be_bytes());
            frame.extend_from_slice(chunk);
            frames.push(frame);
        }
        Ok(frames)
    }
}

struct BoundedJson(Zeroizing<Vec<u8>>);

impl Write for BoundedJson {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > MAX_MESSAGE_BYTES.saturating_sub(self.0.len()) {
            return Err(std::io::Error::other("remote message limit"));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

struct PartialMessage {
    id: u64,
    total: usize,
    count: u16,
    next_chunk: u16,
    bytes: Zeroizing<Vec<u8>>,
}

pub struct FrameDecoder {
    next_message: u64,
    partial: Option<PartialMessage>,
    failed: bool,
}

impl FrameDecoder {
    pub fn new() -> Self {
        Self {
            next_message: 1,
            partial: None,
            failed: false,
        }
    }

    /// Call only after authentication. A framing error closes this decoder permanently.
    /// Reconnect uses a new decoder and requests a bounded snapshot with an explicit gap.
    pub fn push(&mut self, frame: &[u8]) -> Result<Option<WireMessage>> {
        ensure!(!self.failed, "remote framing is closed");
        match self.push_inner(frame) {
            Ok(message) => Ok(message),
            Err(error) => {
                self.partial = None;
                self.failed = true;
                Err(error)
            }
        }
    }

    fn push_inner(&mut self, frame: &[u8]) -> Result<Option<WireMessage>> {
        ensure!(
            (HEADER_BYTES + 1..=MAX_PLAINTEXT_BYTES).contains(&frame.len()),
            "invalid remote chunk length"
        );
        ensure!(
            &frame[..4] == FRAME_MAGIC
                && u16::from_be_bytes(frame[4..6].try_into()?) == PROTOCOL_VERSION,
            "unsupported remote frame version"
        );
        let id = u64::from_be_bytes(frame[6..14].try_into()?);
        let total = usize::try_from(u32::from_be_bytes(frame[14..18].try_into()?))?;
        let index = u16::from_be_bytes(frame[18..20].try_into()?);
        let count = u16::from_be_bytes(frame[20..22].try_into()?);
        let chunk = &frame[HEADER_BYTES..];
        ensure!(
            id == self.next_message && total > 0 && total <= MAX_MESSAGE_BYTES,
            "remote message sequence gap or size limit"
        );
        ensure!(
            count > 0 && index < count && usize::from(count) == total.div_ceil(CHUNK_BYTES),
            "invalid remote chunk count"
        );
        if self.partial.is_none() {
            ensure!(index == 0, "remote message has a chunk gap");
            self.partial = Some(PartialMessage {
                id,
                total,
                count,
                next_chunk: 0,
                bytes: Zeroizing::new(Vec::with_capacity(total)),
            });
        }
        let partial = self
            .partial
            .as_mut()
            .ok_or_else(|| anyhow!("remote reassembly is unavailable"))?;
        ensure!(
            partial.id == id
                && partial.total == total
                && partial.count == count
                && partial.next_chunk == index,
            "remote chunks are interleaved or out of order"
        );
        let remaining = total.saturating_sub(partial.bytes.len());
        ensure!(
            chunk.len() == remaining.min(CHUNK_BYTES),
            "invalid remote chunk size"
        );
        partial.bytes.extend_from_slice(chunk);
        partial.next_chunk += 1;
        if partial.next_chunk < count {
            return Ok(None);
        }
        let complete = self
            .partial
            .take()
            .ok_or_else(|| anyhow!("remote reassembly is unavailable"))?;
        ensure!(
            complete.bytes.len() == total,
            "remote message is incomplete"
        );
        let message: WireMessage = serde_json::from_slice(&complete.bytes)
            .map_err(|_| anyhow!("invalid remote message"))?;
        message.validate()?;
        self.next_message = self
            .next_message
            .checked_add(1)
            .ok_or_else(|| anyhow!("remote frame sequence exhausted; reconnect"))?;
        Ok(Some(message))
    }
}

#[cfg(test)]
#[expect(
    clippy::unwrap_used,
    reason = "test fixtures should fail at the assertion site"
)]
mod tests {
    use super::*;

    fn mutation(sequence: u64, text: String) -> WireMessage {
        WireMessage::client(ClientRequest::Mutation {
            mutation: Mutation {
                sequence,
                expected: ExpectedState {
                    session_id: "session-a".into(),
                    run_id: None,
                    event_sequence: 2,
                },
                action: MutationAction::Submit { text },
            },
        })
    }

    #[test]
    fn evicted_or_unknown_mutations_cannot_execute_again() {
        let mut ledger = MutationLedger::new(1);
        let mut dispatch_count = 0;
        for sequence in [1, 1, 2, 1, 2, 0] {
            if ledger.admit(sequence) == MutationAdmission::Fresh {
                dispatch_count += 1;
                ledger
                    .record(
                        sequence,
                        MutationOutcome::Accepted {
                            run_id: Some(format!("run-{sequence}")),
                        },
                    )
                    .unwrap();
            }
        }
        assert_eq!(dispatch_count, 2);
        assert_eq!(ledger.lookup(1), MutationOutcome::Unknown);
        assert_eq!(
            ledger.admit(1),
            MutationAdmission::Duplicate(MutationOutcome::Unknown)
        );
        assert_eq!(ledger.high_water(), 2);
        assert!(ledger.record(2, MutationOutcome::Queued).is_err());
        assert!(ledger.record(1, MutationOutcome::Queued).is_err());
    }

    #[test]
    fn admission_precedes_dispatch_and_survives_connection_loss() {
        let mut ledger = MutationLedger::new(8);
        assert_eq!(ledger.admit(7), MutationAdmission::Fresh);
        assert_eq!(ledger.lookup(7), MutationOutcome::Pending);
        assert_eq!(
            ledger.admit(7),
            MutationAdmission::Duplicate(MutationOutcome::Pending)
        );
        assert_eq!(
            ledger.admit(3),
            MutationAdmission::Duplicate(MutationOutcome::Unknown)
        );
        assert_eq!(ledger.lookup(99), MutationOutcome::Unknown);
        assert_eq!(ledger.high_water(), 7);
    }

    #[test]
    fn out_of_range_action_numbers_cannot_poison_the_share_counter() {
        let mut ledger = MutationLedger::new(8);
        assert_eq!(
            ledger.admit(MAX_MUTATION_SEQUENCE + 1),
            MutationAdmission::Invalid
        );
        assert_eq!(ledger.high_water(), 0);
        assert!(
            mutation(MAX_MUTATION_SEQUENCE + 1, "invalid".into())
                .validate()
                .is_err()
        );
        assert!(
            mutation(MAX_MUTATION_SEQUENCE, "valid final action".into())
                .validate()
                .is_ok()
        );
        assert_eq!(ledger.admit(1), MutationAdmission::Fresh);
    }

    #[test]
    fn unicode_prompt_roundtrips_in_bounded_ordered_chunks() {
        let text = "界".repeat(MAX_PROMPT_BYTES / 3);
        let mut encoder = FrameEncoder::new();
        let frames = encoder.encode(&mutation(1, text.clone())).unwrap();
        assert!(frames.len() > 1);
        assert!(
            frames
                .iter()
                .all(|frame| frame.len() <= MAX_PLAINTEXT_BYTES)
        );
        let mut decoder = FrameDecoder::new();
        let mut completed = None;
        for frame in frames {
            completed = decoder.push(&frame).unwrap();
        }
        let message = completed.unwrap();
        let restored = match message.body {
            WireBody::Client(ClientRequest::Mutation {
                mutation:
                    Mutation {
                        action: MutationAction::Submit { text },
                        ..
                    },
            }) => Some(text),
            _ => None,
        }
        .unwrap();
        assert_eq!(restored, text);
    }

    #[test]
    fn chunk_gaps_oversize_and_downgrades_fail_without_unbounded_reassembly() {
        let frames = FrameEncoder::new()
            .encode(&mutation(1, "x".repeat(MAX_PROMPT_BYTES)))
            .unwrap();
        let mut decoder = FrameDecoder::new();
        assert!(decoder.push(&frames[1]).is_err());
        assert!(decoder.partial.is_none());
        assert!(decoder.push(&frames[0]).is_err());
        let mut oversized = frames[0].clone();
        oversized[14..18]
            .copy_from_slice(&u32::try_from(MAX_MESSAGE_BYTES + 1).unwrap().to_be_bytes());
        assert!(FrameDecoder::new().push(&oversized).is_err());
        let mut downgraded = frames[0].clone();
        downgraded[4..6].copy_from_slice(&0_u16.to_be_bytes());
        assert!(FrameDecoder::new().push(&downgraded).is_err());
        assert!(
            FrameDecoder::new()
                .push(&vec![0; MAX_PLAINTEXT_BYTES + 1])
                .is_err()
        );
    }

    #[test]
    fn forbidden_wire_actions_unknown_fields_and_prompt_limits_are_rejected() {
        let unknown = br#"{"version":1,"body":{"direction":"client","message":{"type":"set_tool_grants","grants":["shell"]}}}"#;
        assert!(serde_json::from_slice::<WireMessage>(unknown).is_err());
        let extra = br#"{"version":1,"body":{"direction":"client","message":{"type":"snapshot","after_sequence":0,"limit":1,"provider":"secret"}}}"#;
        assert!(serde_json::from_slice::<WireMessage>(extra).is_err());
        assert!(
            FrameEncoder::new()
                .encode(&mutation(1, "x".repeat(MAX_PROMPT_BYTES + 1)))
                .is_err()
        );
        assert!(
            WireMessage::client(ClientRequest::Snapshot {
                after_sequence: 0,
                limit: 513
            })
            .validate()
            .is_err()
        );
        let mut message = mutation(1, "okay".into());
        message.version = 0;
        assert!(message.validate().is_err());
    }
}
