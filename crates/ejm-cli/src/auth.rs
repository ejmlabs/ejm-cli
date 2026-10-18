use std::{convert::Infallible, sync::Arc, time::Duration};

use anyhow::{Context, Result, bail, ensure};
use ejm_harness_auth::{
    AuthConfig, AuthEngine, CredentialSource, FileStore, LoginAttempt, RegistrationStatus,
    RetryRequired, SecretString,
};
use http_body_util::Full;
use hyper::{
    Method, Request, Response, StatusCode,
    body::Bytes,
    header::{CACHE_CONTROL, CONNECTION, CONTENT_TYPE, HOST, HeaderValue, REFERRER_POLICY},
    server::conn::http1,
    service::service_fn,
};
use hyper_util::rt::TokioIo;
use tokio::{net::TcpListener, sync::mpsc};
use tokio_util::sync::CancellationToken;
use url::Url;

use crate::{
    args::{Args, ProviderKind},
    browser,
    render::safe_text,
};

pub fn engine(args: &Args) -> Result<AuthEngine> {
    let store = Arc::new(FileStore::open(args.state_dir().join("auth"))?);
    AuthEngine::new(
        AuthConfig {
            agent_name: "ejm-cli".into(),
        },
        store,
    )
}

pub async fn login(args: &Args, service: ProviderKind, no_browser: bool) -> Result<bool> {
    match service {
        ProviderKind::Openai | ProviderKind::Responses => {
            println!(
                "OpenAI API access uses EJM_CLI_API_KEY or OPENAI_API_KEY from your environment."
            );
            println!("For a ChatGPT account, use: ejm-cli login chatgpt");
            return Ok(true);
        }
        ProviderKind::Anthropic => {
            println!("Claude subscription sign-in stays in the official Claude Code client.");
            println!(
                "Use Claude Code's own login flow: https://code.claude.com/docs/en/authentication"
            );
            println!("ejm-cli does not collect or reuse Claude Code subscription tokens.");
            println!(
                "For Anthropic API access, set ANTHROPIC_API_KEY and choose --provider anthropic --model MODEL."
            );
            return Ok(true);
        }
        ProviderKind::Chatgpt => {}
    }
    let engine = engine(args)?;
    let cancel = CancellationToken::new();
    let flow = login_flow(&engine, args, no_browser, cancel.clone());
    tokio::pin!(flow);
    tokio::select! {
        biased;
        signal = tokio::signal::ctrl_c() => {
            signal.context("listen for sign-in cancellation")?;
            cancel.cancel();
            let _ = flow.await;
            eprintln!("Sign-in cancelled.");
            Ok(false)
        }
        result = &mut flow => result,
    }
}

struct LoginGuard {
    engine: AuthEngine,
    attempt: String,
    cancel: CancellationToken,
}

impl Drop for LoginGuard {
    fn drop(&mut self) {
        self.cancel.cancel();
        let _ = self.engine.cancel(&self.attempt);
    }
}

async fn login_flow(
    engine: &AuthEngine,
    args: &Args,
    no_browser: bool,
    cancel: CancellationToken,
) -> Result<bool> {
    let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
        .await
        .context("start the private browser callback listener")?;
    let redirect = format!(
        "http://127.0.0.1:{}/auth/callback",
        listener.local_addr()?.port()
    );
    let registration = match &args.account {
        Some(id) => Some(select_registration(engine, Some(id)).await?.id),
        None => None,
    };
    eprintln!("Starting ChatGPT sign-in. Ctrl+C cancels.");
    let attempt = engine
        .start(&redirect, registration.as_deref(), cancel.clone())
        .await?;
    let mut guard = LoginGuard {
        engine: engine.clone(),
        attempt: attempt.id.clone(),
        cancel: cancel.clone(),
    };
    let status = complete_login(&mut guard, &listener, &redirect, attempt, no_browser).await?;
    println!("Signed in: {}", account_label(&status));
    println!("Registration: {}", safe_text(&status.id));
    if !status.plan_enabled {
        println!("This account does not report an enabled ChatGPT plan.");
    }
    Ok(true)
}

