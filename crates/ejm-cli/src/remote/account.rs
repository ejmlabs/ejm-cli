//! Relay identity is deliberately separate from provider and registry sign-in.
use super::identity::RelayOrigin;
use anyhow::{Context, Result, bail, ensure};
use ejm_harness_auth::SecretString;
use oauth2::{
    ClientId, DeviceAuthorizationUrl, Scope, StandardDeviceAuthorizationResponse, TokenResponse,
    TokenUrl, basic::BasicClient,
};
use serde::{Deserialize, Serialize};
use std::{path::Path, time::Duration};
use tokio_util::sync::CancellationToken;
use zeroize::Zeroizing;

const CLIENT_ID: &str = "ejm-cli-relay";
const MAX_RESPONSE: usize = 128 * 1024;

#[derive(Clone)]
pub struct RelayAccount {
    pub origin: RelayOrigin,
    pub device_id: String,
    pub account_id: String,
    token: SecretString,
    client: reqwest::Client,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct CachedAccount {
    pub(super) origin: String,
    pub(super) account_id: String,
    pub(super) device_id: String,
}

impl RelayAccount {
    #[cfg(test)]
    pub fn for_test(
        origin: RelayOrigin,
        token: &str,
        account_id: &str,
        device_id: &str,
    ) -> Result<Self> {
        ensure!(
            origin
                .url("/")?
                .host_str()
                .is_some_and(|host| matches!(host, "127.0.0.1" | "[::1]")),
            "test relay must use numeric loopback"
        );
        Ok(Self {
            origin,
            device_id: device_id.into(),
            account_id: account_id.into(),
            token: SecretString::new(token),
            client: reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .timeout(Duration::from_secs(20))
                .build()?,
        })
    }

    pub async fn load(
        origin: RelayOrigin,
        state: &Path,
        cancel: &CancellationToken,
    ) -> Result<Self> {
        let state = state.to_path_buf();
        super::vault::blocking(cancel, move |cancel| {
            Self::load_blocking(origin, &state, cancel)
        })
        .await
    }

    fn load_blocking(
        origin: RelayOrigin,
        state: &Path,
        cancel: &CancellationToken,
    ) -> Result<Self> {
        let metadata =
            metadata(state)?.context("no saved relay account; run ejm-cli remote login")?;
        ensure!(
            metadata.origin == origin.as_str(),
            "saved relay account belongs to another origin; sign in to the configured relay"
        );
        let token = super::vault::load_account(state, &origin, &metadata, cancel)?;
        Ok(Self {
            origin,
            device_id: metadata.device_id,
            account_id: metadata.account_id,
            token,
            client: http_client()?,
        })
    }

    pub async fn login(
        origin: RelayOrigin,
        state: &Path,
        memory_only: bool,
        no_browser: bool,
        cancel: CancellationToken,
    ) -> Result<Self> {
        super::protected::ensure_terminal()?;
        let client = http_client()?;
        let http_origin = origin.clone();
        let http = |request: oauth2::HttpRequest| {
            let client = client.clone();
            let origin = http_origin.clone();
            async move { oauth_http(&client, &origin, request).await }
        };
        let oauth = BasicClient::new(ClientId::new(CLIENT_ID.into()))
            .set_token_uri(TokenUrl::new(
                origin.url("/api/auth/device/token")?.to_string(),
            )?)
            .set_device_authorization_url(DeviceAuthorizationUrl::new(
                origin.url("/api/auth/device/code")?.to_string(),
            )?);
        let details: StandardDeviceAuthorizationResponse = tokio::select! {
            biased;
            () = cancel.cancelled() => bail!("relay login cancelled"),
            details = oauth.exchange_device_code().add_scope(Scope::new("relay".into())).request_async(&http) =>
                details.map_err(|_| anyhow::anyhow!("relay device authorization failed"))?,
        };
        let expires =
            tokio::time::Instant::now() + details.expires_in().min(Duration::from_secs(600));
        show_device_authorization(&origin, &details, no_browser, cancel.clone()).await?;
        let token = device_token(&origin, &client, &details, expires, cancel.clone()).await?;
        let mut account = Self {
            origin,
            device_id: String::new(),
            account_id: String::new(),
            token,
            client,
        };
        account.register_device(cancel.clone()).await?;
        if !memory_only {
            let state = state.to_path_buf();
            let metadata = CachedAccount {
                origin: account.origin.as_str().into(),
                account_id: account.account_id.clone(),
                device_id: account.device_id.clone(),
            };
            let token = account.token.clone();
            super::vault::blocking(&cancel, move |cancel| -> Result<()> {
                std::fs::create_dir_all(&state)?;
                super::vault::save_account(&state, &metadata, &token, cancel)?;
                std::fs::write(
                    state.join("relay-account.json"),
                    serde_json::to_vec(&metadata)?,
                )?;
                Ok(())
            })
            .await?;
        }
        Ok(account)
    }

