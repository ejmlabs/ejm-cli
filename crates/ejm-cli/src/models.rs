use std::sync::Arc;

use anyhow::{Context, Result, bail, ensure};
use clap::ValueEnum;
use ejm_harness::{
    AnthropicConfig, AnthropicProvider, AvailableModel, ClaudeCodeConfig, ClaudeCodeProvider,
    OpenAiConfig, OpenAiProvider, Provider, ResponsesConfig, ResponsesProvider,
};
use ejm_harness_auth::{CredentialSource, SecretString, StaticCredential};
use ejm_harness_exec::Executor;
use tokio_util::sync::CancellationToken;

use crate::{
    args::{Args, ProviderKind},
    auth,
    extensions::Extensions,
};

const MODEL_USAGE: &str = "usage: /model [PROVIDER] MODEL";

/// Compose public harness providers from host configuration without running a model.
pub async fn prepare(
    args: &mut Args,
    extensions: &Extensions,
    executor: &Executor,
    cancel: CancellationToken,
) -> Result<Arc<dyn Provider>> {
    ensure!(!cancel.is_cancelled(), "model configuration cancelled");
    if let Some(name) = &args.package_provider {
        return extensions.provider(name);
    }
    validate(args)?;
    if args.provider == ProviderKind::Claude {
        auth::claude_client(args)
            .status(cancel.child_token())
            .await?
            .require_subscription()?;
        let model = args.model.clone().unwrap_or_else(|| "sonnet".into());
        let provider = ClaudeCodeProvider::new(
            ClaudeCodeConfig {
                program: args.claude_program(),
                cwd: args.cwd()?,
                model: model.clone(),
                timeout_ms: args.run_timeout_ms,
                ..ClaudeCodeConfig::default()
            },
            executor.clone(),
        )?;
        args.model = Some(model);
        return Ok(Arc::new(provider));
    }
    if native(args.provider) {
        return responses_provider(args, cancel).await;
    }
    let key = required_api_key(args)?;
    let model = args
        .model
        .clone()
        .or_else(|| (args.provider == ProviderKind::Openai).then(|| "gpt-4.1-mini".into()))
        .context("Anthropic API access requires --model MODEL or EJM_CLI_MODEL")?;
    args.model = Some(model.clone());
    if args.provider == ProviderKind::Anthropic {
        return Ok(Arc::new(AnthropicProvider::new(AnthropicConfig {
            base_url: args.api_base_url().into(),
            api_key: key,
            model,
        })?));
    }
    Ok(Arc::new(OpenAiProvider::new(OpenAiConfig {
        base_url: args.api_base_url().into(),
        api_key: key,
        model,
    })?))
}

/// Reject incompatible sources before a plain run creates durable state.
pub fn validate(args: &Args) -> Result<()> {
    if args.package_provider.is_some() {
        return Ok(());
    }
    if args.provider == ProviderKind::Claude {
        return validate_claude(args);
    }
    if native(args.provider) {
        validate_native(args)?;
        if args.provider == ProviderKind::Chatgpt {
            return Ok(());
        }
    }
    ensure!(
        args.account.is_none(),
        "--account requires --provider chatgpt"
    );
    if !native(args.provider) {
        ensure!(
            args.reasoning_effort.is_none(),
            "--reasoning-effort requires --provider responses or --provider chatgpt"
        );
    }
    required_api_key(args)?;
    ensure!(
        args.provider != ProviderKind::Anthropic || args.model.is_some(),
        "Anthropic API access requires --model MODEL or EJM_CLI_MODEL"
    );
    Ok(())
}

pub fn validate_claude(args: &Args) -> Result<()> {
    ensure!(
        args.base_url.is_none(),
        "Claude subscription access uses the official client; remove --base-url and EJM_CLI_BASE_URL"
    );
    ensure!(
        args.account.is_none(),
        "Claude uses the official client's current account; remove --account and use /account claude"
    );
    ensure!(
        args.api_key_env == "EJM_CLI_API_KEY",
        "Claude subscription access does not accept --api-key-env; use --provider anthropic for API keys"
    );
    ensure!(
        args.reasoning_effort.is_none(),
        "--reasoning-effort requires --provider responses or --provider chatgpt"
    );
    Ok(())
}

/// Discover models only for the selected native provider.
pub async fn catalog(args: &Args, cancel: CancellationToken) -> Result<Vec<AvailableModel>> {
    validate_native(args)?;
    ensure!(
        args.package_provider.is_none(),
        "package providers have no model catalog; use /model PROVIDER MODEL to select a built-in provider"
    );
    ResponsesProvider::available_models(
        responses_credentials(args, cancel.child_token()).await?,
        cancel,
    )
    .await
}

