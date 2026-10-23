use std::{convert::Infallible, sync::Arc, time::Duration};

use anyhow::{Context, Result, bail, ensure};
use ejm_harness_auth::{
    AuthConfig, AuthEngine, ClaudeCodeClient, ClaudeCodeStatus, CredentialSource, FileStore,
    LoginAttempt, LogoutResult, RegistrationStatus, RetryRequired, SecretString,
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
use serde::Serialize;
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

pub fn claude_client(args: &Args) -> ClaudeCodeClient {
    ClaudeCodeClient::new(args.claude_program())
}

pub struct AccountSummary {
    pub chatgpt: Vec<RegistrationStatus>,
    pub chatgpt_unavailable: bool,
    pub claude: ClaudeAccount,
}

pub const VAULT_RECOVERY: &str = "ChatGPT: encrypted account vault is unavailable, locked or invalid. Restore its original native key access and retry. Do not replace a missing key.";

#[derive(Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum ClaudeAccount {
    Unavailable,
    Available {
        #[serde(flatten)]
        status: ClaudeCodeStatus,
    },
}

impl ClaudeAccount {
    pub fn label(&self) -> String {
        match self {
            Self::Unavailable => {
                "Claude: client unavailable. Install Claude Code or set --claude-path.".into()
            }
            Self::Available { status } if !status.logged_in => {
                "Claude: signed out. Choose Claude in /login or run ejm-cli login claude.".into()
            }
            Self::Available { status } if status.require_subscription().is_ok() => {
                let account = status.email.as_deref().unwrap_or("current account");
                let plan = status
                    .subscription_type
                    .as_deref()
                    .unwrap_or("subscription");
                format!(
                    "Claude: signed in, {} ({}). Official client subscription.",
                    safe_text(account),
                    safe_text(plan)
                )
            }
            Self::Available { status } => format!(
                "Claude: {} authentication is not subscription access. Choose Claude in /login.",
                safe_text(&status.auth_method)
            ),
        }
    }
}

pub async fn account_summary(args: &Args, cancel: CancellationToken) -> Result<AccountSummary> {
    let result = registrations(args, cancel.child_token()).await;
    let chatgpt_unavailable = result.is_err();
    let chatgpt = result.unwrap_or_default();
    let claude = match claude_client(args).status(cancel.child_token()).await {
        Ok(status) => ClaudeAccount::Available { status },
        Err(_) => ClaudeAccount::Unavailable,
    };
    ensure!(!cancel.is_cancelled(), "account lookup cancelled");
    Ok(AccountSummary {
        chatgpt,
        chatgpt_unavailable,
        claude,
    })
}

pub async fn claude_login(args: &Args, cancel: CancellationToken) -> Result<ClaudeCodeStatus> {
    crate::models::validate_claude(args)?;
    let status = claude_client(args).login(cancel).await?;
    status.require_subscription()?;
    Ok(status)
}

async fn claude_shell_action(args: &Args, logout: bool) -> Result<bool> {
    let cancel = CancellationToken::new();
    let operation = async {
        if logout {
            claude_client(args).logout(cancel.clone()).await
        } else {
            claude_login(args, cancel.clone()).await
        }
    };
    tokio::pin!(operation);
    let result = tokio::select! {
        biased;
        () = auth_signal() => {
            cancel.cancel();
            let result = operation.await;
            if !logout || result.is_err() {
                eprintln!("Claude sign-in command cancelled.");
                return Ok(false);
            }
            result
        }
        result = &mut operation => result,
    }?;
    if logout {
        println!("Signed out of Claude in the official client.");
    } else {
        println!("{}", ClaudeAccount::Available { status: result }.label());
    }
    Ok(true)
}

async fn auth_signal() {
    crate::ui::termination_signal().await;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoginProgress {
    Starting,
    OpeningBrowser,
    Retrying,
}

impl LoginProgress {
    pub fn message(self) -> &'static str {
        match self {
            Self::Starting => "Starting ChatGPT sign-in. Ctrl+C cancels.",
            Self::OpeningBrowser => "Opening your browser. Waiting for sign-in…",
            Self::Retrying => "The provider requested one more sign-in step. Ctrl+C cancels.",
        }
    }
}

