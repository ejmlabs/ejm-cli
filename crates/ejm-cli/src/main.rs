mod app;
mod args;
mod auth;
mod browser;
mod commands;
// Copied from the ejmlabs design kit (design-kit/ui/rust). Never edit these two files; copy new versions in.
#[rustfmt::skip]
#[allow(
    dead_code,
    clippy::all,
    clippy::pedantic,
    reason = "Vendored design kit file, kept byte for byte so updates are a plain copy."
)]
mod ejm_theme;
#[rustfmt::skip]
#[allow(
    dead_code,
    clippy::all,
    clippy::pedantic,
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "Vendored design kit file, kept byte for byte so updates are a plain copy."
)]
mod ejm_tui;
mod extensions;
mod packages;
mod remote;
mod render;
mod terminal;
mod terminal_input;
mod ui;

use std::{io::IsTerminal, sync::Arc};

use anyhow::{Context, Result, bail, ensure};
use clap::Parser;
use ejm_harness::{
    AnthropicConfig, AnthropicProvider, EventKind, Harness, HarnessConfig, ModelResponse,
    OpenAiConfig, OpenAiProvider, Provider, ReplayProvider, ResponsesConfig, ResponsesProvider,
    RunStatus, SessionOptions,
};
use ejm_harness_auth::{CredentialSource, SecretString, StaticCredential};
use ejm_harness_coding::{CodingConfig, tools};
use tokio_util::sync::CancellationToken;

use args::{Args, Command, ProviderKind, RegistryChoice, display_registry_url};

fn main() -> std::process::ExitCode {
    let args = Args::parse();
    let result = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .max_blocking_threads(4)
        .enable_all()
        .build()
        .context("start the CLI runtime")
        .and_then(|runtime| runtime.block_on(run(args)));
    match result {
        Ok(true) => std::process::ExitCode::SUCCESS,
        Ok(false) => std::process::ExitCode::from(1),
        Err(error) => {
            eprintln!("ejm-cli: {}", render::safe_text(&format!("{error:#}")));
            std::process::ExitCode::from(1)
        }
    }
}

enum AgentMode {
    Interactive,
    Turn { prompt: String, json: bool },
    Sessions { json: bool },
}

async fn run(mut args: Args) -> Result<bool> {
    let state_dir = args.state_dir();
    let mode = match &args.command {
        Some(Command::Remote {
            command: remote::RemoteCommand::Host { .. },
        })
        | None => AgentMode::Interactive,
        Some(Command::Remote { command }) => return remote::run(&args, command).await,
        Some(Command::Package { command }) => return packages::run(&args, command).await,
        Some(Command::Doctor) => return doctor(&args),
        Some(Command::Login {
            service,
            no_browser,
        }) => {
            return auth::login(&args, *service, *no_browser).await;
        }
        Some(Command::Accounts { json }) => return auth::accounts(&args, *json).await,
        Some(Command::Logout { registration }) => return auth::logout(&args, registration).await,
        Some(Command::Models { json }) => return models(&args, *json).await,
        Some(Command::Run { prompt, json }) => AgentMode::Turn {
            prompt: prompt.join(" "),
            json: *json,
        },
        Some(Command::Sessions { json }) => AgentMode::Sessions { json: *json },
    };
    let extensions = if matches!(mode, AgentMode::Sessions { .. }) {
        extensions::Extensions::empty()?
    } else {
        extensions::Extensions::load(&args).await?
    };
    let result = run_agent(&mut args, state_dir, mode, &extensions).await;
    let shutdown = extensions.shutdown().await;
    let result = result?;
    shutdown?;
    Ok(result)
}

async fn run_agent(
    args: &mut Args,
    state_dir: std::path::PathBuf,
    mode: AgentMode,
    extensions: &extensions::Extensions,
) -> Result<bool> {
    let provider = if let Some(name) = &args.package_provider {
        extensions.provider(name)?
    } else {
        provider(args).await?
    };
    let config = HarnessConfig {
        state_dir: state_dir.clone(),
        max_sessions: args.max_sessions,
        max_model_requests: args.model_concurrency,
        resources: args.resources(),
        max_steps: args.max_steps,
        run_timeout_ms: args.run_timeout_ms,
        ..HarnessConfig::default()
    };
    let mut coding = tools(CodingConfig {
        allow_write: args.allow_write,
        allow_shell: args.allow_shell,
        artifact_dir: state_dir.join("artifacts"),
    })?;
    coding.extend(extensions.tools()?);
    let harness = Harness::with_hooks(config, provider, coding, extensions.hooks()?)?;
    let result = match mode {
        AgentMode::Sessions { json } => {
            let sessions = harness.list_sessions().await?;
            if json {
                println!("{}", serde_json::to_string(&sessions)?);
            } else if sessions.is_empty() {
                println!("No sessions yet. Start with: ejm-cli --replay");
            } else {
                println!("{:<36}  {:<14}  TITLE", "SESSION", "STATUS");
                for session in sessions {
                    println!(
                        "{:<36}  {:<14}  {}",
                        session.id,
                        serde_json::to_value(&session.status)?
                            .as_str()
                            .unwrap_or("unknown"),
                        render::safe_text(&session.title).replace('\n', " ")
                    );
                }
            }
            Ok(true)
        }
        AgentMode::Turn { prompt, json } => run_once(&harness, args, prompt, json).await,
        AgentMode::Interactive => {
            if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
                bail!(
                    "the TUI needs a terminal; use `ejm-cli --replay run \"Hello\"` for plain output"
                );
            }
            ui::run(harness.clone(), args, extensions)
                .await
                .map(|()| true)
        }
    };
    let shutdown = harness.shutdown().await;
    let success = result?;
    shutdown?;
    Ok(success)
}