/// Identify the effective connection for harness continuation compatibility.
/// Credentials are neither read nor serialized here.
pub fn binding(args: &Args) -> Result<String> {
    let connection = if let Some(package) = &args.package_provider {
        serde_json::json!({ "kind": "package", "package": package, "model": args.model })
    } else if args.provider == ProviderKind::Claude {
        validate_claude(args)?;
        serde_json::json!({
            "kind": "builtin",
            "provider": "claude",
            "model": args.model.as_deref().unwrap_or("sonnet"),
            "program": args.claude_program(),
            "source": "official-client-subscription",
        })
    } else {
        let endpoint = if native(args.provider) {
            validate_native(args)?;
            if args.provider == ProviderKind::Chatgpt {
                "official ChatGPT Responses".into()
            } else {
                "official OpenAI Responses".into()
            }
        } else {
            let endpoint =
                url::Url::parse(args.api_base_url()).context("invalid provider base URL")?;
            ensure!(
                endpoint.username().is_empty()
                    && endpoint.password().is_none()
                    && endpoint.query().is_none()
                    && endpoint.fragment().is_none(),
                "provider URL must not contain credentials, query, or fragment"
            );
            endpoint.to_string().trim_end_matches('/').to_owned()
        };
        serde_json::json!({
            "kind": "builtin",
            "provider": provider_name(args.provider),
            "model": args.model,
            "account": args.account,
            "endpoint": endpoint,
            "reasoning_effort": args.reasoning_effort,
        })
    };
    serde_json::to_string(&connection).context("serialize provider binding")
}

/// Parse the argument tail of `/model [PROVIDER] MODEL` into a candidate snapshot.
pub fn select(args: &Args, input: &str) -> Result<Args> {
    let tokens = crate::commands::model_arguments(input)?;
    let (provider, model) = match tokens.as_slice() {
        [model] => (None, model),
        [provider, model] => (
            Some(ProviderKind::from_str(provider, false).map_err(|_| {
                anyhow::anyhow!(
                    "{MODEL_USAGE}; providers: openai, responses, chatgpt, claude, anthropic"
                )
            })?),
            model,
        ),
        _ => bail!(MODEL_USAGE),
    };
    ensure!(
        provider.is_some() || args.package_provider.is_none(),
        "package providers require an explicit provider; use /model PROVIDER MODEL"
    );
    let mut selected = args.clone();
    if let Some(provider) = provider {
        ensure!(
            !native(provider) || args.base_url.is_none(),
            "native Responses uses its fixed official endpoint; remove --base-url and EJM_CLI_BASE_URL before selecting this provider"
        );
        if provider == ProviderKind::Claude {
            ensure!(
                args.base_url.is_none(),
                "Claude subscription access uses the official client; remove --base-url and EJM_CLI_BASE_URL"
            );
        }
        if provider != args.provider {
            selected.base_url = None;
            selected.account = None;
            if !native(provider) {
                selected.reasoning_effort = None;
            }
        }
        selected.provider = provider;
        selected.package_provider = None;
    }
    if selected.provider == ProviderKind::Claude {
        validate_claude(&selected)?;
    }
    selected.model = Some(model.clone());
    Ok(selected)
}

fn native(provider: ProviderKind) -> bool {
    matches!(provider, ProviderKind::Responses | ProviderKind::Chatgpt)
}

fn provider_name(provider: ProviderKind) -> &'static str {
    match provider {
        ProviderKind::Openai => "openai",
        ProviderKind::Responses => "responses",
        ProviderKind::Chatgpt => "chatgpt",
        ProviderKind::Claude => "claude",
        ProviderKind::Anthropic => "anthropic",
    }
}

fn required_api_key(args: &Args) -> Result<String> {
    args.api_key().with_context(|| {
        format!(
            "set {} or {} for your provider, or use /login in the TUI to choose a sign-in provider",
            args.api_key_env,
            args.fallback_key_env()
        )
    })
}

fn validate_native(args: &Args) -> Result<()> {
    ensure!(
        native(args.provider),
        "model discovery requires --provider responses or --provider chatgpt; use /model [PROVIDER] MODEL for an explicit model"
    );
    ensure!(
        args.base_url.is_none(),
        "native Responses uses its fixed official endpoint; remove --base-url and EJM_CLI_BASE_URL"
    );
    Ok(())
}

async fn responses_credentials(
    args: &Args,
    cancel: CancellationToken,
) -> Result<Arc<dyn CredentialSource>> {
    validate_native(args)?;
    ensure!(!cancel.is_cancelled(), "model configuration cancelled");
    if args.provider == ProviderKind::Chatgpt {
        return auth::credentials_with_cancel(args, cancel).await;
    }
    ensure!(
        args.account.is_none(),
        "--account requires --provider chatgpt"
    );
    Ok(Arc::new(StaticCredential::new(SecretString::new(
        required_api_key(args)?,
    ))))
}