async fn complete_login(
    guard: &mut LoginGuard,
    listener: &TcpListener,
    redirect: &str,
    mut attempt: LoginAttempt,
    no_browser: bool,
) -> Result<RegistrationStatus> {
    let mut retried = false;
    loop {
        safe_authorization_url(attempt.authorization_url.expose_secret())?;
        let state = authorization_state(attempt.authorization_url.expose_secret())?;
        open_browser(attempt.authorization_url, no_browser, guard.cancel.clone()).await?;
        let callback = tokio::time::timeout(
            Duration::from_secs(600),
            wait_callback(
                listener,
                redirect,
                state.expose_secret(),
                guard.cancel.clone(),
            ),
        )
        .await
        .context("sign-in timed out; run ejm-cli login chatgpt to try again")??;
        let result = guard
            .engine
            .complete(
                &guard.attempt,
                callback.expose_secret(),
                guard.cancel.clone(),
            )
            .await;
        match result {
            Ok(status) => return Ok(status),
            Err(error) => {
                if !retried && let Some(retry) = error.downcast_ref::<RetryRequired>() {
                    ensure!(
                        retry.attempt_id == guard.attempt,
                        "sign-in retry attempt mismatch"
                    );
                    eprintln!("The provider requested one more sign-in step. Ctrl+C cancels.");
                    attempt = guard
                        .engine
                        .retry(&guard.attempt, redirect, guard.cancel.clone())
                        .await?;
                    guard.attempt.clone_from(&attempt.id);
                    retried = true;
                    continue;
                }
                return Err(error).context("complete ChatGPT sign-in");
            }
        }
    }
}

fn safe_authorization_url(value: &str) -> Result<()> {
    ensure!(
        !value.chars().any(char::is_control),
        "authorization URL contains control characters"
    );
    let url = Url::parse(value).context("invalid authorization URL")?;
    ensure!(
        url.scheme() == "https"
            && url.host_str() == Some("auth.openai.com")
            && url.port_or_known_default() == Some(443)
            && url.username().is_empty()
            && url.password().is_none()
            && url.fragment().is_none(),
        "the provider returned an unsafe authorization URL"
    );
    ensure!(
        !url.query_pairs().any(|(key, _)| matches!(
            key.to_ascii_lowercase().as_str(),
            "access_token"
                | "refresh_token"
                | "id_token"
                | "id_token_hint"
                | "token"
                | "client_secret"
        )),
        "authorization URLs must not contain credentials"
    );
    Ok(())
}

fn authorization_state(value: &str) -> Result<SecretString> {
    let url = Url::parse(value)?;
    let states: Vec<_> = url
        .query_pairs()
        .filter(|(key, _)| key == "state")
        .collect();
    ensure!(
        states.len() == 1 && !states[0].1.is_empty(),
        "authorization state is missing or duplicated"
    );
    Ok(SecretString::new(states[0].1.as_ref()))
}

async fn open_browser(url: SecretString, manual: bool, cancel: CancellationToken) -> Result<()> {
    if manual {
        println!(
            "Open this sign-in link in your browser:\n{}",
            url.expose_secret()
        );
    } else {
        eprintln!("Opening your browser. Waiting for sign-in…");
        browser::launch(url, cancel).await?;
    }
    Ok(())
}

async fn wait_callback(
    listener: &TcpListener,
    redirect: &str,
    state: &str,
    cancel: CancellationToken,
) -> Result<SecretString> {
    for _ in 0..32 {
        let (stream, _) = tokio::select! {
            biased;
            () = cancel.cancelled() => bail!("sign-in cancelled"),
            result = listener.accept() => result.context("accept the browser callback")?,
        };
        let (sender, mut receiver) = mpsc::channel(1);
        let service = service_fn(move |request| {
            let response = match validate_callback(&request, redirect, state) {
                Ok(Some(uri)) => match sender.try_send(uri) {
                    Ok(()) => callback_response(
                        StatusCode::OK,
                        "Browser step received. Return to ejm-cli to finish sign-in and close this tab.",
                    ),
                    Err(_) => {
                        callback_response(StatusCode::BAD_REQUEST, "Invalid sign-in callback.")
                    }
                },
                Ok(None) => callback_response(StatusCode::NOT_FOUND, "Not found."),
                _ => callback_response(StatusCode::BAD_REQUEST, "Invalid sign-in callback."),
            };
            async move { Ok::<_, Infallible>(response) }
        });
        let mut http = http1::Builder::new();
        http.keep_alive(false).max_headers(32).max_buf_size(8192);
        tokio::select! {
            biased;
            () = cancel.cancelled() => bail!("sign-in cancelled"),
            _ = tokio::time::timeout(
                Duration::from_secs(3),
                http.serve_connection(TokioIo::new(stream), service),
            ) => {}
        }
        if let Ok(uri) = receiver.try_recv() {
            return Ok(uri);
        }
    }
    bail!("too many invalid browser callbacks; restart sign-in")
}

