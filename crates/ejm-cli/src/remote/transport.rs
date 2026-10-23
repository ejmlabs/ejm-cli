//! Certificate-verified sockets carry only Noise handshake bytes and ciphertext.

use std::{
    future::pending,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{Result, anyhow, bail, ensure};
use ejm_harness::Event;
use futures_util::{SinkExt, StreamExt};
use tokio::{
    net::TcpStream,
    sync::{broadcast, mpsc, oneshot},
    time::{Instant, sleep, sleep_until, timeout},
};
use tokio_rustls::rustls::{ClientConfig, RootCertStore};
use tokio_tungstenite::{
    Connector, MaybeTlsStream, WebSocketStream, connect_async_tls_with_config,
    tungstenite::{
        Message, client::IntoClientRequest, handshake::client::Request, http::HeaderValue,
        protocol::WebSocketConfig,
    },
};
use tokio_util::sync::CancellationToken;
use zeroize::Zeroizing;

use super::{
    account::{RelayAccount, valid_id},
    backend::LocalBackend,
    channel::{EncryptedChannel, MAX_CIPHERTEXT_BYTES, NoiseHandshake},
    identity::EndpointIdentity,
    invitation::{Enrollment, Invitation},
    protocol::{
        ClientRequest, FrameDecoder, FrameEncoder, GapReason, MAX_REPLAY_EVENTS,
        OUTPUT_COALESCE_MS, RemoteError, ReplayGap, ServerMessage, StopReason, WireBody,
        WireMessage,
    },
};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(20);
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(30);
const WRITE_TIMEOUT: Duration = Duration::from_secs(20);
const MAX_CONSECUTIVE_RETRIES: u32 = 8;
const FRAME_SPACING: Duration = Duration::from_millis(12);
const LEASE_RENEWAL_MS: u64 = 14 * 60 * 1000;
const DAY_MS: u64 = 24 * 60 * 60 * 1000;

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

pub enum HostNotice {
    Listening,
    Confirm {
        fingerprint: String,
        answer: oneshot::Sender<bool>,
    },
    Connected,
    Disconnected,
    Failed,
}

pub struct RelayConnection {
    socket: Socket,
}

impl RelayConnection {
    pub async fn connect(account: &RelayAccount, room: &str, role: &str) -> Result<Self> {
        ensure!(
            valid_id(room) && matches!(role, "host" | "controller"),
            "invalid relay socket scope"
        );
        let ticket = account.ticket(room, role).await?;
        let mut url = account.origin.url(&format!("/v1/rooms/{room}/{role}"))?;
        let scheme = match url.scheme() {
            "https" => "wss",
            #[cfg(test)]
            "http" => "ws",
            _ => bail!("relay WebSocket requires a configured HTTPS origin"),
        };
        url.set_scheme(scheme)
            .map_err(|()| anyhow!("invalid relay WebSocket origin"))?;
        let mut request = url
            .as_str()
            .into_client_request()
            .map_err(|_| anyhow!("invalid relay WebSocket request"))?;
        let authorization = Zeroizing::new(format!("Bearer {}", ticket.expose_secret()));
        let mut value = HeaderValue::from_str(&authorization)
            .map_err(|_| anyhow!("invalid relay connection ticket"))?;
        value.set_sensitive(true);
        request.headers_mut().insert("authorization", value);
        request.headers_mut().insert(
            "origin",
            HeaderValue::from_str(&account.origin.url("/")?.origin().ascii_serialization())?,
        );
        Self::connect_request(request, bundled_tls_connector()?).await
    }

    async fn connect_request(request: Request, connector: Connector) -> Result<Self> {
        // One connection, with no credential-bearing redirect following.
        let (socket, response) = timeout(
            CONNECT_TIMEOUT,
            connect_async_tls_with_config(request, Some(socket_config()), true, Some(connector)),
        )
        .await
        .map_err(|_| anyhow!("relay connection timed out"))?
        .map_err(|_| anyhow!("relay connection refused or certificate validation failed"))?;
        ensure!(
            response.headers().get("sec-websocket-extensions").is_none(),
            "relay requested unsupported WebSocket extensions"
        );
        Ok(Self { socket })
    }

    async fn send_binary(&mut self, bytes: Vec<u8>) -> Result<()> {
        ensure!(
            !bytes.is_empty() && bytes.len() <= MAX_CIPHERTEXT_BYTES,
            "relay frame exceeds limit"
        );
        timeout(
            WRITE_TIMEOUT,
            self.socket.send(Message::Binary(bytes.into())),
        )
        .await
        .map_err(|_| anyhow!("relay write timed out"))?
        .map_err(|_| anyhow!("relay connection closed during write"))
    }

    async fn receive_binary(&mut self) -> Result<Vec<u8>> {
        loop {
            match self.socket.next().await {
                Some(Ok(Message::Binary(bytes))) => {
                    ensure!(
                        !bytes.is_empty() && bytes.len() <= MAX_CIPHERTEXT_BYTES,
                        "relay frame exceeds limit"
                    );
                    return Ok(bytes.to_vec());
                }
                Some(Ok(Message::Ping(_) | Message::Pong(_))) => {
                    self.socket
                        .flush()
                        .await
                        .map_err(|_| anyhow!("relay connection closed"))?;
                }
                Some(Ok(Message::Close(_))) | None => bail!("relay connection closed"),
                Some(Ok(_)) => bail!("relay sent an unsupported frame"),
                Some(Err(_)) => bail!("relay transport failed"),
            }
        }
    }

    async fn close(&mut self) {
        let _ = timeout(Duration::from_secs(1), self.socket.close(None)).await;
    }
}

fn bundled_tls_connector() -> Result<Connector> {
    tls_connector(RootCertStore::from_iter(
        webpki_roots::TLS_SERVER_ROOTS.iter().cloned(),
    ))
}

fn tls_connector(roots: RootCertStore) -> Result<Connector> {
    let provider = tokio_rustls::rustls::crypto::aws_lc_rs::default_provider();
    let config = ClientConfig::builder_with_provider(Arc::new(provider))
        .with_safe_default_protocol_versions()?
        .with_root_certificates(roots)
        .with_no_client_auth();
    Ok(Connector::Rustls(Arc::new(config)))
}

fn socket_config() -> WebSocketConfig {
    WebSocketConfig::default()
        .read_buffer_size(MAX_CIPHERTEXT_BYTES)
        .write_buffer_size(0)
        .max_write_buffer_size(64 * 1024)
        .max_message_size(Some(MAX_CIPHERTEXT_BYTES))
        .max_frame_size(Some(MAX_CIPHERTEXT_BYTES))
}

pub struct EncryptedSocket {
    connection: RelayConnection,
    channel: EncryptedChannel,
    encoder: FrameEncoder,
    decoder: FrameDecoder,
    next_send: Instant,
    host_authority: Option<SendAuthority>,
}

struct SendAuthority {
    backend: LocalBackend,
    epoch: String,
    session: String,
    cancel: CancellationToken,
}

impl EncryptedSocket {
    fn new(connection: RelayConnection, channel: EncryptedChannel) -> Self {
        Self {
            connection,
            channel,
            encoder: FrameEncoder::new(),
            decoder: FrameDecoder::new(),
            next_send: Instant::now(),
            host_authority: None,
        }
    }

    pub async fn send(&mut self, message: &WireMessage) -> Result<()> {
        message.validate()?;
        let final_notice = matches!(
            &message.body,
            WireBody::Server(ServerMessage::Stopped { .. })
        );
        let frames = match self.encoder.encode(message) {
            Ok(frames) => frames,
            Err(error) => {
                if let Some(gap) = truncated_message(message) {
                    self.encoder.encode(&gap)?
                } else {
                    return Err(error);
                }
            }
        };
        for frame in frames {
            sleep_until(self.next_send).await;
            if let Some(authority) = &self.host_authority
                && !final_notice
            {
                ensure!(
                    !authority.cancel.is_cancelled()
                        && authority
                            .backend
                            .is_current_share(&authority.epoch, &authority.session, now_ms())
                            .await,
                    "sharing has stopped"
                );
            }
            let plaintext = Zeroizing::new(frame);
            let ciphertext = self.channel.encrypt(&plaintext, now_ms())?;
            if let Some(authority) = &self.host_authority
                && !final_notice
            {
                tokio::select! {
                    biased;
                    () = authority.cancel.cancelled() => bail!("sharing has stopped"),
                    result = self.connection.send_binary(ciphertext) => result?,
                }
            } else {
                self.connection.send_binary(ciphertext).await?;
            }
            self.next_send = Instant::now() + FRAME_SPACING;
        }
        Ok(())
    }

    pub async fn receive(&mut self) -> Result<WireMessage> {
        loop {
            let ciphertext = self.connection.receive_binary().await?;
            let plaintext = self.channel.decrypt(&ciphertext, now_ms())?;
            if let Some(message) = self.decoder.push(&plaintext)? {
                return Ok(message);
            }
        }
    }

    pub async fn close(&mut self) {
        self.connection.close().await;
    }
}

fn truncated_message(message: &WireMessage) -> Option<WireMessage> {
    let WireBody::Server(response) = &message.body else {
        return None;
    };
    let response = match response {
        ServerMessage::Snapshot {
            session,
            events,
            through_sequence,
            active_run,
            role,
            mutation_high_water,
            ..
        } => ServerMessage::Snapshot {
            session: session.clone(),
            events: Vec::new(),
            through_sequence: *through_sequence,
            gap: Some(truncated_gap(events, *through_sequence)),
            active_run: active_run.clone(),
            role: *role,
            mutation_high_water: *mutation_high_water,
        },
        ServerMessage::Events {
            events,
            through_sequence,
            ..
        } => ServerMessage::Events {
            events: Vec::new(),
            through_sequence: *through_sequence,
            gap: Some(truncated_gap(events, *through_sequence)),
        },
        _ => return None,
    };
    Some(WireMessage::server(response))
}

fn truncated_gap(events: &[Event], through_sequence: u64) -> ReplayGap {
    ReplayGap {
        requested_after_sequence: events
            .first()
            .map_or(through_sequence, |event| event.sequence.saturating_sub(1)),
        available_after_sequence: through_sequence,
        reason: GapReason::OutputTruncated,
    }
}

pub async fn connect_controller(
    account: &RelayAccount,
    identity: &EndpointIdentity,
    invite: &Invitation,
) -> Result<EncryptedSocket> {
    ensure!(
        invite.origin() == &account.origin,
        "invitation belongs to a different relay origin"
    );
    invite.validate_share(now_ms())?;
    let mut connection =
        RelayConnection::connect(account, invite.routing_id(), "controller").await?;
    let mut handshake = NoiseHandshake::controller(identity, invite, now_ms())?;
    timeout(HANDSHAKE_TIMEOUT, async {
        connection.send_binary(handshake.write_message()?).await?;
        handshake.read_message(&connection.receive_binary().await?)?;
        connection.send_binary(handshake.write_message()?).await?;
        Result::<()>::Ok(())
    })
    .await
    .map_err(|_| anyhow!("encrypted pairing timed out"))??;
    let channel = handshake.finish_controller(invite, now_ms())?;
    Ok(EncryptedSocket::new(connection, channel))
}

pub async fn serve_host(
    account: RelayAccount,
    identity: EndpointIdentity,
    mut enrollment: Enrollment,
    backend: LocalBackend,
    cancel: CancellationToken,
    ui: mpsc::Sender<HostNotice>,
) -> Result<()> {
    let result = host_loop(&account, &identity, &mut enrollment, &backend, &cancel, &ui).await;
    // Local revocation precedes best-effort service cleanup and never depends on the relay.
    enrollment.revoke();
    backend
        .stop_share_if(enrollment.invitation().routing_id())
        .await;
    let stop_path = format!("/v1/rooms/{}/stop", enrollment.invitation().routing_id());
    let _ = timeout(
        Duration::from_secs(3),
        account.json(
            reqwest::Method::POST,
            &stop_path,
            Some(serde_json::json!({})),
        ),
    )
    .await;
    if result.is_err() {
        let _ = ui.try_send(HostNotice::Failed);
    }
    result
}

async fn host_loop(
    account: &RelayAccount,
    identity: &EndpointIdentity,
    enrollment: &mut Enrollment,
    backend: &LocalBackend,
    cancel: &CancellationToken,
    ui: &mpsc::Sender<HostNotice>,
) -> Result<()> {
    let mut failures = 0_u32;
    loop {
        if cancel.is_cancelled() {
            return Ok(());
        }
        let Some(session) = backend.shared_session().await else {
            return Ok(());
        };
        if !backend
            .is_current_share(enrollment.invitation().routing_id(), &session, now_ms())
            .await
        {
            return Ok(());
        }
        enrollment.validate_handshake(now_ms())?;
        let connection = tokio::select! {
            biased;
            () = cancel.cancelled() => return Ok(()),
            result = pair_host(account, identity, enrollment, cancel, ui) => result,
        };
        match connection {
            Ok(mut socket) => {
                let session = backend
                    .shared_session()
                    .await
                    .ok_or_else(|| anyhow!("sharing has stopped"))?;
                socket.host_authority = Some(SendAuthority {
                    backend: backend.clone(),
                    epoch: enrollment.invitation().routing_id().into(),
                    session,
                    cancel: cancel.clone(),
                });
                let authenticated_at = Instant::now();
                let _ = ui.try_send(HostNotice::Connected);
                let _outcome =
                    serve_connected(&mut socket, enrollment.invitation(), backend, cancel).await;
                socket.close().await;
                if cancel.is_cancelled() {
                    return Ok(());
                }
                if authenticated_at.elapsed() >= Duration::from_secs(5) {
                    failures = 0;
                } else {
                    failures += 1;
                }
            }
            Err(_) => {
                failures += 1;
            }
        }
        let _ = ui.try_send(HostNotice::Disconnected);
        ensure!(
            failures < MAX_CONSECUTIVE_RETRIES,
            "relay reconnect limit reached; create a new local share"
        );
        let delay = Duration::from_millis(250 * (1_u64 << failures.min(7)));
        tokio::select! { biased; () = cancel.cancelled() => return Ok(()), () = sleep(delay) => {} }
    }
}

async fn pair_host(
    account: &RelayAccount,
    identity: &EndpointIdentity,
    enrollment: &mut Enrollment,
    cancel: &CancellationToken,
    ui: &mpsc::Sender<HostNotice>,
) -> Result<EncryptedSocket> {
    let invite = enrollment.invitation();
    ensure!(
        invite.origin() == &account.origin,
        "sharing belongs to another relay origin"
    );
    let mut connection = RelayConnection::connect(account, invite.routing_id(), "host").await?;
    let _ = ui.try_send(HostNotice::Listening);
    let mut handshake = NoiseHandshake::host(identity, invite, now_ms())?;
    let first_deadline = if enrollment.is_enrolled() {
        invite.share_expires_at_ms()
    } else {
        invite.enrollment_expires_at_ms()
    };
    let first_wait = Duration::from_millis(first_deadline.saturating_sub(now_ms()));
    let first = timeout(first_wait, connection.receive_binary())
        .await
        .map_err(|_| anyhow!("pairing or sharing expired"))??;
    handshake.read_message(&first)?;
    timeout(HANDSHAKE_TIMEOUT, async {
        connection.send_binary(handshake.write_message()?).await?;
        handshake.read_message(&connection.receive_binary().await?)?;
        Result::<()>::Ok(())
    })
    .await
    .map_err(|_| anyhow!("encrypted pairing timed out"))??;
    let candidate = handshake.finish_host()?;
    let locally_confirmed = if let Some(pinned) = enrollment.controller_key() {
        ensure!(
            pinned == candidate.remote_public_key(),
            "another controller is already enrolled"
        );
        false
    } else {
        let (answer, receive) = oneshot::channel();
        timeout(
            Duration::from_secs(1),
            ui.send(HostNotice::Confirm {
                fingerprint: candidate.fingerprint(),
                answer,
            }),
        )
        .await
        .map_err(|_| anyhow!("host confirmation is unavailable"))?
        .map_err(|_| anyhow!("host confirmation is unavailable"))?;
        tokio::select! {
            biased;
            () = cancel.cancelled() => bail!("sharing stopped"),
            result = timeout(Duration::from_secs(60), receive) => result
                .map_err(|_| anyhow!("host confirmation timed out"))?
                .map_err(|_| anyhow!("host confirmation was cancelled"))?,
        }
    };
    let channel = candidate.authorize(enrollment, locally_confirmed, now_ms())?;
    Ok(EncryptedSocket::new(connection, channel))
}

async fn serve_connected(
    socket: &mut EncryptedSocket,
    invite: &Invitation,
    backend: &LocalBackend,
    cancel: &CancellationToken,
) -> Result<()> {
    let session = backend
        .shared_session()
        .await
        .ok_or_else(|| anyhow!("sharing stopped"))?;
    let mut events = backend.subscribe();
    let mut cursor = 0;
    let mut initialized = false;
    let mut flush_at = None;
    let started = now_ms();
    let lease_at = started
        .saturating_add(LEASE_RENEWAL_MS)
        .min(started / DAY_MS * DAY_MS + DAY_MS);
    let lease_deadline = Instant::now() + Duration::from_millis(lease_at.saturating_sub(started));
    let share_deadline = Instant::now()
        + Duration::from_millis(invite.share_expires_at_ms().saturating_sub(started));
    loop {
        ensure!(
            backend
                .is_current_share(invite.routing_id(), &session, now_ms())
                .await,
            "sharing has stopped"
        );
        tokio::select! {
            biased;
            () = cancel.cancelled() => {
                backend.stop_share_if(invite.routing_id()).await;
                let stopped = WireMessage::server(ServerMessage::Stopped { reason: StopReason::LocalStop });
                let _ = timeout(Duration::from_millis(250), socket.send(&stopped)).await;
                return Ok(());
            }
            () = sleep_until(share_deadline) => return Err(anyhow!("sharing expired")),
            () = sleep_until(lease_deadline) => return Ok(()),
            ready = backend.queue_ready(now_ms()) => {
                ready?;
                backend.settle(now_ms()).await?;
            }
            request = socket.receive() => {
                let Ok(request) = request else { return Ok(()); };
                let WireBody::Client(request) = request.body else { bail!("controller sent a server message"); };
                let snapshot = matches!(request, ClientRequest::Snapshot { .. });
                let response = backend.request(invite.routing_id(), request, now_ms()).await
                    .unwrap_or_else(|_| ServerMessage::Error { code: RemoteError::Unavailable, message: "host request is unavailable; refresh session state".into() });
                if let ServerMessage::Snapshot { through_sequence, .. } = &response {
                    cursor = *through_sequence;
                    initialized = true;
                }
                socket.send(&WireMessage::server(response)).await?;
                if snapshot && initialized { flush_at = Some(Instant::now()); }
            }
            () = wait_for_flush(flush_at) => {
                flush_at = None;
                if initialized {
                    let more = flush_events(socket, backend, &session, &mut cursor).await?;
                    if more { flush_at = Some(Instant::now() + Duration::from_millis(OUTPUT_COALESCE_MS)); }
                }
            }
            event = events.recv() => {
                match event {
                    Ok(event) if event.session_id == session => {
                        backend.settle(now_ms()).await?;
                        if initialized && flush_at.is_none() { flush_at = Some(Instant::now() + Duration::from_millis(OUTPUT_COALESCE_MS)); }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        if initialized { flush_at = Some(Instant::now()); }
                    }
                    Err(broadcast::error::RecvError::Closed) => bail!("host event stream stopped"),
                    Ok(_) => {}
                }
            }
        }
    }
}

async fn wait_for_flush(deadline: Option<Instant>) {
    if let Some(deadline) = deadline {
        sleep_until(deadline).await;
    } else {
        pending::<()>().await;
    }
}

async fn flush_events(
    socket: &mut EncryptedSocket,
    backend: &LocalBackend,
    session: &str,
    cursor: &mut u64,
) -> Result<bool> {
    // Four pages per flush bound work; remaining durable events continue after the next 20 ms window.
    for _ in 0..4 {
        let events = backend.events(session, *cursor, MAX_REPLAY_EVENTS).await?;
        if events.is_empty() {
            return Ok(false);
        }
        let full = events.len() == MAX_REPLAY_EVENTS;
        let through_sequence = events.last().map_or(*cursor, |event| event.sequence);
        socket
            .send(&WireMessage::server(ServerMessage::Events {
                events,
                through_sequence,
                gap: None,
            }))
            .await?;
        *cursor = through_sequence;
        if !full {
            return Ok(false);
        }
    }
    Ok(true)
}

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| {
            u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
        })
}

