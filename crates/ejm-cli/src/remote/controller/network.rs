//! The connection actor owns retransmission policy. Only reads are retried.
use super::{Command, CurrentSession, Update};
use crate::remote::{
    account::RelayAccount,
    backend::ReplayPage,
    identity::EndpointIdentity,
    invitation::Invitation,
    protocol::{
        ClientRequest, MAX_REPLAY_EVENTS, MutationOutcome, RemoteError, ServerMessage, WireBody,
        WireMessage,
    },
    transport::{self, EncryptedSocket},
};
use anyhow::{Result, anyhow, bail};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use std::time::Duration;
use tokio::sync::{Notify, RwLock, mpsc, oneshot};
use tokio_util::sync::CancellationToken;

pub(super) struct Context {
    pub account: RelayAccount,
    pub identity: EndpointIdentity,
    pub invitation: Invitation,
    pub cursor: Arc<AtomicU64>,
    pub cursor_changed: Arc<Notify>,
    pub current: Arc<RwLock<CurrentSession>>,
    pub cancel: CancellationToken,
}

struct Actor {
    socket: EncryptedSocket,
    context: Context,
    commands: mpsc::Receiver<Command>,
    display: Display,
    pending: Option<u64>,
    replay: Option<oneshot::Sender<Result<ReplayPage>>>,
    reconnects: u32,
    refreshing: bool,
}

struct Display {
    updates: mpsc::Sender<Update>,
    ready: bool,
    recovery: Option<Recovery>,
}

struct Recovery {
    through: u64,
    page: Option<SnapshotProgress>,
}

struct SnapshotProgress {
    through: u64,
    full: bool,
}

enum Input {
    Command(Option<Command>),
    Message(Result<WireMessage>),
    CatchUp,
    Cursor,
}

pub(super) async fn run(
    socket: EncryptedSocket,
    context: Context,
    commands: mpsc::Receiver<Command>,
    updates: mpsc::Sender<Update>,
) {
    Actor {
        socket,
        context,
        commands,
        display: Display::new(updates),
        pending: None,
        replay: None,
        reconnects: 0,
        refreshing: false,
    }
    .run()
    .await;
}

impl Actor {
    async fn run(&mut self) {
        loop {
            let catch_up = !self.refreshing
                && self.replay.is_none()
                && self
                    .display
                    .needs_snapshot(self.context.cursor.load(Ordering::Relaxed));
            let input = tokio::select! {
                biased;
                () = self.context.cancel.cancelled() => { self.socket.close().await; return; }
                permit = self.display.updates.reserve(), if catch_up => {
                    if permit.is_err() { return; }
                    drop(permit);
                    Input::CatchUp
                }
                () = self.context.cursor_changed.notified() => Input::Cursor,
                command = self.commands.recv() => Input::Command(command),
                message = self.socket.receive() => Input::Message(message),
            };
            let result = match input {
                Input::Command(None) => {
                    self.socket.close().await;
                    return;
                }
                Input::Command(Some(command)) => self.command(command).await,
                Input::Message(Ok(WireMessage {
                    body: WireBody::Server(message),
                    ..
                })) => {
                    self.reconnects = 0;
                    self.message(message).await
                }
                Input::Message(_) => Err(anyhow!("remote connection interrupted")),
                Input::CatchUp => self.refresh().await,
                Input::Cursor => Ok(true),
            };
            match result {
                Ok(true) => {}
                Ok(false) => return,
                Err(_) => {
                    if !self.reconnect().await {
                        return;
                    }
                }
            }
        }
    }

    async fn command(&mut self, command: Command) -> Result<bool> {
        match command {
            Command::Ready => {
                self.display.ready = true;
                self.display
                    .recover(self.context.cursor.load(Ordering::Relaxed));
            }
            Command::Replay { after, reply } if self.replay.is_none() => {
                self.replay = Some(reply);
                self.snapshot(after).await?;
            }
            Command::Replay { reply, .. } => {
                let _ = reply.send(Err(anyhow!("another display replay is pending")));
            }
            Command::Refresh => return self.refresh().await,
            Command::Mutation(mutation) if self.pending.is_none() => {
                // Set the sequence before send. A failed write can have reached the host.
                self.pending = Some(mutation.sequence);
                self.socket
                    .send(&WireMessage::client(ClientRequest::Mutation { mutation }))
                    .await?;
            }
            Command::Mutation(mutation) => {
                self.display
                    .publish(ServerMessage::Outcome {
                        sequence: mutation.sequence,
                        outcome: MutationOutcome::Rejected {
                            code: RemoteError::Unavailable,
                            message: "one action is already pending".into(),
                        },
                    })
                    .await?;
            }
        }
        Ok(true)
    }