fn validate_callback<B>(
    request: &Request<B>,
    redirect: &str,
    state: &str,
) -> Result<Option<SecretString>> {
    let redirect = Url::parse(redirect)?;
    ensure!(request.method() == Method::GET, "callback must use GET");
    ensure!(
        request.uri().scheme().is_none() && request.uri().authority().is_none(),
        "callback must use an origin-form target"
    );
    let authority = format!(
        "127.0.0.1:{}",
        redirect.port().context("callback port missing")?
    );
    let mut hosts = request.headers().get_all(HOST).iter();
    let host = hosts.next().context("callback host missing")?;
    ensure!(
        hosts.next().is_none() && host.to_str().context("invalid callback host")? == authority,
        "callback host mismatch"
    );
    if request.uri().path() != "/auth/callback" {
        return Ok(None);
    }
    let target = request
        .uri()
        .path_and_query()
        .context("callback target missing")?;
    ensure!(!target.as_str().contains('#'), "invalid callback fragment");
    let callback = redirect.join(target.as_str())?;
    ensure!(
        callback.origin() == redirect.origin(),
        "callback origin mismatch"
    );
    let states: Vec<_> = callback
        .query_pairs()
        .filter(|(key, _)| key == "state")
        .collect();
    ensure!(
        states.len() == 1 && states[0].1 == state,
        "callback state mismatch"
    );
    Ok(Some(SecretString::new(callback.to_string())))
}

fn callback_response(status: StatusCode, message: &'static str) -> Response<Full<Bytes>> {
    let mut response = Response::new(Full::new(Bytes::from_static(message.as_bytes())));
    *response.status_mut() = status;
    response.headers_mut().extend([
        (
            CONTENT_TYPE,
            HeaderValue::from_static("text/plain; charset=utf-8"),
        ),
        (CACHE_CONTROL, HeaderValue::from_static("no-store")),
        (REFERRER_POLICY, HeaderValue::from_static("no-referrer")),
        (CONNECTION, HeaderValue::from_static("close")),
    ]);
    response
}

pub async fn accounts(args: &Args, json: bool) -> Result<bool> {
    let accounts = engine(args)?
        .registrations(CancellationToken::new())
        .await?;
    if json {
        println!("{}", serde_json::to_string(&accounts)?);
    } else if accounts.is_empty() {
        println!("No saved accounts. Sign in with: ejm-cli login chatgpt");
    } else {
        println!("{:<36}  {:<12}  ACCOUNT", "REGISTRATION", "STATUS");
        for account in accounts {
            println!(
                "{:<36}  {:<12}  {}",
                safe_text(&account.id),
                if account.signed_in {
                    "signed in"
                } else {
                    "signed out"
                },
                account_label(&account)
            );
        }
    }
    Ok(true)
}

pub async fn logout(args: &Args, registration: &str) -> Result<bool> {
    let engine = engine(args)?;
    let registration = select_registration(&engine, Some(registration)).await?;
    let result = engine
        .logout(&registration.id, CancellationToken::new())
        .await?;
    println!(
        "Removed saved registration {}.",
        safe_text(&registration.id)
    );
    if !result.remote_revoked {
        println!(
            "Provider revocation was not confirmed. Review connected apps in your provider account."
        );
    }
    Ok(true)
}

pub async fn credentials(args: &Args) -> Result<Arc<dyn CredentialSource>> {
    let engine = engine(args)?;
    let registration = select_registration(&engine, args.account.as_deref()).await?;
    Ok(engine.source(&registration.id))
}