#[cfg(test)]
#[path = "transport_fixture.rs"]
mod worker_fixture;

#[cfg(test)]
#[path = "transport_tls_fixture.rs"]
mod tls_fixture;

#[cfg(test)]
#[expect(
    clippy::unwrap_used,
    reason = "test fixtures should fail at the assertion site"
)]
mod tests {
    use super::super::{
        identity::RelayOrigin,
        invitation::ShareRole,
        protocol::{ExpectedState, Mutation, MutationAction, MutationOutcome},
    };
    use super::*;
    use ejm_harness::EventKind;
    use std::sync::Arc;
    use tokio::{net::TcpListener, sync::Mutex, task::JoinHandle};
    use tokio_tungstenite::connect_async_with_config;

    #[derive(Clone, Copy)]
    enum RelayAttack {
        None,
        Tamper,
        Replay,
        Oversize,
    }

    struct RelayFixture {
        host: EncryptedSocket,
        controller: EncryptedSocket,
        captured: Arc<Mutex<Vec<Vec<u8>>>>,
        relay: JoinHandle<()>,
        enrollment: Enrollment,
    }

    async fn forward_frames(
        listener: TcpListener,
        recording: Arc<Mutex<Vec<Vec<u8>>>>,
        attack: RelayAttack,
    ) {
        let (host, _) = listener.accept().await.unwrap();
        let mut host = tokio_tungstenite::accept_async(host).await.unwrap();
        let (controller, _) = listener.accept().await.unwrap();
        let mut controller = tokio_tungstenite::accept_async(controller).await.unwrap();
        let mut frames = 0;
        loop {
            let (message, from_controller) = tokio::select! {
                message = controller.next() => (message, true),
                message = host.next() => (message, false),
            };
            let Some(Ok(Message::Binary(bytes))) = message else {
                break;
            };
            frames += 1;
            let mut recording = recording.lock().await;
            if recording.len() < 128 {
                recording.push(bytes.to_vec());
            }
            drop(recording);
            let peer = if from_controller {
                &mut host
            } else {
                &mut controller
            };
            let mut bytes = bytes.to_vec();
            if frames == 4 {
                match attack {
                    RelayAttack::Tamper => bytes[0] ^= 1,
                    RelayAttack::Oversize => bytes.resize(MAX_CIPHERTEXT_BYTES + 1, 0),
                    _ => {}
                }
            }
            if peer
                .send(Message::Binary(bytes.clone().into()))
                .await
                .is_err()
            {
                break;
            }
            if frames == 4
                && matches!(attack, RelayAttack::Replay)
                && peer.send(Message::Binary(bytes.into())).await.is_err()
            {
                break;
            }
        }
    }

