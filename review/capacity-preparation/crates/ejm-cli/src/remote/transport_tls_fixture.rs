//! Real WebSocket TLS handshakes protect the relay ticket boundary.

use std::{net::Ipv4Addr, sync::Arc, time::Duration};

use anyhow::{Result, anyhow, ensure};
use futures_util::{SinkExt, StreamExt};
use tokio::{net::TcpListener, task::JoinSet, time::timeout};
use tokio_rustls::{
    TlsAcceptor,
    rustls::{
        RootCertStore, ServerConfig,
        pki_types::{CertificateDer, PrivatePkcs8KeyDer},
    },
};
use tokio_tungstenite::{
    accept_hdr_async,
    tungstenite::{
        Message,
        client::IntoClientRequest,
        handshake::server::{Callback, ErrorResponse, Response},
    },
};

use super::{RelayConnection, Request, bundled_tls_connector, tls_connector};

const CERTIFICATE: &[u8] = include_bytes!("testdata/untrusted-relay-cert.der");
const PRIVATE_KEY: &[u8] = include_bytes!("testdata/untrusted-relay-key.der");
const FIXTURE_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Default)]
struct Upgrades {
    count: usize,
    tickets: usize,
}

impl Callback for &mut Upgrades {
    fn on_request(self, request: &Request, response: Response) -> Result<Response, ErrorResponse> {
        self.count += 1;
        if request
            .headers()
            .get("authorization")
            .is_some_and(|value| value == "Bearer fixture-ticket")
        {
            self.tickets += 1;
        }
        Ok(response)
    }
}

fn trusted_connector() -> Result<tokio_tungstenite::Connector> {
    let mut roots = RootCertStore::empty();
    roots.add(CertificateDer::from(CERTIFICATE.to_vec()))?;
    tls_connector(roots)
}

fn request(host: &str, port: u16) -> Result<Request> {
    let mut request = format!("wss://{host}:{port}/ticket-boundary").into_client_request()?;
    request
        .headers_mut()
        .insert("authorization", "Bearer fixture-ticket".parse()?);
    Ok(request)
}

#[tokio::test]
async fn websocket_tls_verifies_chain_and_name_before_sending_ticket() -> Result<()> {
    let provider = tokio_rustls::rustls::crypto::aws_lc_rs::default_provider();
    let config = ServerConfig::builder_with_provider(Arc::new(provider))
        .with_safe_default_protocol_versions()?
        .with_no_client_auth()
        .with_single_cert(
            vec![CertificateDer::from(CERTIFICATE.to_vec())],
            PrivatePkcs8KeyDer::from(PRIVATE_KEY.to_vec()).into(),
        )?;
    let acceptor = TlsAcceptor::from(Arc::new(config));
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await?;
    let port = listener.local_addr()?.port();
    let mut peers = JoinSet::new();
    peers.spawn(async move {
        let mut rejected = 0;
        let mut upgrades = Upgrades::default();
        for _ in 0..3 {
            let (stream, _) = timeout(FIXTURE_TIMEOUT, listener.accept()).await??;
            let Ok(stream) = timeout(FIXTURE_TIMEOUT, acceptor.accept(stream)).await? else {
                rejected += 1;
                continue;
            };
            let mut socket =
                timeout(FIXTURE_TIMEOUT, accept_hdr_async(stream, &mut upgrades)).await??;
            let message = timeout(FIXTURE_TIMEOUT, socket.next())
                .await?
                .ok_or_else(|| anyhow!("fixture connection closed before its frame"))??;
            ensure!(
                matches!(&message, Message::Binary(bytes) if bytes.as_ref() == b"bounded-frame"),
                "fixture received an unexpected frame"
            );
            timeout(FIXTURE_TIMEOUT, socket.send(message)).await??;
            timeout(FIXTURE_TIMEOUT, socket.close(None)).await??;
        }
        Ok::<_, anyhow::Error>((rejected, upgrades.count, upgrades.tickets))
    });

    let result = async {
        // Use the same bundled connector and request path as production.
        let rejected = timeout(
            FIXTURE_TIMEOUT,
            RelayConnection::connect_request(request("127.0.0.1", port)?, bundled_tls_connector()?),
        )
        .await?;
        ensure!(
            rejected.is_err(),
            "untrusted WebSocket certificate was accepted"
        );

        let rejected = timeout(
            FIXTURE_TIMEOUT,
            RelayConnection::connect_request(request("localhost", port)?, trusted_connector()?),
        )
        .await?;
        ensure!(
            rejected.is_err(),
            "wrong WebSocket certificate name was accepted"
        );

        // Trust only the fixture certificate. This must complete a real upgrade
        // and exchange a frame before either rejection counts as valid evidence.
        let mut connection = timeout(
            FIXTURE_TIMEOUT,
            RelayConnection::connect_request(request("127.0.0.1", port)?, trusted_connector()?),
        )
        .await??;
        connection.send_binary(b"bounded-frame".to_vec()).await?;
        ensure!(
            timeout(FIXTURE_TIMEOUT, connection.receive_binary()).await?? == b"bounded-frame",
            "WebSocket TLS positive control failed"
        );
        connection.close().await;
        let observed = timeout(FIXTURE_TIMEOUT, peers.join_next())
            .await?
            .ok_or_else(|| anyhow!("fixture task was missing"))???;
        ensure!(
            observed == (2, 1, 1),
            "certificate rejection exposed an upgrade or ticket"
        );
        Ok::<(), anyhow::Error>(())
    }
    .await;
    peers.abort_all();
    while peers.join_next().await.is_some() {}
    result
}