async fn responses_provider(
    args: &mut Args,
    cancel: CancellationToken,
) -> Result<Arc<dyn Provider>> {
    validate_native(args)?;
    if args.provider == ProviderKind::Chatgpt {
        let registration = auth::selected_registration(args, cancel.child_token()).await?;
        args.account = Some(registration.id);
    }
    let credentials = responses_credentials(args, cancel.child_token()).await?;
    let model = match &args.model {
        Some(model) => model.clone(),
        None if args.provider == ProviderKind::Responses => "gpt-4.1-mini".into(),
        None => {
            ResponsesProvider::available_models(credentials.clone(), cancel)
                .await?
                .into_iter()
                .next()
                .context("this ChatGPT account returned no available models")?
                .slug
        }
    };
    args.model = Some(model.clone());
    Ok(Arc::new(ResponsesProvider::new(
        ResponsesConfig {
            model,
            reasoning_effort: args.reasoning_effort.clone(),
        },
        credentials,
    )?))
}

#[cfg(test)]
#[expect(
    clippy::unwrap_used,
    reason = "Fixture arguments must parse or fail this isolated test."
)]
mod tests {
    use super::*;
    use clap::Parser;

    fn executor(state: &tempfile::TempDir) -> Executor {
        Executor::new(
            Arc::new(
                ejm_harness_exec::Governor::new(ejm_harness_exec::ResourceLimits::default())
                    .unwrap(),
            ),
            state.path().join("artifacts"),
        )
        .unwrap()
    }

    fn args() -> Args {
        let mut args = Args::try_parse_from(["ejm-cli"]).unwrap();
        args.base_url = None;
        args.model = None;
        args
    }

    #[test]
    fn model_only_selection_retains_current_provider_configuration() {
        let mut current = args();
        current.provider = ProviderKind::Chatgpt;
        current.account = Some("registration".into());
        current.reasoning_effort = Some("high".into());
        let selected = select(&current, "gpt-5.4").unwrap();
        assert_eq!(selected.provider, ProviderKind::Chatgpt);
        assert_eq!(selected.account, current.account);
        assert_eq!(selected.reasoning_effort, current.reasoning_effort);
        assert_eq!(selected.model.as_deref(), Some("gpt-5.4"));
        assert!(current.model.is_none());
    }

    #[test]
    fn claude_selection_clears_chatgpt_identity_and_binds_the_official_client() {
        let mut current = args();
        current.provider = ProviderKind::Chatgpt;
        current.account = Some("native-registration".into());
        current.reasoning_effort = Some("high".into());
        current.claude_path = Some("/fixture/claude".into());
        let selected = select(&current, "claude sonnet").unwrap();
        assert_eq!(selected.provider, ProviderKind::Claude);
        assert_eq!(selected.model.as_deref(), Some("sonnet"));
        assert!(selected.account.is_none());
        assert!(selected.reasoning_effort.is_none());
        let original = binding(&selected).unwrap();
        assert!(original.contains("official-client-subscription"));
        assert!(!original.contains("native-registration"));
        let mut other_client = selected;
        other_client.claude_path = Some("/fixture/another-claude".into());
        assert_ne!(binding(&other_client).unwrap(), original);
        assert_eq!(current.account.as_deref(), Some("native-registration"));
    }

    #[test]
    fn claude_configuration_rejects_endpoints_accounts_and_credential_options() {
        let mut current = args();
        current.provider = ProviderKind::Claude;
        for option in ["base_url", "account", "api_key_env", "reasoning_effort"] {
            let mut invalid = current.clone();
            match option {
                "base_url" => invalid.base_url = Some("https://gateway.test/v1".into()),
                "account" => invalid.account = Some("chatgpt-registration".into()),
                "api_key_env" => invalid.api_key_env = "ANTHROPIC_API_KEY".into(),
                _ => invalid.reasoning_effort = Some("high".into()),
            }
            assert!(validate(&invalid).is_err(), "{option}");
        }
        assert!(validate(&current).is_ok());
        assert_eq!(current.provider_label(), "sonnet");
    }

    #[test]
    fn explicit_provider_selection_leaves_package_provider() {
        let mut current = args();
        current.package_provider = Some("@example/offline:echo".into());
        let selected = select(&current, "responses gpt-4.1").unwrap();
        assert_eq!(selected.provider, ProviderKind::Responses);
        assert_eq!(selected.model.as_deref(), Some("gpt-4.1"));
        assert!(selected.package_provider.is_none());
        assert!(current.package_provider.is_some());
    }

