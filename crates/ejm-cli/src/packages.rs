use std::{path::PathBuf, sync::Arc};

use anyhow::{Context, Result, bail, ensure};
use clap::Subcommand;
use ejm_harness_exec::Governor;
use ejm_harness_packages::{
    PackageGrants, PackageManager, RegistryClient, delete_registry_token, install_source,
    load_registry_token, migrate_registry_token, pack_directory, save_registry_token,
};
use tokio_util::sync::CancellationToken;

use crate::{
    args::{Args, RegistryChoice, RegistrySource, display_registry_url, registry_override_error},
    render::safe_text,
};

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
    MigrateCredentials {
        #[arg(
            long,
            required = true,
            help = "Confirm every older CLI and IDE registry credential writer has stopped"
        )]
        old_clients_stopped: bool,
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
    let registry = open_registry(args)?;
    dispatch(args, &manager, &registry, command).await?;
    Ok(true)
}

fn open_registry(args: &Args) -> Result<RegistryClient> {
    let choice = args.registry_choice();
    registry_override_error(&choice)?;
    let RegistryChoice::Selected { origin, source } = choice else {
        bail!("registry origin was rejected");
    };
    if source != RegistrySource::Default {
        println!(
            "Registry: {} ({})",
            safe_text(&display_registry_url(&origin)),
            source.label()
        );
    }
    RegistryClient::new(&origin)
}

async fn dispatch(
    args: &Args,
    manager: &PackageManager,
    registry: &RegistryClient,
    command: &PackageCommand,
) -> Result<()> {
    match command {
        PackageCommand::Search { query } => show(&registry.search(query, None).await?)?,
        PackageCommand::Info { name, version } => show(&registry.version(name, version).await?)?,
        PackageCommand::Install { source } => install(args, manager, registry, source).await?,
        PackageCommand::Update { name, version } => {
            ensure!(
                manager.list()?.iter().any(|record| &record.name == name),
                "package is not installed"
            );
            install(args, manager, registry, &format!("{name}@{version}")).await?;
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
                    providers: grant_provider.clone(),
                    commands: command.clone(),
                    widgets: grant_widget.clone(),
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
        PackageCommand::Login { no_browser } => login(manager, registry, *no_browser).await?,
        PackageCommand::MigrateCredentials {
            old_clients_stopped,
        } => migrate_credentials(manager, registry, *old_clients_stopped)?,
        PackageCommand::Logout => logout(manager, registry).await?,
        PackageCommand::Whoami => {
            show(
                &registry
                    .me(&require_session(manager, registry.origin())?)
                    .await?,
            )?;
        }
        PackageCommand::Publisher {
            handle,
            display_name,
        } => show(
            &registry
                .claim_publisher(
                    handle,
                    display_name,
                    &require_session(manager, registry.origin())?,
                )
                .await?,
        )?,
        PackageCommand::Publish { directory } => {
            let bytes = pack_directory(directory)?;
            let token = require_session(manager, registry.origin())?;
            show(&registry.publish(bytes, &token).await?)?;
        }
    }
    Ok(())
}

fn finish_logout(revoked: Result<()>, clear: impl FnOnce() -> Result<()>) -> Result<()> {
    clear()?;
    revoked.context("local session removed; remote revocation failed")
}

async fn logout(manager: &PackageManager, registry: &RegistryClient) -> Result<()> {
    let token = require_session(manager, registry.origin())?;
    let revoked = registry.logout(&token).await;
    finish_logout(revoked, || {
        delete_registry_token(manager.root(), registry.origin())
    })?;
    println!("Registry session revoked and removed.");
    Ok(())
}

fn migrate_credentials(
    manager: &PackageManager,
    registry: &RegistryClient,
    old_clients_stopped: bool,
) -> Result<()> {
    let retired = migrate_registry_token(manager.root(), registry.origin(), old_clients_stopped)?;
    println!(
        "{}",
        if retired {
            "Legacy registry session retired."
        } else {
            "No legacy registry session found."
        }
    );
    Ok(())
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
        Some(registry),
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

fn require_session(
    manager: &PackageManager,
    origin: &str,
) -> Result<ejm_harness_auth::SecretString> {
    load_registry_token(manager.root(), origin)
        .with_context(|| format!("no registry session for {}", display_registry_url(origin)))
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, ffi::OsStr};

    use clap::Parser;

    use super::*;
    use crate::args::Command;

    #[test]
    fn failed_revocation_still_clears_local_session() {
        let cleared = Cell::new(false);
        let result = finish_logout(Err(anyhow::anyhow!("fixture revocation failure")), || {
            cleared.set(true);
            Ok(())
        });
        assert!(cleared.get());
        assert!(result.is_err());
    }

    #[test]
    fn local_clear_failure_prevents_logout_success() {
        let result = finish_logout(Ok(()), || Err(anyhow::anyhow!("fixture storage failure")));
        assert!(result.is_err());
    }

    #[test]
    fn registry_migration_requires_confirmation_before_storage() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let state = directory.path().join("state");
        let result = Args::try_parse_from([
            OsStr::new("ejm-cli"),
            OsStr::new("--state-dir"),
            state.as_os_str(),
            OsStr::new("package"),
            OsStr::new("migrate-credentials"),
        ]);
        assert!(result.is_err());
        assert!(!state.exists());
        Ok(())
    }

    #[test]
    fn registry_migration_accepts_explicit_confirmation() -> Result<()> {
        let args = Args::try_parse_from([
            "ejm-cli",
            "package",
            "migrate-credentials",
            "--old-clients-stopped",
        ])?;
        assert!(matches!(
            args.command,
            Some(Command::Package {
                command: PackageCommand::MigrateCredentials {
                    old_clients_stopped: true
                }
            })
        ));
        Ok(())
    }
}
