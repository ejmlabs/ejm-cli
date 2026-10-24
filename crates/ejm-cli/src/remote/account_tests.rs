//! Local HTTP contracts for the native account bridge. No provider or secret store is opened.
#[cfg(test)]
#[path = "account_tls_fixture.rs"]
mod tls_fixture;

#[cfg(test)]
mod contracts {
    use std::{
        convert::Infallible,
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
    };

    use anyhow::Result;
    use http_body_util::Full;
    use hyper::{
        Request, Response,
        body::{Bytes, Incoming},
        server::conn::http1,
        service::service_fn,
    };
    use hyper_util::rt::TokioIo;
    use tokio::{
        net::TcpListener,
        sync::Notify,
        task::{JoinHandle, JoinSet},
    };

    use super::super::*;

    #[derive(Default)]
    struct Observed {
        requests: AtomicUsize,
        redirected: AtomicUsize,
        polls: AtomicUsize,
        pending: Notify,
    }

    struct HttpFixture {
        origin: RelayOrigin,
        observed: Arc<Observed>,
        task: JoinHandle<()>,
    }

    impl HttpFixture {
        async fn new() -> Result<Self> {
            let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).await?;
            let origin =
                RelayOrigin::loopback_for_test(&format!("http://{}", listener.local_addr()?))?;
            let observed = Arc::new(Observed::default());
            let seen = observed.clone();
            let task = tokio::spawn(async move {
                let mut connections = JoinSet::new();
                while let Ok((stream, _)) = listener.accept().await {
                    let observed = seen.clone();
                    connections.spawn(async move {
                        let service = service_fn(move |request| respond(request, observed.clone()));
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

        fn account(&self) -> Result<RelayAccount> {
            RelayAccount::for_test(
                self.origin.clone(),
                "private-test-bearer",
                "account",
                "device",
            )
        }

        fn authorization(
            &self,
            seconds: u64,
            code: &str,
        ) -> Result<StandardDeviceAuthorizationResponse> {
            Ok(serde_json::from_value(serde_json::json!({
                "device_code":code, "user_code":"ABCD-1234",
                "verification_uri": self.origin.url("/approve")?,
                "expires_in":seconds, "interval":0,
            }))?)
        }
    }

    impl Drop for HttpFixture {
        fn drop(&mut self) {
            self.task.abort();
        }
    }

    async fn respond(
        request: Request<Incoming>,
        seen: Arc<Observed>,
    ) -> Result<Response<Full<Bytes>>, Infallible> {
        seen.requests.fetch_add(1, Ordering::SeqCst);
        let mut response = Response::new(Full::new(Bytes::from_static(b"{}")));
        match request.uri().path() {
            "/redirect" => {
                *response.status_mut() = hyper::StatusCode::TEMPORARY_REDIRECT;
                response
                    .headers_mut()
                    .insert("location", hyper::header::HeaderValue::from_static("/sink"));
            }
            "/sink" => {
                seen.redirected.fetch_add(1, Ordering::SeqCst);
            }
            "/oversized" => {
                *response.body_mut() = Full::new(Bytes::from(vec![b'x'; MAX_RESPONSE + 1]));
            }
            "/api/auth/device/token" => {
                let body = http_body_util::BodyExt::collect(request.into_body()).await;
                let pending = body.is_ok_and(|body| {
                    body.to_bytes()
                        .windows(12)
                        .any(|part| part == b"hold-fixture")
                });
                if pending {
                    seen.pending.notify_one();
                    std::future::pending::<()>().await;
                }
                let poll = seen.polls.fetch_add(1, Ordering::SeqCst);
                if poll == 0 {
                    *response.status_mut() = hyper::StatusCode::BAD_REQUEST;
                    *response.body_mut() =
                        Full::new(Bytes::from_static(br#"{"error":"slow_down"}"#));
                } else {
                    *response.body_mut() = Full::new(Bytes::from_static(br#"{"access_token":"private-fixture-token","token_type":"Bearer","expires_in":3600}"#));
                }
                response.headers_mut().insert(
                    "content-type",
                    hyper::header::HeaderValue::from_static("application/json"),
                );
            }
            _ => {}
        }
        Ok(response)
    }

    #[tokio::test]
    async fn credentials_never_follow_redirects_and_account_responses_stay_bounded() -> Result<()> {
        let fixture = HttpFixture::new().await?;
        let account = fixture.account()?;
        assert!(
            account
                .json(reqwest::Method::GET, "/redirect", None)
                .await
                .is_err()
        );
        assert_eq!(fixture.observed.redirected.load(Ordering::SeqCst), 0);
        assert!(
            account
                .json(reqwest::Method::GET, "/oversized", None)
                .await
                .is_err()
        );
        assert!(
            http_client()?
                .get(fixture.origin.url("/")?)
                .send()
                .await
                .is_err(),
            "production HTTP client must refuse the plaintext loopback fixture"
        );
        Ok(())
    }

    #[tokio::test]
    async fn oauth_bridge_rejects_cross_origin_before_sending_and_refuses_redirects() -> Result<()>
    {
        let fixture = HttpFixture::new().await?;
        let account = fixture.account()?;
        let request = oauth2::http::Request::builder()
            .uri("https://other.invalid/api/auth/device/token")
            .body(b"private-device-code".to_vec())?;
        assert!(
            oauth_http(&account.client, &fixture.origin, request)
                .await
                .is_err()
        );
        assert_eq!(fixture.observed.requests.load(Ordering::SeqCst), 0);
        for route in ["/redirect", "/oversized"] {
            let request = oauth2::http::Request::builder()
                .uri(fixture.origin.url(route)?.as_str())
                .body(b"private-device-code".to_vec())?;
            assert!(
                oauth_http(&account.client, &fixture.origin, request)
                    .await
                    .is_err()
            );
        }
        assert_eq!(fixture.observed.redirected.load(Ordering::SeqCst), 0);
        Ok(())
    }

    #[tokio::test]
    async fn device_polling_honors_slowdown_expiry_and_inflight_cancellation() -> Result<()> {
        let fixture = HttpFixture::new().await?;
        let account = fixture.account()?;
        let details = fixture.authorization(30, "slow-fixture")?;
        let started = std::time::Instant::now();
        let token = device_token(
            &fixture.origin,
            &account.client,
            &details,
            tokio::time::Instant::now() + details.expires_in(),
            CancellationToken::new(),
        )
        .await?;
        assert_eq!(token.expose_secret(), "private-fixture-token");
        assert_eq!(fixture.observed.polls.load(Ordering::SeqCst), 2);
        assert!(
            started.elapsed() >= Duration::from_secs(5),
            "server slowdown must delay another poll"
        );

        let before = fixture.observed.requests.load(Ordering::SeqCst);
        assert!(
            device_token(
                &fixture.origin,
                &account.client,
                &fixture.authorization(0, "expired")?,
                tokio::time::Instant::now(),
                CancellationToken::new()
            )
            .await
            .is_err()
        );
        assert_eq!(fixture.observed.requests.load(Ordering::SeqCst), before);

        let cancel = CancellationToken::new();
        let held = fixture.authorization(30, "hold-fixture")?;
        let polling = device_token(
            &fixture.origin,
            &account.client,
            &held,
            tokio::time::Instant::now() + held.expires_in(),
            cancel.clone(),
        );
        tokio::pin!(polling);
        tokio::select! {
            result = &mut polling => { result?; anyhow::bail!("poll completed before cancellation"); }
            () = fixture.observed.pending.notified() => {}
        }
        cancel.cancel();
        assert!(
            tokio::time::timeout(Duration::from_secs(1), polling)
                .await?
                .is_err()
        );
        Ok(())
    }
}