async fn select_registration(
    engine: &AuthEngine,
    selected: Option<&str>,
) -> Result<RegistrationStatus> {
    let accounts = engine.registrations(CancellationToken::new()).await?;
    let mut matching = accounts
        .into_iter()
        .filter(|account| selected.is_none_or(|id| account.id.starts_with(id)));
    let account = matching
        .next()
        .context("no matching saved account; run ejm-cli login chatgpt")?;
    ensure!(
        matching.next().is_none(),
        "multiple saved accounts match; choose --account REGISTRATION from ejm-cli accounts"
    );
    Ok(account)
}

fn account_label(status: &RegistrationStatus) -> String {
    safe_text(status.email.as_deref().unwrap_or(&status.subject)).replace('\n', " ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpStream,
    };

    #[test]
    fn authorization_link_rejects_token_hints_and_foreign_origins() -> Result<()> {
        safe_authorization_url(
            "https://auth.openai.com/authorize?state=fixture&code_challenge=fixture",
        )?;
        for value in [
            "https://auth.openai.com/authorize?id_token_hint=fixture-secret",
            "https://auth.openai.com/authorize?access_token=fixture-secret",
            "https://other.test/authorize?state=fixture",
        ] {
            let error = safe_authorization_url(value)
                .err()
                .context("unsafe link was accepted")?;
            ensure!(
                !error.to_string().contains("fixture-secret"),
                "secret appeared in a validation error"
            );
        }
        Ok(())
    }

    #[test]
    fn callback_requires_exact_loopback_host_and_does_not_leak_codes() -> Result<()> {
        let redirect = "http://127.0.0.1:12345/auth/callback";
        let request = Request::get("/auth/callback?code=fixture-secret&state=fixture")
            .header(HOST, "127.0.0.1:12345")
            .body(())?;
        let callback =
            validate_callback(&request, redirect, "fixture")?.context("callback missing")?;
        ensure!(
            callback.expose_secret().starts_with(redirect),
            "wrong callback origin"
        );
        let request = Request::get("/auth/callback?code=fixture-secret")
            .header(HOST, "other.test")
            .body(())?;
        let error = validate_callback(&request, redirect, "fixture")
            .err()
            .context("foreign host accepted")?;
        ensure!(
            !error.to_string().contains("fixture-secret"),
            "callback code leaked"
        );
        let request = Request::get("/favicon.ico")
            .header(HOST, "127.0.0.1:12345")
            .body(())?;
        ensure!(
            validate_callback(&request, redirect, "fixture")?.is_none(),
            "favicon treated as callback"
        );
        Ok(())
    }

    #[tokio::test]
    async fn invalid_callback_keeps_the_listener_available_for_the_real_browser() -> Result<()> {
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).await?;
        let address = listener.local_addr()?;
        let redirect = format!("http://{address}/auth/callback");
        let server = async {
            let callback =
                wait_callback(&listener, &redirect, "fixture", CancellationToken::new()).await?;
            ensure!(
                callback.expose_secret().contains("code=fixture-code"),
                "accepted the wrong callback"
            );
            Ok::<(), anyhow::Error>(())
        };
        let client = async {
            for (state, status) in [("wrong", "400 Bad Request"), ("fixture", "200 OK")] {
                let mut stream = TcpStream::connect(address).await?;
                stream
                    .write_all(
                        format!(
                            "GET /auth/callback?code=fixture-code&state={state} HTTP/1.1\r\nHost: {address}\r\n\r\n"
                        )
                        .as_bytes(),
                    )
                    .await?;
                let mut response = Vec::new();
                stream.take(1024).read_to_end(&mut response).await?;
                ensure!(
                    response.starts_with(format!("HTTP/1.1 {status}\r\n").as_bytes()),
                    "wrong callback response status"
                );
                ensure!(
                    !response.windows(12).any(|value| value == b"fixture-code"),
                    "callback response exposed the code"
                );
            }
            Ok::<(), anyhow::Error>(())
        };
        tokio::time::timeout(Duration::from_secs(5), async {
            tokio::try_join!(server, client)?;
            Ok::<(), anyhow::Error>(())
        })
        .await??;
        Ok(())
    }
}