    async fn register_device(&mut self, cancel: CancellationToken) -> Result<()> {
        let status = self.json(reqwest::Method::GET, "/v1/account", None).await?;
        self.account_id = status
            .get("id")
            .and_then(serde_json::Value::as_str)
            .context("relay account missing identity")?
            .into();
        let required = status
            .get("requiredTermsVersion")
            .and_then(serde_json::Value::as_str)
            .context("relay terms unavailable")?;
        if status
            .get("termsVersion")
            .and_then(serde_json::Value::as_str)
            != Some(required)
        {
            let notice = format!(
                "Review {}\nAccept relay service terms version {}? Data sales stay off.",
                self.origin.url("/terms")?,
                required
            );
            ensure!(
                super::protected::confirm(&notice, cancel.clone()).await?,
                "relay terms were not accepted"
            );
            self.json(
                reqwest::Method::POST,
                "/v1/account/terms",
                Some(serde_json::json!({"version":required})),
            )
            .await?;
        }
        let device = self
            .json(
                reqwest::Method::POST,
                "/v1/devices",
                Some(serde_json::json!({"label":"Native CLI"})),
            )
            .await?;
        self.device_id = device
            .get("id")
            .and_then(serde_json::Value::as_str)
            .context("relay device missing identity")?
            .into();
        Ok(())
    }

    pub async fn json(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<serde_json::Value>,
    ) -> Result<serde_json::Value> {
        let mut request = self
            .client
            .request(method, self.origin.url(path)?)
            .bearer_auth(self.token.expose_secret())
            .header(
                "origin",
                self.origin.url("/")?.origin().ascii_serialization(),
            );
        if let Some(body) = body {
            request = request.json(&body);
        }
        let response = request
            .send()
            .await
            .map_err(|_| anyhow::anyhow!("relay request failed"))?;
        let status = response.status();
        ensure!(
            !status.is_redirection(),
            "relay credential redirect rejected"
        );
        let bytes = bounded_response(response).await?;
        ensure!(
            status.is_success(),
            "relay refused request ({status}); check account, capacity and plan"
        );
        serde_json::from_slice(&bytes).context("invalid relay response")
    }

    pub async fn ticket(&self, room: &str, role: &str) -> Result<SecretString> {
        ensure!(
            valid_id(room) && matches!(role, "host" | "controller"),
            "invalid room request"
        );
        let response = self
            .json(
                reqwest::Method::POST,
                &format!("/v1/rooms/{room}/tickets"),
                Some(serde_json::json!({"role":role,"deviceId":self.device_id})),
            )
            .await?;
        let ticket = response
            .get("ticket")
            .and_then(serde_json::Value::as_str)
            .context("relay ticket unavailable")?;
        ensure!(
            ticket.len() <= 4096 && !ticket.chars().any(char::is_control),
            "invalid relay ticket"
        );
        Ok(SecretString::new(ticket))
    }