    async fn fixture(attack: RelayAttack) -> RelayFixture {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let url = format!("ws://{}/", listener.local_addr().unwrap());
        let captured = Arc::new(Mutex::new(Vec::new()));
        let relay = tokio::spawn(forward_frames(listener, Arc::clone(&captured), attack));
        let first = connect_async_with_config(url.as_str(), Some(socket_config()), true)
            .await
            .unwrap()
            .0;
        let second = connect_async_with_config(url.as_str(), Some(socket_config()), true)
            .await
            .unwrap()
            .0;
        let mut host_connection = RelayConnection { socket: first };
        let mut controller_connection = RelayConnection { socket: second };
        let host_identity = EndpointIdentity::memory_only().unwrap();
        let controller_identity = EndpointIdentity::memory_only().unwrap();
        let invitation = Invitation::new(
            RelayOrigin::parse("https://relay.example").unwrap(),
            host_identity.public_key(),
            ShareRole::Control,
            now_ms(),
        )
        .unwrap();
        let mut host = NoiseHandshake::host(&host_identity, &invitation, now_ms()).unwrap();
        let mut controller =
            NoiseHandshake::controller(&controller_identity, &invitation, now_ms()).unwrap();
        controller_connection
            .send_binary(controller.write_message().unwrap())
            .await
            .unwrap();
        host.read_message(&host_connection.receive_binary().await.unwrap())
            .unwrap();
        host_connection
            .send_binary(host.write_message().unwrap())
            .await
            .unwrap();
        controller
            .read_message(&controller_connection.receive_binary().await.unwrap())
            .unwrap();
        controller_connection
            .send_binary(controller.write_message().unwrap())
            .await
            .unwrap();
        host.read_message(&host_connection.receive_binary().await.unwrap())
            .unwrap();
        let controller = controller.finish_controller(&invitation, now_ms()).unwrap();
        let mut enrollment = Enrollment::new(invitation);
        let host = host
            .finish_host()
            .unwrap()
            .authorize(&mut enrollment, true, now_ms())
            .unwrap();
        RelayFixture {
            host: EncryptedSocket::new(host_connection, host),
            controller: EncryptedSocket::new(controller_connection, controller),
            captured,
            relay,
            enrollment,
        }
    }

