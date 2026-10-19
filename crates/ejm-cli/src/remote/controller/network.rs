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
    updates: mpsc::Sender<Update>,
    pending: Option<u64>,
    replay: Option<oneshot::Sender<Result<ReplayPage>>>,
    reconnects: u32,
    ready: bool,
    missed: bool,
    refreshing: bool,
    catch_up_after: u64,
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
        updates,
        pending: None,
        replay: None,
        reconnects: 0,
        ready: false,
        missed: false,
        refreshing: false,
        catch_up_after: 0,
    }
    .run()
    .await;
}

impl Actor {
    async fn run(&mut self) {
        loop {
            let input = tokio::select! {
                biased;
                () = self.context.cancel.cancelled() => { self.socket.close().await; return; }
                permit = self.updates.reserve(), if self.ready && self.missed && !self.refreshing && self.context.cursor.load(Ordering::Relaxed) >= self.catch_up_after => {
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
                self.ready = true;
                self.missed = true;
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
                self.publish(ServerMessage::Outcome {
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
                if events.len() == MAX_REPLAY_EVENTS {
                    self.missed = true;
                    self.catch_up_after = *through_sequence;
                }
            }
            ServerMessage::Outcome { sequence, outcome } if self.pending == Some(*sequence) => {
                if !matches!(outcome, MutationOutcome::Pending) {
                    self.pending = None;
                }
            }
            ServerMessage::Stopped { .. } => {
                self.publish(message).await?;
                return Ok(false);
            }
            _ => {}
        }
        if !self.ready && matches!(message, ServerMessage::Events { .. }) {
            self.missed = true;
            return Ok(true);
        }
        self.publish(message).await?;
        Ok(true)
    }

    async fn publish(&mut self, message: ServerMessage) -> Result<()> {
        if matches!(message, ServerMessage::Events { .. }) {
            match self.updates.try_send(Update::Server(message)) {
                Ok(()) => {}
                Err(mpsc::error::TrySendError::Full(_)) => self.missed = true,
                Err(mpsc::error::TrySendError::Closed(_)) => bail!("controller display ended"),
            }
        } else {
            self.updates
                .send(Update::Server(message))
                .await
                .map_err(|_| anyhow!("controller display ended"))?;
        }
        Ok(())
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
            self.snapshot(self.context.cursor.load(Ordering::Relaxed))
                .await?;
            self.refreshing = true;
            self.missed = false;
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
        if self.updates.send(Update::Notice("Connection lost. Reconnecting restores output and checks pending results without resubmitting actions.")).await.is_err() {
            return false;
        }
        loop {
            self.reconnects += 1;
            if self.reconnects > 8 {
                let _ = self.updates.send(Update::Ended).await;
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