    pub async fn logout(&self, state: &Path, cancel: &CancellationToken) -> Result<bool> {
        let revoked = self
            .json(
                reqwest::Method::POST,
                "/v1/logout",
                Some(serde_json::json!({})),
            )
            .await
            .is_ok();
        let state = state.to_path_buf();
        let origin = self.origin.clone();
        super::vault::blocking(cancel, move |cancel| -> Result<()> {
            super::vault::remove_account(&state, &origin, cancel)?;
            let path = state.join("relay-account.json");
            if path.exists() {
                std::fs::remove_file(path)?;
            }
            Ok(())
        })
        .await?;
        Ok(revoked)
    }
}

async fn show_device_authorization(
    origin: &RelayOrigin,
    details: &StandardDeviceAuthorizationResponse,
    no_browser: bool,
    cancel: CancellationToken,
) -> Result<()> {
    let verification = details.verification_uri().url();
    ensure!(
        verification.origin() == origin.url("/")?.origin() && verification.scheme() == "https",
        "relay returned an untrusted authorization origin"
    );
    ensure!(
        verification.username().is_empty() && verification.password().is_none(),
        "invalid authorization URL"
    );
    // The user code is public approval material. The secret device code stays in the OAuth library.
    let code = details.user_code().secret();
    ensure!(
        code.len() <= 64 && code.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'),
        "invalid approval code"
    );
    let panel = format!(
        "Open {verification}\nEnter code: {code}\nApprove only this device, then return here."
    );
    super::protected::login_notice(&panel, no_browser, verification.as_str(), cancel).await
}

pub fn cached(state: &Path) -> Result<Option<String>> {
    let Some(account) = metadata(state)? else {
        return Ok(None);
    };
    Ok(Some(format!(
        "Relay: {}\nAccount: {}\nDevice: {}",
        crate::render::safe_text(&account.origin),
        crate::render::safe_text(&account.account_id),
        crate::render::safe_text(&account.device_id)
    )))
}

pub(super) fn metadata(state: &Path) -> Result<Option<CachedAccount>> {
    let path = state.join("relay-account.json");
    if !path.exists() {
        return Ok(None);
    }
    ensure!(
        std::fs::metadata(&path)?.len() <= 4096,
        "invalid relay account metadata"
    );
    let data = std::fs::read(path)?;
    ensure!(data.len() <= 4096, "invalid relay account metadata");
    let account: CachedAccount = serde_json::from_slice(&data)?;
    Ok(Some(account))
}
async fn device_token(
    origin: &RelayOrigin,
    client: &reqwest::Client,
    details: &StandardDeviceAuthorizationResponse,
    expires: tokio::time::Instant,
    cancel: CancellationToken,
) -> Result<SecretString> {
    let remaining = expires.saturating_duration_since(tokio::time::Instant::now());
    ensure!(!remaining.is_zero(), "relay device authorization expired");
    let oauth = BasicClient::new(ClientId::new(CLIENT_ID.into())).set_token_uri(TokenUrl::new(
        origin.url("/api/auth/device/token")?.to_string(),
    )?);
    let http = |request| oauth_http(client, origin, request);
    let token = tokio::select! {
        biased;
        () = cancel.cancelled() => bail!("relay login cancelled"),
        () = tokio::time::sleep_until(expires) => bail!("relay device authorization expired"),
        token = oauth.exchange_device_access_token(details)
            .request_async(&http, tokio::time::sleep, Some(remaining)) =>
            token.map_err(|_| anyhow::anyhow!("relay approval denied, expired or unavailable"))?,
    };
    Ok(SecretString::new(token.access_token().secret()))
}

fn http_client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .https_only(true)
        .timeout(Duration::from_secs(20))
        .connect_timeout(Duration::from_secs(10))
        .build()
        .context("initialize certificate-verified relay HTTP")
}
async fn bounded_response(mut response: reqwest::Response) -> Result<Zeroizing<Vec<u8>>> {
    ensure!(
        response
            .content_length()
            .is_none_or(|size| size <= MAX_RESPONSE as u64),
        "relay response exceeds limit"
    );
    let mut bytes = Zeroizing::new(Vec::new());
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| anyhow::anyhow!("relay response failed"))?
    {
        ensure!(
            bytes.len().saturating_add(chunk.len()) <= MAX_RESPONSE,
            "relay response exceeds limit"
        );
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}
async fn oauth_http(
    client: &reqwest::Client,
    origin: &RelayOrigin,
    request: oauth2::HttpRequest,
) -> Result<oauth2::HttpResponse, std::io::Error> {
    async {
        let (parts, body) = request.into_parts();
        let url = url::Url::parse(&parts.uri.to_string())?;
        ensure!(
            url.origin() == origin.url("/")?.origin(),
            "authorization origin changed"
        );
        let response = client
            .request(parts.method, url)
            .headers(parts.headers)
            .body(Zeroizing::new(body).to_vec())
            .send()
            .await?;
        ensure!(
            !response.status().is_redirection(),
            "credential redirect rejected"
        );
        let mut builder = oauth2::http::Response::builder().status(response.status());
        for (name, value) in response.headers() {
            builder = builder.header(name, value);
        }
        let bytes = bounded_response(response).await?;
        Ok(builder.body(bytes.to_vec())?)
    }
    .await
    .map_err(|_: anyhow::Error| std::io::Error::other("relay device authorization request failed"))
}
pub fn valid_id(id: &str) -> bool {
    (16..=128).contains(&id.len())
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

#[cfg(test)]
#[path = "account_tests.rs"]
mod tests;
