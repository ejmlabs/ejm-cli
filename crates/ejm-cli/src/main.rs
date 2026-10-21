mod app;
mod args;
mod auth;
mod browser;
mod capacity;
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
mod interaction;
mod models;
mod packages;
mod remote;
mod render;
mod sessions;
mod terminal;
mod terminal_input;
mod ui;
mod views;

use std::io::IsTerminal;

use anyhow::{Context, Result, bail, ensure};
use clap::Parser;
use ejm_harness::{
    CapacityContext, CapacityPolicy, EventKind, Harness, HarnessConfig, RunStatus, SessionOptions,
};
use ejm_harness_coding::{CodingConfig, tools};
use tokio_util::sync::CancellationToken;

use args::{Args, Command, ProviderKind};

fn main() -> std::process::ExitCode {
    if let Some(result) = ejm_harness::ejm_harness_exec::native::dispatch_helper() {
        return match result {
            Ok(code) => code,
            Err(error) => {
                eprintln!("ejm-cli: {}", render::safe_text(&format!("{error:#}")));
                std::process::ExitCode::from(1)
            }
        };
    }
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
        Some(Command::Package { command, registry }) => {
            return packages::run(&args, command, registry.as_deref()).await;
        }
        Some(Command::Doctor) => return doctor(&args).await,
        Some(Command::Capacity {
            command: args::CapacityCommand::Reset,
        }) => {
            let count = tokio::task::spawn_blocking(ejm_harness::reset_capacity_learning).await??;
            println!(
                "Reset {count} local learned capacity profiles. Active reservations and provider rate budgets remain valid."
            );
            return Ok(true);
        }
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
    if matches!(mode, AgentMode::Turn { .. }) {
        models::validate(&args)?;
    }
    let startup_cancel = CancellationToken::new();
    let mut shutdown = ui::termination_signal();
    let prepared =
        prepare_capacity(&args, state_dir.clone(), &startup_cancel, &mut shutdown).await?;
    let extensions = {
        let loading = async {
            if matches!(mode, AgentMode::Sessions { .. }) {
                extensions::Extensions::empty()
            } else {
                extensions::Extensions::load(
                    &args,
                    prepared.as_ref().map(|(_, capacity)| capacity.clone()),
                    startup_cancel.clone(),
                )
                .await
            }
        };
        tokio::pin!(loading);
        tokio::select! {
            biased;
            () = &mut shutdown => {
                startup_cancel.cancel();
                let cleanup = match loading.await {
                    Ok(extensions) => extensions.shutdown().await,
                    Err(error) => Err(error),
                };
                return cancelled_startup(cleanup);
            }
            result = &mut loading => result?,
        }
    };
    let prepared_capacity = prepared.as_ref().map(|(_, capacity)| capacity.clone());
    let result = run_agent(
        &mut args,
        state_dir,
        mode,
        &extensions,
        prepared,
        startup_cancel,
        shutdown,
    )
    .await;
    let shutdown = extensions.shutdown().await;
    // Keep the host owner through both Harness and package-runtime cleanup.
    drop(prepared_capacity);
    complete_cleanup(result, shutdown, "package shutdown")
}

async fn prepare_capacity(
    args: &Args,
    state_dir: std::path::PathBuf,
    cancel: &CancellationToken,
    shutdown: &mut ui::TerminationSignal,
) -> Result<Option<(HarnessConfig, CapacityContext)>> {
    if args.capacity.is_none() {
        return Ok(None);
    }
    let config = harness_config(args, state_dir);
    let policy = capacity_policy(args);
    let prepare = async {
        ensure!(!cancel.is_cancelled(), "startup cancelled");
        tokio::task::spawn_blocking(move || {
            let mut config = config;
            let capacity = CapacityContext::prepare(&mut config, policy)?;
            Ok::<_, anyhow::Error>((config, capacity))
        })
        .await?
    };
    tokio::pin!(prepare);
    tokio::select! {
        biased;
        () = shutdown => {
            cancel.cancel();
            // Registration must finish before its owner can be cleaned up safely.
            cancelled_startup(prepare.await)
        }
        result = &mut prepare => result.map(Some),
    }
}

fn cancelled_startup<T, U>(result: Result<T>) -> Result<U> {
    match result {
        Ok(_) => bail!("startup cancelled"),
        Err(error) => Err(error.context("startup cancelled")),
    }
}

fn complete_cleanup<T>(result: Result<T>, cleanup: Result<()>, owner: &'static str) -> Result<T> {
    match (result, cleanup) {
        (result, Ok(())) => result,
        (Ok(_), Err(error)) => Err(error.context(owner)),
        (Err(error), Err(cleanup)) => {
            Err(error.context(format!("{owner} also failed: {cleanup:#}")))
        }
    }
}

