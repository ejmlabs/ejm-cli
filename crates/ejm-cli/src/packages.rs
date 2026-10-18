use std::{path::PathBuf, sync::Arc};

use anyhow::{Context, Result, ensure};
use clap::Subcommand;
use ejm_harness_exec::Governor;
use ejm_harness_packages::{
    DEFAULT_REGISTRY, PackageGrants, PackageManager, RegistryClient, install_source,
    load_registry_token, pack_directory, save_registry_token,
};
use tokio_util::sync::CancellationToken;

use crate::{args::Args, render::safe_text};

#[derive(Debug, Subcommand)]
pub enum PackageCommand {
    Search {
        query: String,
    },
    Info {
        name: String,
        version: String,
    },
    Install {
        source: String,
    },
    Update {
        name: String,
        version: String,
    },
    List,
    Enable {
        name: String,
        #[arg(
            long,
            required = true,
            help = "Trust this exact version as native code running as you"
        )]
        trust: bool,
        #[arg(long, value_name = "EXPORT")]
        tool: Vec<String>,
        #[arg(long, value_name = "EVENT", value_parser = ["run_start", "before_model", "before_tool", "after_tool", "run_end"])]
        hook: Vec<String>,
        #[arg(long, value_name = "EXPORT")]
        provider: Vec<String>,
        #[arg(long, value_name = "EXPORT")]
        command: Vec<String>,
        #[arg(long, value_name = "EXPORT")]
        widget: Vec<String>,
    },
    Disable {
        name: String,
    },
    Remove {
        name: String,
    },
    Pack {
        #[arg(default_value = ".")]
        directory: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    Login {
        #[arg(long)]
        no_browser: bool,
    },
    Logout,
    Whoami,
    Publisher {
        handle: String,
        #[arg(long)]
        display_name: String,
    },
    Publish {
        #[arg(default_value = ".")]
        directory: PathBuf,
    },
}

pub fn manager(args: &Args) -> Result<PackageManager> {
    PackageManager::new(args.state_dir().join("packages"))
}

pub async fn run(args: &Args, command: &PackageCommand) -> Result<bool> {
    let manager = manager(args)?;
    let registry = RegistryClient::new(DEFAULT_REGISTRY)?;
    match command {
        PackageCommand::Search { query } => show(&registry.search(query, None).await?)?,
        PackageCommand::Info { name, version } => show(&registry.version(name, version).await?)?,
        PackageCommand::Install { source } => install(args, &manager, &registry, source).await?,
        PackageCommand::Update { name, version } => {
            ensure!(
                manager.list()?.iter().any(|record| &record.name == name),
                "package is not installed"
            );
            install(args, &manager, &registry, &format!("{name}@{version}")).await?;
        }
        PackageCommand::List => show(&manager.list()?)?,
        PackageCommand::Enable {
            name,
            trust,
            tool,
            hook,
            provider,
            command,
            widget,
        } => {
            ensure!(*trust, "--trust is required for native extensions");
            let hooks = hook
                .iter()
                .map(|name| serde_json::from_value(serde_json::Value::String(name.clone())))
                .collect::<std::result::Result<Vec<_>, _>>()?;
            let record = manager.enable(
                name,
                PackageGrants {
                    tools: tool.clone(),
                    hooks,
                    providers: provider.clone(),
                    commands: command.clone(),
                    widgets: widget.clone(),
                },
            )?;
            show(&record)?;
            println!(
                "Trusted for future sessions. Native packages run as your user; export grants control registration."
            );
        }
        PackageCommand::Disable { name } => {
            manager.disable(name)?;
            println!("Disabled for future sessions. Restart an active CLI to unload it.");
        }
        PackageCommand::Remove { name } => {
            manager.remove(name)?;
            println!("Removed the registration. Active sessions retain their snapshot until exit.");
        }
        PackageCommand::Pack { directory, output } => {
            use std::io::Write;
            let bytes = pack_directory(directory)?;
            let mut file = std::fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(output)
                .context("create new package archive")?;
            file.write_all(&bytes)?;
            println!(
                "{} bytes: {}",
                bytes.len(),
                safe_text(&output.display().to_string())
            );
        }
        PackageCommand::Login { no_browser } => login(&manager, &registry, *no_browser).await?,
        PackageCommand::Logout => {
            let token = load_registry_token(manager.root(), registry.origin())?;
            let revoked = registry.logout(&token).await;
            std::fs::remove_file(manager.root().join("registry-session.json"))?;
            revoked.context("local session removed; remote revocation failed")?;
            println!("Registry session revoked and removed.");
        }
        PackageCommand::Whoami => show(
            &registry
                .me(&load_registry_token(manager.root(), registry.origin())?)
                .await?,
        )?,
        PackageCommand::Publisher {
            handle,
            display_name,
        } => show(
            &registry
                .claim_publisher(
                    handle,
                    display_name,
                    &load_registry_token(manager.root(), registry.origin())?,
                )
                .await?,
        )?,
        PackageCommand::Publish { directory } => {
            let bytes = pack_directory(directory)?;
            let token = load_registry_token(manager.root(), registry.origin())?;
            show(&registry.publish(bytes, &token).await?)?;
        }
    }
    Ok(true)
}

async fn install(
    args: &Args,
    manager: &PackageManager,
    registry: &RegistryClient,
    source: &str,
) -> Result<()> {
    let governor = Arc::new(Governor::new(args.resources())?);
    let record = install_source(
        manager,
        source,
        registry,
        &governor,
        CancellationToken::new(),
    )
    .await?;
    show(&record)?;
    println!(
        "Installed and disabled. Inspect it, then enable the exact version with --trust and named export grants."
    );
    Ok(())
}

async fn login(
    manager: &PackageManager,
    registry: &RegistryClient,
    no_browser: bool,
) -> Result<()> {
    let code = registry.device_code().await?;
    println!(
        "Open {} and enter {}",
        safe_text(&code.verification_uri),
        safe_text(&code.user_code)
    );
    println!(
        "Approve only the ejm-cli request you just started. This grants a revocable registry account session."
    );
    if !no_browser {
        crate::browser::launch(
            ejm_harness_auth::SecretString::new(&code.verification_uri),
            CancellationToken::new(),
        )
        .await?;
    }
    let token = registry
        .wait_for_login(&code, CancellationToken::new())
        .await?;
    save_registry_token(manager.root(), registry.origin(), &token)?;
    println!("Signed in to the package registry.");
    Ok(())
}

fn show(value: &impl serde::Serialize) -> Result<()> {
    println!("{}", safe_text(&serde_json::to_string_pretty(value)?));
    Ok(())
}