pub fn login_guidance(service: ProviderKind) -> Option<&'static str> {
    match service {
        ProviderKind::Openai => Some(
            "OpenAI-compatible API access uses EJM_CLI_API_KEY or OPENAI_API_KEY.\n\
             Set the key through your shell's secret handling before starting ejm-cli.\n\
             Choose an endpoint and model at launch: ejm-cli --provider openai --base-url URL --model MODEL\n\
             With a key already available to this process, use /model openai MODEL to connect.\n\
             Use --api-key-env NAME for another key variable. Restart after changing the environment.",
        ),
        ProviderKind::Responses => Some(
            "OpenAI Responses API access uses EJM_CLI_API_KEY or OPENAI_API_KEY.\n\
             Set the key through your shell's secret handling before starting ejm-cli.\n\
             This provider uses its fixed official endpoint; remove --base-url and EJM_CLI_BASE_URL.\n\
             With a key already available to this process, use /model responses MODEL to connect.\n\
             Use --api-key-env NAME for another key variable. Restart after changing the environment.",
        ),
        ProviderKind::Anthropic => Some(
            "Anthropic API access uses EJM_CLI_API_KEY or ANTHROPIC_API_KEY and requires a model.\n\
             Set the key through your shell's secret handling before starting ejm-cli.\n\
             Choose an endpoint and model at launch: ejm-cli --provider anthropic --model MODEL [--base-url URL]\n\
             With a key already available to this process, use /model anthropic MODEL to connect.\n\
             Use --api-key-env NAME for another key variable. Restart after changing the environment.\n\
             For subscription access through official Claude Code, choose Claude in /login or run ejm-cli login claude.\n\
             ejm-cli does not collect or reuse Claude Code subscription tokens.",
        ),
        ProviderKind::Chatgpt | ProviderKind::Claude => None,
    }
}

pub async fn login(args: &Args, service: ProviderKind, no_browser: bool) -> Result<bool> {
    if service == ProviderKind::Claude {
        ensure!(
            !no_browser,
            "Claude Code owns its sign-in flow; --no-browser is only supported for ChatGPT"
        );
        return claude_shell_action(args, false).await;
    }
    if let Some(guidance) = login_guidance(service) {
        println!("{guidance}");
        return Ok(true);
    }
    let engine = engine(args)?;
    let cancel = CancellationToken::new();
    let presentation = LoginPresentation::Shell { manual: no_browser };
    let flow = login_flow(&engine, args, &presentation, cancel.clone());
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
        result = &mut flow => {
            let status = result?;
            println!("Signed in: {}", account_label(&status));
            println!("Registration: {}", safe_text(&status.id));
            if !status.plan_enabled {
                println!("This account does not report an enabled ChatGPT plan.");
            }
            Ok(true)
        },
    }
}

pub async fn login_interactive(
    args: &Args,
    cancel: CancellationToken,
    progress: mpsc::Sender<LoginProgress>,
) -> Result<RegistrationStatus> {
    ensure!(!cancel.is_cancelled(), "sign-in cancelled");
    let engine = engine(args)?;
    let presentation = LoginPresentation::Interactive(&progress);
    tokio::select! {
        biased;
        () = cancel.cancelled() => bail!("sign-in cancelled"),
        result = login_flow(&engine, args, &presentation, cancel.clone()) => result,
    }
}

enum LoginPresentation<'a> {
    Shell { manual: bool },
    Interactive(&'a mpsc::Sender<LoginProgress>),
}

impl LoginPresentation<'_> {
    fn progress(&self, progress: LoginProgress) {
        match self {
            Self::Shell { .. } => eprintln!("{}", progress.message()),
            Self::Interactive(sender) => {
                // Progress is advisory. A full or closed view must not stall sign-in.
                let _ = sender.try_send(progress);
            }
        }
    }

    async fn open_browser(&self, url: SecretString, cancel: CancellationToken) -> Result<()> {
        if matches!(self, Self::Shell { manual: true }) {
            println!(
                "Open this sign-in link in your browser:\n{}",
                url.expose_secret()
            );
        } else {
            self.progress(LoginProgress::OpeningBrowser);
            browser::launch(url, cancel).await?;
        }
        Ok(())
    }

    fn operation_error(&self, error: anyhow::Error, operation: &'static str) -> anyhow::Error {
        match self {
            Self::Shell { .. } => error.context(operation),
            // Provider error sources can contain request details. The TUI gets only
            // the operation name, with no authorization URL or response body.
            Self::Interactive(_) => anyhow::anyhow!(operation),
        }
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
    presentation: &LoginPresentation<'_>,
    cancel: CancellationToken,
) -> Result<RegistrationStatus> {
    let cancel = cancel.child_token();
    let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
        .await
        .context("start the private browser callback listener")?;
    let redirect = format!(
        "http://127.0.0.1:{}/auth/callback",
        listener.local_addr()?.port()
    );
    let registration = match &args.account {
        Some(id) => Some(
            select_registration(engine, Some(id), cancel.clone())
                .await?
                .id,
        ),
        None => None,
    };
    presentation.progress(LoginProgress::Starting);
    let attempt = engine
        .start(&redirect, registration.as_deref(), cancel.clone())
        .await
        .map_err(|error| presentation.operation_error(error, "could not start ChatGPT sign-in"))?;
    let mut guard = LoginGuard {
        engine: engine.clone(),
        attempt: attempt.id.clone(),
        cancel: cancel.clone(),
    };
    complete_login(&mut guard, &listener, &redirect, attempt, presentation).await
}

