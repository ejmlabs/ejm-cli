//! A real, untrusted local TLS peer for native account and socket credentials.
#![cfg(test)]

use std::{
    convert::Infallible,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use anyhow::Result;
use http_body_util::Full;
use hyper::{Response, body::Bytes, server::conn::http1, service::service_fn};
use hyper_util::rt::TokioIo;
use tokio::{
    net::TcpListener,
    sync::Notify,
    task::{JoinHandle, JoinSet},
};
use tokio_rustls::{
    TlsAcceptor,
    rustls::{
        ServerConfig,
        pki_types::{CertificateDer, PrivatePkcs8KeyDer},
    },
};
use tokio_tungstenite::{connect_async_with_config, tungstenite::client::IntoClientRequest};

use super::super::{RelayOrigin, http_client, oauth_http};

const CERTIFICATE: &[u8] = include_bytes!("testdata/untrusted-relay-cert.der");
const PRIVATE_KEY: &[u8] = include_bytes!("testdata/untrusted-relay-key.der");

#[derive(Default)]
struct Observed {
    rejected_handshakes: AtomicUsize,
    application_requests: AtomicUsize,
    changed: Notify,
}

struct TlsFixture {
    origin: RelayOrigin,
    observed: Arc<Observed>,
    task: JoinHandle<()>,
}

impl TlsFixture {
    async fn new() -> Result<Self> {
        let provider = tokio_rustls::rustls::crypto::aws_lc_rs::default_provider();
        let config = ServerConfig::builder_with_provider(Arc::new(provider))
            .with_safe_default_protocol_versions()?
            .with_no_client_auth()
            .with_single_cert(
                vec![CertificateDer::from(CERTIFICATE.to_vec())],
                PrivatePkcs8KeyDer::from(PRIVATE_KEY.to_vec()).into(),
            )?;
        let acceptor = TlsAcceptor::from(Arc::new(config));
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).await?;
        let origin = RelayOrigin::parse(&format!("https://{}", listener.local_addr()?))?;
        let observed = Arc::new(Observed::default());
        let seen = observed.clone();
        let task = tokio::spawn(async move {
            let mut connections = JoinSet::new();
            while let Ok((stream, _)) = listener.accept().await {
                let acceptor = acceptor.clone();
                let seen = seen.clone();
                connections.spawn(async move {
                    let Ok(stream) = acceptor.accept(stream).await else {
                        seen.rejected_handshakes.fetch_add(1, Ordering::SeqCst);
                        seen.changed.notify_one();
                        return;
                    };
                    let service = service_fn(move |_| {
                        seen.application_requests.fetch_add(1, Ordering::SeqCst);
                        async {
                            Ok::<_, Infallible>(Response::new(Full::new(Bytes::from_static(b"{}"))))
                        }
                    });
                    let _ = http1::Builder::new()
                        .serve_connection(TokioIo::new(stream), service)
                        .await;
                });
            }
        });
        Ok(Self {
            origin,
            observed,
            task,
        })
    }

    async fn rejected(&self, count: usize) -> Result<()> {
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let changed = self.observed.changed.notified();
                if self.observed.rejected_handshakes.load(Ordering::SeqCst) >= count {
                    return;
                }
                changed.await;
            }
        })
        .await?;
        Ok(())
    }
}

impl Drop for TlsFixture {
    fn drop(&mut self) {
        self.task.abort();
    }
}

#[tokio::test]
async fn invalid_certificate_blocks_bearer_oauth_and_ticket_before_application_data() -> Result<()>
{
    let fixture = TlsFixture::new().await?;
    let client = http_client()?;
    let bearer = client
        .post(fixture.origin.url("/v1/account")?)
        .bearer_auth("private-invalid-certificate-bearer")
        .body("private-invalid-certificate-account-body")
        .send();
    assert!(
        tokio::time::timeout(Duration::from_secs(5), bearer)
            .await?
            .is_err()
    );
    let request = oauth2::http::Request::builder()
        .method("POST")
        .uri(fixture.origin.url("/api/auth/device/token")?.as_str())
        .header("content-type", "application/x-www-form-urlencoded")
        .body(b"device_code=private-invalid-certificate-device-code".to_vec())?;
    assert!(
        tokio::time::timeout(
            Duration::from_secs(5),
            oauth_http(&client, &fixture.origin, request)
        )
        .await?
        .is_err()
    );
    let mut socket_url = fixture.origin.url("/v1/rooms/cert-fixture/controller")?;
    assert!(socket_url.set_scheme("wss").is_ok());
    let mut request = socket_url.as_str().into_client_request()?;
    request.headers_mut().insert(
        "authorization",
        "Bearer private-invalid-certificate-ticket".parse()?,
    );
    assert!(
        tokio::time::timeout(
            Duration::from_secs(5),
            connect_async_with_config(request, None, true)
        )
        .await?
        .is_err()
    );
    fixture.rejected(3).await?;
    assert_eq!(
        fixture.observed.application_requests.load(Ordering::SeqCst),
        0
    );

    // Trust only this certificate for the probe, so a broken TLS fixture cannot pass.
    let trusted = reqwest::Client::builder()
        .https_only(true)
        .tls_certs_only([reqwest::Certificate::from_der(CERTIFICATE)?])
        .build()?;
    let response = tokio::time::timeout(
        Duration::from_secs(5),
        trusted.get(fixture.origin.url("/probe")?).send(),
    )
    .await??;
    assert!(response.status().is_success());
    assert_eq!(
        fixture.observed.application_requests.load(Ordering::SeqCst),
        1
    );
    Ok(())
}