    async fn message(&mut self, message: ServerMessage) -> Result<bool> {
        match &message {
            ServerMessage::Snapshot {
                session,
                active_run,
                mutation_high_water,
                events,
                through_sequence,
                gap,
                ..
            } => {
                *self.context.current.write().await = CurrentSession {
                    session: session.clone(),
                    active_run: active_run.clone(),
                    mutation_high_water: *mutation_high_water,
                };
                self.refreshing = false;
                if let Some(reply) = self.replay.take() {
                    let _ = reply.send(Ok(ReplayPage {
                        events: events.clone(),
                        through_sequence: *through_sequence,
                        gap: gap.is_some(),
                    }));
                    return Ok(true);
                }
            }
            ServerMessage::Outcome { sequence, outcome } if self.pending == Some(*sequence) => {
                if !matches!(outcome, MutationOutcome::Pending) {
                    self.pending = None;
                }
            }
            ServerMessage::Stopped { .. } => {
                self.display.publish(message).await?;
                return Ok(false);
            }
            _ => {}
        }
        self.display.publish(message).await?;
        Ok(true)
    }

    async fn snapshot(&mut self, after_sequence: u64) -> Result<()> {
        self.socket
            .send(&WireMessage::client(ClientRequest::Snapshot {
                after_sequence,
                limit: 512,
            }))
            .await
    }

    async fn refresh(&mut self) -> Result<bool> {
        if !self.refreshing && self.replay.is_none() {
            let after = self.context.cursor.load(Ordering::Relaxed);
            self.display.recover(after);
            self.snapshot(after).await?;
            self.refreshing = true;
        }
        if let Some(sequence) = self.pending {
            self.socket
                .send(&WireMessage::client(ClientRequest::Outcome { sequence }))
                .await?;
        }
        Ok(true)
    }

    async fn reconnect(&mut self) -> bool {
        if let Some(reply) = self.replay.take() {
            let _ = reply.send(Err(anyhow!("display connection interrupted")));
        }
        if self.display.updates.send(Update::Notice("Connection lost. Reconnecting restores output and checks pending results without resubmitting actions.")).await.is_err() {
            return false;
        }
        loop {
            self.reconnects += 1;
            if self.reconnects > 8 {
                let _ = self.display.updates.send(Update::Ended).await;
                return false;
            }
            let delay = Duration::from_millis((250_u64 << self.reconnects.min(5)).min(8000));
            tokio::select! { biased; () = self.context.cancel.cancelled() => return false, () = tokio::time::sleep(delay) => {} }
            let connected = tokio::select! {
                biased;
                () = self.context.cancel.cancelled() => return false,
                result = transport::connect_controller(&self.context.account, &self.context.identity, &self.context.invitation) => result,
            };
            let Ok(socket) = connected else {
                continue;
            };
            self.socket = socket;
            self.refreshing = false;
            if self.refresh().await.is_ok() {
                return true;
            }
        }
    }
}

impl Display {
    fn new(updates: mpsc::Sender<Update>) -> Self {
        Self {
            updates,
            ready: false,
            recovery: None,
        }
    }

    fn recover(&mut self, through: u64) {
        let recovery = self.recovery.get_or_insert(Recovery {
            through,
            page: None,
        });
        recovery.through = recovery.through.max(through);
    }

    fn needs_snapshot(&mut self, cursor: u64) -> bool {
        if !self.ready {
            return false;
        }
        let Some(recovery) = &self.recovery else {
            return false;
        };
        if let Some(page) = &recovery.page {
            if cursor < page.through {
                return false;
            }
            if !page.full && page.through >= recovery.through {
                self.recovery = None;
                return false;
            }
        }
        true
    }

    async fn publish(&mut self, message: ServerMessage) -> Result<()> {
        match &message {
            ServerMessage::Events {
                through_sequence, ..
            } => {
                let through = *through_sequence;
                if !self.ready || self.recovery.is_some() {
                    self.recover(through);
                    return Ok(());
                }
                match self.updates.try_send(Update::Server(message)) {
                    Ok(()) => {}
                    Err(mpsc::error::TrySendError::Full(_)) => self.recover(through),
                    Err(mpsc::error::TrySendError::Closed(_)) => bail!("controller display ended"),
                }
                return Ok(());
            }
            ServerMessage::Snapshot {
                events,
                through_sequence,
                ..
            } if self.recovery.is_some() || events.len() == MAX_REPLAY_EVENTS => {
                self.recover(*through_sequence);
                if let Some(recovery) = &mut self.recovery {
                    recovery.page = Some(SnapshotProgress {
                        through: *through_sequence,
                        full: events.len() == MAX_REPLAY_EVENTS,
                    });
                }
            }
            _ => {}
        }
        self.updates
            .send(Update::Server(message))
            .await
            .map_err(|_| anyhow!("controller display ended"))
    }
}

#[cfg(test)]
#[path = "network_tests.rs"]
mod tests;