    fn submitted(text: String) -> WireMessage {
        WireMessage::client(ClientRequest::Mutation {
            mutation: Mutation {
                sequence: 1,
                expected: ExpectedState {
                    session_id: "session-sentinel".into(),
                    run_id: None,
                    event_sequence: 0,
                },
                action: MutationAction::Submit { text },
            },
        })
    }

    #[tokio::test]
    async fn native_websocket_roundtrip_keeps_content_and_invitation_out_of_relay_frames() {
        let mut fixture = fixture(RelayAttack::None).await;
        let sentinel = "private-native-command-sentinel";
        let text = sentinel.repeat(1000);
        let request = submitted(text.clone());
        fixture.controller.send(&request).await.unwrap();
        let received = timeout(Duration::from_secs(3), fixture.host.receive())
            .await
            .unwrap()
            .unwrap();
        let restored = match received.body {
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
        let answer = WireMessage::server(ServerMessage::Outcome {
            sequence: 1,
            outcome: MutationOutcome::Accepted {
                run_id: Some("run-sentinel".into()),
            },
        });
        fixture.host.send(&answer).await.unwrap();
        assert!(matches!(
            fixture.controller.receive().await.unwrap().body,
            WireBody::Server(ServerMessage::Outcome { sequence: 1, .. })
        ));
        let captured = fixture.captured.lock().await;
        for frame in captured.iter() {
            for forbidden in [
                sentinel.as_bytes(),
                b"session-sentinel",
                b"run-sentinel",
                fixture.enrollment.invitation().psk().as_slice(),
            ] {
                assert!(!frame.windows(forbidden.len()).any(|part| part == forbidden));
            }
            assert!(frame.len() <= MAX_CIPHERTEXT_BYTES);
        }
        fixture.relay.abort();
    }

    #[tokio::test]
    async fn malicious_websocket_relay_cannot_tamper_replay_or_exceed_frame_limit() {
        for attack in [
            RelayAttack::Tamper,
            RelayAttack::Replay,
            RelayAttack::Oversize,
        ] {
            let mut fixture = fixture(attack).await;
            fixture
                .controller
                .send(&submitted("execute once".into()))
                .await
                .unwrap();
            if matches!(attack, RelayAttack::Replay) {
                assert!(fixture.host.receive().await.is_ok());
            }
            assert!(
                timeout(Duration::from_secs(3), fixture.host.receive())
                    .await
                    .unwrap()
                    .is_err()
            );
            fixture.relay.abort();
        }
    }

    #[tokio::test]
    async fn revoked_socket_cannot_send_after_same_session_is_shared_again() {
        use ejm_harness::{Harness, HarnessConfig, ReplayProvider, SessionOptions};

        let mut fixture = fixture(RelayAttack::None).await;
        let directory = tempfile::tempdir().unwrap();
        let harness = Harness::new(
            HarnessConfig {
                state_dir: directory.path().join("sessions"),
                ..HarnessConfig::default()
            },
            Arc::new(ReplayProvider::echo()),
            vec![],
        )
        .unwrap();
        let session = harness
            .create_session(SessionOptions::default())
            .await
            .unwrap();
        let backend = LocalBackend::new(harness);
        let epoch = fixture.enrollment.invitation().routing_id().to_owned();
        let expires = fixture.enrollment.invitation().share_expires_at_ms();
        backend
            .begin_share(&session.id, epoch.clone(), ShareRole::Control, expires)
            .await
            .unwrap();
        fixture.host.host_authority = Some(SendAuthority {
            backend: backend.clone(),
            epoch,
            session: session.id.clone(),
            cancel: CancellationToken::new(),
        });
        let response = WireMessage::server(ServerMessage::Outcome {
            sequence: 1,
            outcome: MutationOutcome::Accepted { run_id: None },
        });
        fixture.host.send(&response).await.unwrap();
        assert!(fixture.controller.receive().await.is_ok());
        backend.stop_share().await;
        assert!(fixture.host.send(&response).await.is_err());
        backend
            .begin_share(
                &session.id,
                "different-share-epoch".into(),
                ShareRole::Control,
                expires,
            )
            .await
            .unwrap();
        assert!(fixture.host.send(&response).await.is_err());
        assert!(
            timeout(Duration::from_millis(50), fixture.controller.receive())
                .await
                .is_err()
        );
        backend.shutdown().await.unwrap();
        fixture.relay.abort();
    }

    #[test]
    fn oversized_output_has_an_explicit_gap_without_echoing_payload() {
        let message = WireMessage::server(ServerMessage::Events {
            events: vec![Event {
                sequence: 11,
                session_id: "session".into(),
                run_id: None,
                timestamp_ms: 1000,
                kind: EventKind::TextDelta {
                    text: "private-sentinel".repeat(100_000),
                },
            }],
            through_sequence: 11,
            gap: None,
        });
        assert!(FrameEncoder::new().encode(&message).is_err());
        let bounded = truncated_message(&message).unwrap();
        let encoded = FrameEncoder::new().encode(&bounded).unwrap();
        assert_eq!(encoded.len(), 1);
        assert!(
            !encoded[0]
                .windows(16)
                .any(|part| part == b"private-sentinel")
        );
        let (events, gap, through_sequence) = match bounded.body {
            WireBody::Server(ServerMessage::Events {
                events,
                gap,
                through_sequence,
            }) => Some((events, gap, through_sequence)),
            _ => None,
        }
        .unwrap();
        assert!(events.is_empty());
        assert_eq!(gap.unwrap().reason, GapReason::OutputTruncated);
        assert_eq!(through_sequence, 11);
    }
}