    #[test]
    fn model_only_selection_rejects_package_provider() {
        let mut current = args();
        current.package_provider = Some("@example/offline:echo".into());
        current.model = Some("current-model".into());
        let original = binding(&current).unwrap();
        let error = select(&current, "next-model").err().unwrap();
        assert!(error.to_string().contains("/model PROVIDER MODEL"));
        assert_eq!(binding(&current).unwrap(), original);
        assert_eq!(current.model.as_deref(), Some("current-model"));
    }

    #[test]
    fn provider_change_clears_incompatible_account_endpoint_and_effort() {
        let mut current = args();
        current.provider = ProviderKind::Chatgpt;
        current.account = Some("registration".into());
        current.reasoning_effort = Some("high".into());
        current.base_url = Some("https://compat.test/v1".into());
        let selected = select(&current, "anthropic claude-model").unwrap();
        assert_eq!(selected.provider, ProviderKind::Anthropic);
        assert!(selected.account.is_none());
        assert!(selected.base_url.is_none());
        assert!(selected.reasoning_effort.is_none());
    }

    #[test]
    fn native_selection_rejects_custom_endpoint_without_changing_current_args() {
        let mut current = args();
        current.base_url = Some("https://compat.test/v1".into());
        for provider in ["responses", "chatgpt"] {
            let error = select(&current, &format!("{provider} model"))
                .err()
                .unwrap();
            assert!(error.to_string().contains("fixed official endpoint"));
        }
        assert_eq!(current.provider, ProviderKind::Openai);
        assert_eq!(current.base_url.as_deref(), Some("https://compat.test/v1"));
    }

    #[test]
    fn invalid_model_arguments_report_usage() {
        for input in [
            "",
            "unknown model",
            "openai one two",
            "--api-key",
            "openai --key",
        ] {
            let error = select(&args(), input).err().unwrap();
            assert!(error.to_string().contains(MODEL_USAGE), "{input}");
        }
    }

    #[test]
    fn bindings_are_stable_and_cover_connection_identity_without_credentials() {
        let mut current = args();
        current.provider = ProviderKind::Chatgpt;
        current.model = Some("model-one".into());
        current.account = Some("account-one".into());
        let original = binding(&current).unwrap();
        assert_eq!(original, binding(&current.clone()).unwrap());
        current.api_key_env = "DO_NOT_SERIALIZE_THIS_CREDENTIAL_SOURCE".into();
        assert_eq!(original, binding(&current).unwrap());
        for selected in [
            select(&current, "model-two").unwrap(),
            select(&current, "responses model-one").unwrap(),
        ] {
            assert_ne!(original, binding(&selected).unwrap());
        }
        current.account = Some("account-two".into());
        assert_ne!(original, binding(&current).unwrap());
        let mut compatible = select(&current, "openai model-one").unwrap();
        let official = binding(&compatible).unwrap();
        compatible.base_url = Some("https://compat.test/v1".into());
        assert_ne!(official, binding(&compatible).unwrap());
        current.package_provider = Some("@example/offline:echo".into());
        assert!(binding(&current).unwrap().contains("@example/offline:echo"));
    }

    #[test]
    fn binding_rejects_embedded_endpoint_credentials_without_echoing_them() {
        let mut current = args();
        for endpoint in [
            "https://user:fixture-secret@compat.test/v1",
            "https://compat.test/v1?api_key=fixture-secret",
            "https://compat.test/v1#fixture-secret",
        ] {
            current.base_url = Some(endpoint.into());
            let error = binding(&current).err().unwrap();
            assert!(!format!("{error:#}").contains("fixture-secret"));
        }
    }

    #[tokio::test]
    async fn unconfigured_preparation_fails_and_package_catalog_never_connects() {
        let state = tempfile::tempdir().unwrap();
        let executor = executor(&state);
        let mut current = args();
        current.api_key_env = "INVALID=ENVIRONMENT_NAME".into();
        let extensions = Extensions::empty().unwrap();
        assert!(
            prepare(
                &mut current,
                &extensions,
                &executor,
                CancellationToken::new()
            )
            .await
            .is_err()
        );
        let error = catalog(&current, CancellationToken::new())
            .await
            .err()
            .unwrap();
        assert!(error.to_string().contains("explicit model"));
        current.provider = ProviderKind::Responses;
        current.package_provider = Some("@fixture/model:provider".into());
        let error = catalog(&current, CancellationToken::new())
            .await
            .err()
            .unwrap();
        assert!(error.to_string().contains("package providers"));
        extensions.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn cancelled_preparation_does_not_select_a_provider() {
        let state = tempfile::tempdir().unwrap();
        let executor = executor(&state);
        let mut current = args();
        let extensions = Extensions::empty().unwrap();
        let cancel = CancellationToken::new();
        cancel.cancel();
        let error = prepare(&mut current, &extensions, &executor, cancel)
            .await
            .err()
            .unwrap();
        assert!(error.to_string().contains("cancelled"));
        extensions.shutdown().await.unwrap();
    }
}