fn harness_config(args: &Args, state_dir: std::path::PathBuf) -> HarnessConfig {
    HarnessConfig {
        state_dir,
        max_sessions: args.max_sessions.unwrap_or(100),
        max_model_requests: args.model_concurrency.unwrap_or(8),
        resources: args.resources(),
        max_steps: args.max_steps,
        run_timeout_ms: args.run_timeout_ms,
        ..HarnessConfig::default()
    }
}

fn capacity_policy(args: &Args) -> CapacityPolicy {
    CapacityPolicy {
        mode: match args.capacity.unwrap_or(args::CapacityMode::Fixed) {
            args::CapacityMode::Auto => ejm_harness::CapacityMode::Auto,
            args::CapacityMode::Fixed => ejm_harness::CapacityMode::Fixed,
        },
        max_sessions: args.max_sessions,
        max_model_requests: args.model_concurrency,
        max_jobs: args.tool_concurrency,
        max_memory_bytes: args.tool_memory_mib.map(|value| value * 1024 * 1024),
        ..CapacityPolicy::default()
    }
}

async fn run_agent(
    args: &mut Args,
    state_dir: std::path::PathBuf,
    mode: AgentMode,
    extensions: &extensions::Extensions,
    prepared: Option<(HarnessConfig, CapacityContext)>,
    startup_cancel: CancellationToken,
    mut shutdown: ui::TerminationSignal,
) -> Result<bool> {
    if matches!(mode, AgentMode::Turn { .. }) {
        models::validate(args)?;
    }
    let (config, capacity) = prepared.map_or_else(
        || (harness_config(args, state_dir.clone()), None),
        |(config, capacity)| (config, Some(capacity)),
    );
    let mut coding = tools(CodingConfig {
        allow_write: args.allow_write,
        allow_shell: args.allow_shell,
        artifact_dir: state_dir.join("artifacts"),
    })?;
    coding.extend(extensions.tools()?);
    let harness = if let Some(capacity) = capacity {
        Harness::with_capacity(config, None, coding, extensions.hooks()?, capacity)?
    } else {
        Harness::with_optional_provider(config, None, coding, extensions.hooks()?)?
    };
    let result = async {
        let mut executor = ejm_harness_exec::Executor::new(harness.governor(), state_dir.join("artifacts"))?;
        if let Some(owner) = harness.native_owner() {
            executor = executor.with_native_owner(owner);
        }
        if matches!(mode, AgentMode::Turn { .. }) {
            let mut candidate = args.clone();
            {
                let preparing = async {
                    let provider = models::prepare(&mut candidate, extensions, &executor, startup_cancel.child_token()).await?;
                    ensure!(!startup_cancel.is_cancelled(), "startup cancelled");
                    harness
                        .configure_provider(Some(provider), &models::binding(&candidate)?)
                        .await
                };
                tokio::pin!(preparing);
                tokio::select! {
                    biased;
                    () = &mut shutdown => {
                        startup_cancel.cancel();
                        return cancelled_startup(preparing.await);
                    }
                    result = &mut preparing => result?,
                }
            }
            *args = candidate;
        }
        match mode {
        AgentMode::Sessions { json } => {
            let mut sessions = Vec::new();
            let mut cursor = None;
            loop {
                let page = harness.sessions_page(ejm_harness::SessionQuery { cursor, ..ejm_harness::SessionQuery::default() }).await?;
                sessions.extend(page.sessions);
                cursor = page.next;
                if cursor.is_none() { break; }
            }
            if json {
                println!("{}", serde_json::to_string(&sessions)?);
            } else if sessions.is_empty() {
                println!("No sessions yet. Start with: ejm-cli");
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
        AgentMode::Turn { prompt, json } => run_once(&harness, args, prompt, json, shutdown).await,
        AgentMode::Interactive => {
            if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
                bail!(
                    "the TUI needs a terminal; configure a provider and use `ejm-cli run \"Hello\"` for plain output"
                );
            }
            ui::run(harness.clone(), args, extensions, executor, shutdown)
                .await
                .map(|()| true)
        }
        }
    }
    .await;
    let shutdown = harness.shutdown().await;
    complete_cleanup(result, shutdown, "harness shutdown")
}

async fn models(args: &Args, json: bool) -> Result<bool> {
    let catalog = models::catalog(args, CancellationToken::new()).await?;
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

async fn run_once(
    harness: &Harness,
    args: &Args,
    prompt: String,
    json: bool,
    mut signal: ui::TerminationSignal,
) -> Result<bool> {
    use std::io::Write;

    let session = if let Some(id) = &args.session {
        harness.get_session(id).await?
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
            () = &mut signal => {
                harness.cancel(&session.id).await?;
                signal = ui::termination_signal();
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

async fn doctor(args: &Args) -> Result<bool> {
    let mut config = harness_config(args, args.state_dir());
    let capacity = if args.capacity.is_some() {
        let policy = capacity_policy(args);
        let prepared = tokio::task::spawn_blocking(move || {
            let capacity = CapacityContext::prepare(&mut config, policy)?;
            Ok::<_, anyhow::Error>((config, capacity))
        })
        .await??;
        config = prepared.0;
        Some(prepared.1)
    } else {
        None
    };
    let limits = config.resources;
    let governor = match &capacity {
        Some(capacity) => capacity.governor(),
        None => std::sync::Arc::new(ejm_harness_exec::Governor::new(limits.clone())?),
    };
    let mut executor =
        ejm_harness_exec::Executor::new(governor, args.state_dir().join("artifacts"))?;
    if let Some(capacity) = &capacity {
        executor = executor.with_native_owner(capacity.native_owner());
    }
    let cwd = args.cwd()?;
    println!("ejm-cli {}", env!("CARGO_PKG_VERSION"));
    println!("Workspace: {}", cwd.display());
    println!("State: {}", args.state_dir().display());
    println!("Model: {}", render::safe_text(args.provider_label()));
    println!(
        "Endpoint: {}",
        match args.provider {
            ProviderKind::Chatgpt => "official ChatGPT Responses endpoint".into(),
            ProviderKind::Responses => "official OpenAI Responses endpoint".into(),
            ProviderKind::Claude => "official Claude Code client subscription".into(),
            ProviderKind::Openai | ProviderKind::Anthropic =>
                render::safe_text(args.api_base_url()),
        }
    );
    println!(
        "Provider credentials: {}",
        if args.provider == ProviderKind::Chatgpt {
            "saved browser sign-in; inspect with ejm-cli accounts"
        } else if args.provider == ProviderKind::Claude {
            "owned by the official Claude client; inspect with ejm-cli accounts"
        } else if args.api_key().is_some() {
            "configured (value hidden)"
        } else {
            "missing; set your provider key or use ejm-cli login chatgpt or ejm-cli login claude"
        }
    );
    println!("Grants: {}", args.grants());
    println!("Resources: {}", serde_json::to_string_pretty(&limits)?);
    if let Some(capacity) = &capacity {
        println!("{}", capacity::describe(&capacity.snapshot()));
    } else {
        println!("Capacity: legacy fixed process limits; host coordination is opt-in");
    }
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
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleanup_failure_preserves_the_operation_cause_and_cleanup_diagnostic() -> Result<()> {
        let error = complete_cleanup::<()>(
            Err(std::io::Error::other("fixture import rejected").into()),
            Err(anyhow::anyhow!("native cleanup remains uncertain")),
            "package shutdown",
        )
        .err()
        .context("combined failure was lost")?;
        assert!(error.downcast_ref::<std::io::Error>().is_some());
        let diagnostic = format!("{error:#}");
        assert!(diagnostic.contains("fixture import rejected"));
        assert!(diagnostic.contains("native cleanup remains uncertain"));
        Ok(())
    }

    #[tokio::test]
    async fn plain_run_still_fails_before_creating_state_without_credentials() -> Result<()> {
        let state = tempfile::tempdir()?;
        let harness_state = state.path().join("sessions");
        let mut args = Args::try_parse_from(["ejm-cli", "run", "hello"])?;
        args.base_url = None;
        args.model = None;
        // Environment variable names cannot contain '=', so this fixture cannot use a real key.
        args.api_key_env = "INVALID=ENVIRONMENT_NAME".into();
        let extensions = extensions::Extensions::empty()?;
        let error = run_agent(
            &mut args,
            harness_state.clone(),
            AgentMode::Turn {
                prompt: "hello".into(),
                json: false,
            },
            &extensions,
            None,
            CancellationToken::new(),
            ui::termination_signal(),
        )
        .await
        .err()
        .context("plain run accepted missing credentials")?;
        assert!(error.to_string().contains("use /login in the TUI"));
        assert!(!harness_state.exists());
        extensions.shutdown().await?;
        Ok(())
    }

    #[tokio::test]
    async fn session_inspection_ignores_unusable_provider_configuration() -> Result<()> {
        let state = tempfile::tempdir()?;
        let mut args = Args::try_parse_from(["ejm-cli", "sessions"])?;
        args.package_provider = Some("@fixture/missing:provider".into());
        args.api_key_env = "INVALID=ENVIRONMENT_NAME".into();
        let extensions = extensions::Extensions::empty()?;
        assert!(
            run_agent(
                &mut args,
                state.path().join("sessions"),
                AgentMode::Sessions { json: true },
                &extensions,
                None,
                CancellationToken::new(),
                ui::termination_signal(),
            )
            .await?
        );
        extensions.shutdown().await?;
        Ok(())
    }
}