async fn provider(args: &mut Args) -> Result<Arc<dyn Provider>> {
    if let Some(path) = &args.replay_file {
        let bytes = std::fs::read(path).context("read replay responses")?;
        if bytes.len() > 8 * 1024 * 1024 {
            bail!("replay file exceeds 8 MiB");
        }
        let responses: Vec<ModelResponse> = serde_json::from_slice(&bytes)
            .context("replay file must contain an array of model responses")?;
        return Ok(Arc::new(ReplayProvider::new(responses)));
    }
    if args.replay {
        return Ok(Arc::new(ReplayProvider::echo()));
    }
    // Session inspection does not need credentials or a network connection.
    if matches!(args.command, Some(Command::Sessions { .. })) {
        return Ok(Arc::new(ReplayProvider::echo()));
    }
    if matches!(
        args.provider,
        ProviderKind::Chatgpt | ProviderKind::Responses
    ) {
        return responses_provider(args).await;
    }
    ensure!(
        args.account.is_none(),
        "--account requires --provider chatgpt"
    );
    ensure!(
        args.reasoning_effort.is_none(),
        "--reasoning-effort requires --provider responses or --provider chatgpt"
    );
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

fn required_api_key(args: &Args) -> Result<String> {
    args.api_key().with_context(|| {
        format!(
            "set {} or {} for your provider, or use --replay for an offline demo",
            args.api_key_env,
            args.fallback_key_env()
        )
    })
}

async fn responses_credentials(args: &Args) -> Result<Arc<dyn CredentialSource>> {
    ensure!(
        matches!(
            args.provider,
            ProviderKind::Responses | ProviderKind::Chatgpt
        ),
        "model discovery requires --provider responses or --provider chatgpt"
    );
    ensure!(
        args.base_url.is_none(),
        "native Responses uses its fixed official endpoint; remove --base-url and EJM_CLI_BASE_URL"
    );
    if args.provider == ProviderKind::Chatgpt {
        return auth::credentials(args).await;
    }
    ensure!(
        args.account.is_none(),
        "--account requires --provider chatgpt"
    );
    Ok(Arc::new(StaticCredential::new(SecretString::new(
        required_api_key(args)?,
    ))))
}

async fn models(args: &Args, json: bool) -> Result<bool> {
    let catalog = ResponsesProvider::available_models(
        responses_credentials(args).await?,
        CancellationToken::new(),
    )
    .await?;
    if json {
        println!("{}", serde_json::to_string(&catalog)?);
    } else {
        for model in catalog {
            println!(
                "{:<32}  {}",
                render::safe_text(&model.slug),
                render::safe_text(&model.display_name)
            );
        }
    }
    Ok(true)
}

async fn responses_provider(args: &mut Args) -> Result<Arc<dyn Provider>> {
    let credentials = responses_credentials(args).await?;
    let model = match &args.model {
        Some(model) => model.clone(),
        None if args.provider == ProviderKind::Responses => "gpt-4.1-mini".into(),
        None => {
            ResponsesProvider::available_models(credentials.clone(), CancellationToken::new())
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

async fn run_once(harness: &Harness, args: &Args, prompt: String, json: bool) -> Result<bool> {
    use std::io::Write;

    let session = if let Some(id) = &args.session {
        harness
            .list_sessions()
            .await?
            .into_iter()
            .find(|session| session.id == *id)
            .with_context(|| format!("session {id} does not exist"))?
    } else {
        harness
            .create_session(SessionOptions {
                cwd: args.cwd()?,
                title: prompt.chars().take(72).collect(),
            })
            .await?
    };
    let mut last_sequence = 0;
    loop {
        let page = harness.events(&session.id, last_sequence, 1024).await?;
        if page.is_empty() {
            break;
        }
        if let Some(last) = page.last() {
            last_sequence = last.sequence;
        }
    }
    let mut events = harness.subscribe();
    let mut run = harness.start(&session.id, prompt).await?;
    let mut completion = Box::pin(run.wait());
    let mut signal = Box::pin(tokio::signal::ctrl_c());
    let mut printed_text = false;
    loop {
        tokio::select! {
            event = events.recv() => match event {
                Ok(event) if event.session_id == session.id && event.sequence > last_sequence => {
                    last_sequence = event.sequence;
                    print_event(&event, json, &mut printed_text)?;
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                    loop {
                        let page = harness.events(&session.id, last_sequence, 1024).await?;
                        if page.is_empty() { break; }
                        for event in page {
                            last_sequence = event.sequence;
                            print_event(&event, json, &mut printed_text)?;
                        }
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                _ => {}
            },
            result = &mut completion => {
                let summary = result?;
                // Replay closes the race between a fast run and its last live hint.
                loop {
                    let page = harness.events(&session.id, last_sequence, 1024).await?;
                    if page.is_empty() { break; }
                    for event in page {
                        last_sequence = event.sequence;
                        print_event(&event, json, &mut printed_text)?;
                    }
                }
                if !json {
                    if !printed_text && !summary.text.is_empty() {
                        print!("{}", render::safe_text(&summary.text));
                    }
                    println!();
                    if let Some(error) = summary.error { eprintln!("{}", render::safe_text(&error)); }
                }
                std::io::stdout().flush()?;
                return Ok(matches!(summary.status, RunStatus::Completed));
            }
            _ = &mut signal => {
                harness.cancel(&session.id).await?;
                signal = Box::pin(tokio::signal::ctrl_c());
            }
        }
    }
    bail!("event stream closed before the run completed")
}

fn print_event(event: &ejm_harness::Event, json: bool, printed_text: &mut bool) -> Result<()> {
    use std::io::Write;
    if json {
        println!("{}", serde_json::to_string(event)?);
    } else {
        match &event.kind {
            EventKind::TextDelta { text } => {
                print!("{}", render::safe_text(text));
                std::io::stdout().flush()?;
                *printed_text = true;
            }
            EventKind::ToolQueued { call } => {
                eprintln!("[queued] {}", render::safe_text(&call.name));
            }
            EventKind::ToolStarted { call } => {
                eprintln!("[running] {}", render::safe_text(&call.name));
            }
            EventKind::ToolFinished { output, .. } if output.is_error => {
                eprintln!("[tool failed] {}", render::safe_text(&output.content));
            }
            _ => {}
        }
    }
    Ok(())
}

fn doctor(args: &Args) -> Result<bool> {
    let limits = args.resources();
    let governor = Arc::new(ejm_harness_exec::Governor::new(limits.clone())?);
    let executor = ejm_harness_exec::Executor::new(governor, args.state_dir().join("artifacts"))?;
    let cwd = args.cwd()?;
    let registry_rejected = print_registry(args);
    println!("ejm-cli {}", env!("CARGO_PKG_VERSION"));
    println!("Workspace: {}", cwd.display());
    println!("State: {}", args.state_dir().display());
    println!("Model: {}", render::safe_text(args.provider_label()));
    println!(
        "Endpoint: {}",
        match args.provider {
            ProviderKind::Chatgpt => "official ChatGPT Responses endpoint".into(),
            ProviderKind::Responses => "official OpenAI Responses endpoint".into(),
            ProviderKind::Openai | ProviderKind::Anthropic =>
                render::safe_text(args.api_base_url()),
        }
    );
    println!(
        "Provider credentials: {}",
        if args.replay || args.replay_file.is_some() {
            "offline replay"
        } else if args.provider == ProviderKind::Chatgpt {
            "saved browser sign-in; inspect with ejm-cli accounts"
        } else if args.api_key().is_some() {
            "configured (value hidden)"
        } else {
            "missing; use --replay to try ejm-cli offline"
        }
    );
    println!("Grants: {}", args.grants());
    println!("Resources: {}", serde_json::to_string_pretty(&limits)?);
    println!(
        "Executor capabilities: {}",
        serde_json::to_string_pretty(&executor.capabilities())?
    );
    println!("Native execution uses the current user. It is not a security sandbox.");
    if cfg!(target_os = "macos") {
        println!(
            "macOS: aggregate RAM use is monitored; no hard process-tree RAM ceiling is available."
        );
    }
    Ok(!registry_rejected)
}

fn print_registry(args: &Args) -> bool {
    let rejected = match args.registry_choice() {
        RegistryChoice::Selected { origin, source } => {
            println!(
                "Registry: {} ({})",
                display_registry_url(&origin),
                source.label()
            );
            false
        }
        RegistryChoice::Rejected {
            raw,
            source,
            message,
        } => {
            println!(
                "Registry: rejected ({} from {}): {}",
                display_registry_url(&raw),
                source.label(),
                render::safe_text(&message)
            );
            true
        }
    };
    match packages::retired_registry_session_notice(&args.state_dir().join("packages")) {
        Ok(Some(message)) => println!("{}", render::safe_text(&message)),
        Ok(None) => {}
        Err(error) => println!(
            "Registry session file: {}",
            render::safe_text(&format!("{error:#}"))
        ),
    }
    rejected
}
