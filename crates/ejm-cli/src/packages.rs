use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{Context, Result, ensure};
use clap::Subcommand;
use ejm_harness_exec::Governor;
use ejm_harness_packages::{
    PackageGrants, PackageManager, RegistryClient, delete_registry_token, install_source,
    load_registry_token, migrate_registry_token, pack_directory, save_registry_token,
};
use tokio_util::sync::CancellationToken;

use crate::{args::Args, render::safe_text};

#[derive(Clone, Debug, Subcommand)]
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
        grant_provider: Vec<String>,
        #[arg(long, value_name = "EXPORT")]
        command: Vec<String>,
        #[arg(long, value_name = "EXPORT")]
        grant_widget: Vec<String>,
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
    /// Import legacy registry credentials after stopping every older CLI or IDE.
    MigrateCredentials {
        #[arg(
            long,
            required = true,
            help = "Confirm all older registry clients have stopped"
        )]
        old_clients_stopped: bool,
    },
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

pub async fn run(args: &Args, command: &PackageCommand, origin: Option<&str>) -> Result<bool> {
    let registry = origin.map(RegistryClient::new).transpose()?;
    let required_registry = || {
        registry
            .as_ref()
            .context("set --registry ORIGIN or EJM_REGISTRY_URL for registry operations")
    };
    let manager = manager(args)?;
    match command {
        PackageCommand::Search { query } => show(&required_registry()?.search(query, None).await?)?,
        PackageCommand::Info { name, version } => {
            show(&required_registry()?.version(name, version).await?)?;
        }
        PackageCommand::Install { source } => {
            if source.starts_with('@') {
                required_registry()?;
            }
            install(args, &manager, registry.as_ref(), source).await?;
        }
        PackageCommand::Update { name, version } => {
            let registry = required_registry()?;
            ensure!(
                manager.list()?.iter().any(|record| &record.name == name),
                "package is not installed"
            );
            install(args, &manager, Some(registry), &format!("{name}@{version}")).await?;
        }
        PackageCommand::List => show(&manager.list()?)?,
        PackageCommand::Enable {
            name,
            trust,
            tool,
            hook,
            grant_provider,
            command,
            grant_widget,
        } => trust_package(
            &manager,
            name,
            *trust,
            PackageGrants {
                tools: tool.clone(),
                hooks: hook
                    .iter()
                    .map(|name| serde_json::from_value(serde_json::Value::String(name.clone())))
                    .collect::<std::result::Result<Vec<_>, _>>()?,
                providers: grant_provider.clone(),
                commands: command.clone(),
                widgets: grant_widget.clone(),
            },
        )?,
        PackageCommand::Disable { name } => {
            manager.disable(name)?;
            println!("Disabled for future sessions. Restart an active CLI to unload it.");
        }
        PackageCommand::Remove { name } => {
            manager.remove(name)?;
            println!("Removed the registration. Active sessions retain their snapshot until exit.");
        }
        PackageCommand::Pack { directory, output } => pack_archive(directory, output)?,
        PackageCommand::Login { no_browser } => {
            login(&manager, required_registry()?, *no_browser).await?;
        }
        PackageCommand::Logout => logout(&manager, required_registry()?).await?,
        PackageCommand::MigrateCredentials {
            old_clients_stopped,
        } => migrate_credentials(&manager, required_registry()?, *old_clients_stopped).await?,
        PackageCommand::Whoami => show(
            &required_registry()?
                .me(
                    &registry_credentials(&manager, required_registry()?, load_registry_token)
                        .await?,
                )
                .await?,
        )?,
        PackageCommand::Publisher {
            handle,
            display_name,
        } => show(
            &required_registry()?
                .claim_publisher(
                    handle,
                    display_name,
                    &registry_credentials(&manager, required_registry()?, load_registry_token)
                        .await?,
                )
                .await?,
        )?,
        PackageCommand::Publish { directory } => {
            let registry = required_registry()?;
            let bytes = pack_directory(directory)?;
            let token = registry_credentials(&manager, registry, load_registry_token).await?;
            show(&registry.publish(bytes, &token).await?)?;
        }
    }
    Ok(true)
}

fn trust_package(
    manager: &PackageManager,
    name: &str,
    trust: bool,
    grants: PackageGrants,
) -> Result<()> {
    ensure!(trust, "--trust is required for native extensions");
    show(&manager.enable(name, grants)?)?;
    println!(
        "Trusted for future sessions. Native packages run as your user; export grants control registration."
    );
    Ok(())
}

fn pack_archive(directory: &Path, output: &Path) -> Result<()> {
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
    Ok(())
}

async fn install(
    args: &Args,
    manager: &PackageManager,
    registry: Option<&RegistryClient>,
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
    registry_credentials(manager, registry, move |root, origin| {
        save_registry_token(root, origin, &token)
    })
    .await?;
    println!("Signed in to the package registry.");
    Ok(())
}

async fn logout(manager: &PackageManager, registry: &RegistryClient) -> Result<()> {
    let token = registry_credentials(manager, registry, load_registry_token).await?;
    let revoked = registry.logout(&token).await;
    registry_credentials(manager, registry, delete_registry_token).await?;
    revoked.context("local session removed; remote revocation failed")?;
    println!("Registry session revoked and removed.");
    Ok(())
}

async fn migrate_credentials(
    manager: &PackageManager,
    registry: &RegistryClient,
    stopped: bool,
) -> Result<()> {
    let cleaned = registry_credentials(manager, registry, move |root, origin| {
        migrate_registry_token(root, origin, stopped)
    })
    .await?;
    if cleaned {
        println!("Registry credentials migrated to the encrypted vault.");
    } else {
        println!(
            "No legacy registry session file was found. Use package login if no encrypted session is saved."
        );
    }
    Ok(())
}

async fn registry_credentials<T: Send + 'static>(
    manager: &PackageManager,
    registry: &RegistryClient,
    operation: impl FnOnce(&Path, &str) -> Result<T> + Send + 'static,
) -> Result<T> {
    let root = manager.root().to_path_buf();
    let origin = registry.origin().to_owned();
    tokio::task::spawn_blocking(move || operation(&root, &origin))
        .await
        .context("registry credential operation interrupted")?
}

fn show(value: &impl serde::Serialize) -> Result<()> {
    println!("{}", safe_text(&serde_json::to_string_pretty(value)?));
    Ok(())
}