async fn complete_login(
    guard: &mut LoginGuard,
    listener: &TcpListener,
    redirect: &str,
    mut attempt: LoginAttempt,
    presentation: &LoginPresentation<'_>,
) -> Result<RegistrationStatus> {
    let mut retried = false;
    loop {
        safe_authorization_url(attempt.authorization_url.expose_secret())?;
        let state = authorization_state(attempt.authorization_url.expose_secret())?;
        presentation
            .open_browser(attempt.authorization_url, guard.cancel.clone())
            .await?;
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
                    presentation.progress(LoginProgress::Retrying);
                    attempt = guard
                        .engine
                        .retry(&guard.attempt, redirect, guard.cancel.clone())
                        .await
                        .map_err(|error| {
                            presentation.operation_error(error, "could not retry ChatGPT sign-in")
                        })?;
                    guard.attempt.clone_from(&attempt.id);
                    retried = true;
                    continue;
                }
                return Err(presentation.operation_error(error, "complete ChatGPT sign-in"));
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

pub async fn registrations(
    args: &Args,
    cancel: CancellationToken,
) -> Result<Vec<RegistrationStatus>> {
    ensure!(!cancel.is_cancelled(), "account lookup cancelled");
    if !args.state_dir().join("auth").try_exists()? {
        return Ok(Vec::new());
    }
    engine(args)?.registrations(cancel).await
}

pub async fn accounts(args: &Args, json: bool) -> Result<bool> {
    let accounts = account_summary(args, CancellationToken::new()).await?;
    if json {
        let mut entries = accounts
            .chatgpt
            .iter()
            .map(serde_json::to_value)
            .collect::<Result<Vec<_>, _>>()?;
        if accounts.chatgpt_unavailable {
            entries.push(serde_json::json!({
                "provider": "chatgpt", "state": "unavailable", "message": VAULT_RECOVERY,
            }));
        }
        let mut claude = serde_json::to_value(&accounts.claude)?;
        claude["provider"] = "claude".into();
        entries.push(claude);
        println!("{}", serde_json::to_string(&entries)?);
    } else if accounts.chatgpt_unavailable {
        println!("{VAULT_RECOVERY}");
    } else if accounts.chatgpt.is_empty() {
        println!("No saved accounts. Sign in with: ejm-cli login chatgpt");
    } else {
        println!("{:<36}  {:<12}  ACCOUNT", "REGISTRATION", "STATUS");
        for account in &accounts.chatgpt {
            println!(
                "{:<36}  {:<12}  {}",
                safe_text(&account.id),
                if account.signed_in {
                    "signed in"
                } else {
                    "signed out"
                },
                account_label(account)
            );
        }
    }
    if !json {
        println!("{}", accounts.claude.label());
    }
    Ok(true)
}

pub async fn revoke(
    args: &Args,
    registration: &str,
    cancel: CancellationToken,
) -> Result<(RegistrationStatus, LogoutResult)> {
    ensure!(!cancel.is_cancelled(), "sign-out cancelled");
    ensure!(
        args.state_dir().join("auth").try_exists()?,
        "no matching saved account"
    );
    let engine = engine(args)?;
    let registration = select_registration(&engine, Some(registration), cancel.clone()).await?;
    let result = engine.logout(&registration.id, cancel).await?;
    Ok((registration, result))
}

pub async fn logout(args: &Args, registration: &str) -> Result<bool> {
    if registration.eq_ignore_ascii_case("claude") {
        return claude_shell_action(args, true).await;
    }
    let (registration, result) = revoke(args, registration, CancellationToken::new()).await?;
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

pub async fn selected_registration(
    args: &Args,
    cancel: CancellationToken,
) -> Result<RegistrationStatus> {
    ensure!(!cancel.is_cancelled(), "account lookup cancelled");
    select_registration(&engine(args)?, args.account.as_deref(), cancel).await
}

pub async fn credentials_with_cancel(
    args: &Args,
    cancel: CancellationToken,
) -> Result<Arc<dyn CredentialSource>> {
    ensure!(!cancel.is_cancelled(), "account lookup cancelled");
    let engine = engine(args)?;
    let registration = select_registration(&engine, args.account.as_deref(), cancel).await?;
    ensure!(
        registration.signed_in,
        "saved account is signed out; choose ChatGPT in /login"
    );
    Ok(engine.source(&registration.id))
}

async fn select_registration(
    engine: &AuthEngine,
    selected: Option<&str>,
    cancel: CancellationToken,
) -> Result<RegistrationStatus> {
    let accounts = engine.registrations(cancel).await?;
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

pub fn account_label(status: &RegistrationStatus) -> String {
    safe_text(status.email.as_deref().unwrap_or(&status.subject)).replace('\n', " ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpStream,
    };

    #[test]
    fn interactive_progress_is_bounded_and_keeps_provider_errors_private() -> Result<()> {
        let (sender, mut receiver) = mpsc::channel(1);
        let presentation = LoginPresentation::Interactive(&sender);
        presentation.progress(LoginProgress::Starting);
        presentation.progress(LoginProgress::Retrying);
        ensure!(
            receiver.try_recv()? == LoginProgress::Starting,
            "progress did not preserve its first pending update"
        );
        ensure!(receiver.try_recv().is_err(), "progress exceeded its bound");

        let error = presentation.operation_error(
            anyhow::anyhow!("https://auth.openai.com/?code=fixture-secret"),
            "complete ChatGPT sign-in",
        );
        ensure!(
            format!("{error:#}") == "complete ChatGPT sign-in",
            "interactive error exposed provider request details"
        );
        Ok(())
    }

    #[tokio::test]
    async fn cancelled_operations_do_not_open_the_auth_store() -> Result<()> {
        let state = tempfile::tempdir()?;
        let mut args = Args::try_parse_from(["ejm-cli"])?;
        args.state_dir = Some(state.path().join("unused"));
        let cancel = CancellationToken::new();
        cancel.cancel();
        let (sender, _receiver) = mpsc::channel(1);
        ensure!(
            login_interactive(&args, cancel.clone(), sender)
                .await
                .is_err(),
            "cancelled login was accepted"
        );
        ensure!(
            registrations(&args, cancel.clone()).await.is_err(),
            "cancelled account listing was accepted"
        );
        ensure!(
            selected_registration(&args, cancel.clone()).await.is_err(),
            "cancelled account selection was accepted"
        );
        ensure!(
            credentials_with_cancel(&args, cancel.clone())
                .await
                .is_err(),
            "cancelled credential selection was accepted"
        );
        ensure!(
            revoke(&args, "fixture", cancel).await.is_err(),
            "cancelled sign-out was accepted"
        );
        ensure!(
            !args.state_dir().exists(),
            "cancelled operations created credential state"
        );
        Ok(())
    }

    #[tokio::test]
    async fn account_picker_resolves_unique_ids_and_keeps_metadata_safe() -> Result<()> {
        use ejm_harness_auth::{
            CredentialStore, StoredAuth, StoredCredentials, StoredRegistration,
        };

        let store = Arc::new(FileStore::memory());
        let mut transaction = store.lock(CancellationToken::new()).await?;
        transaction.save(&StoredAuth {
            host_id: "fixture-host".into(),
            registrations: ["account-one", "account-two"]
                .into_iter()
                .map(|id| StoredRegistration {
                    id: id.into(),
                    issuer: "https://auth.openai.com".into(),
                    subject: id.into(),
                    client_id: "fixture-client".into(),
                    email: Some(format!("{id}@example.test")),
                    credentials: Some(StoredCredentials {
                        access_token: Some(SecretString::new("fixture-secret")),
                        refresh_token: None,
                        id_token: SecretString::new("fixture-secret"),
                        scopes: vec![],
                        expires_at: None,
                        earliest_refresh_at: None,
                    }),
                    authorization_revision: 0,
                    refresh_pending: false,
                })
                .collect(),
        })?;
        drop(transaction);

        let engine = AuthEngine::new(
            AuthConfig {
                agent_name: "ejm-cli".into(),
            },
            store,
        )?;
        let listed = engine.registrations(CancellationToken::new()).await?;
        ensure!(listed.len() == 2, "saved account metadata was lost");
        ensure!(
            !serde_json::to_string(&listed)?.contains("fixture-secret"),
            "account metadata exposed credentials"
        );
        ensure!(
            select_registration(&engine, None, CancellationToken::new())
                .await
                .is_err(),
            "multiple accounts were selected implicitly"
        );
        let selected =
            select_registration(&engine, Some("account-o"), CancellationToken::new()).await?;
        ensure!(
            selected.id == "account-one",
            "unique prefix was not resolved"
        );
        ensure!(
            account_label(&selected) == "account-one@example.test",
            "wrong account label"
        );
        Ok(())
    }

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
    async fn callback_cancellation_releases_its_listener() -> Result<()> {
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).await?;
        let address = listener.local_addr()?;
        let redirect = format!("http://{address}/auth/callback");
        let cancel = CancellationToken::new();
        let callback_cancel = cancel.clone();
        let task = tokio::spawn(async move {
            wait_callback(&listener, &redirect, "fixture", callback_cancel).await
        });
        cancel.cancel();
        let result = tokio::time::timeout(Duration::from_secs(1), task).await??;
        ensure!(
            result.is_err(),
            "cancelled callback returned authorization data"
        );
        let rebound = TcpListener::bind(address).await?;
        drop(rebound);
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
