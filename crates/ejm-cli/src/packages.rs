use std::{path::PathBuf, sync::Arc};

use anyhow::{Context, Result, bail, ensure};
use clap::Subcommand;
use ejm_harness_exec::Governor;
use ejm_harness_packages::{
    PackageGrants, PackageManager, RegistryClient, install_source, load_registry_token,
    pack_directory, save_registry_token,
};
use tokio_util::sync::CancellationToken;

use crate::{
    args::{
        Args, RegistryChoice, RegistrySource, display_registry_origin, host_is_meetgerry,
        registry_override_error, retired_registry_message,
    },
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
    /// Delete a local registry session saved for the retired meetgerry.ai host. Sends nothing.
    DiscardRetiredSession,
}

pub fn manager(args: &Args) -> Result<PackageManager> {
    PackageManager::new(args.state_dir().join("packages"))
}

pub async fn run(args: &Args, command: &PackageCommand) -> Result<bool> {
    let manager = manager(args)?;
    if matches!(command, PackageCommand::DiscardRetiredSession) {
        discard_retired_session(manager.root())?;
        return Ok(true);
    }
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
            safe_text(display_registry_origin(&origin)),
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
        PackageCommand::DiscardRetiredSession => {}
    }
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
    if legacy_origin(manager.root())?.is_some_and(|origin| origin_is_retired(&origin)) {
        bail!("{}", retired_registry_message());
    }
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

const LEGACY_SESSION: &str = "registry-session.json";
const MAX_LEGACY_SESSION: usize = 8192;

async fn logout(manager: &PackageManager, registry: &RegistryClient) -> Result<()> {
    let origin = registry.origin();
    match legacy_origin(manager.root())? {
        Some(saved) if saved == origin => {
            let token = load_registry_token(manager.root(), origin)?;
            let revoked = registry.logout(&token).await;
            match std::fs::remove_file(manager.root().join(LEGACY_SESSION)) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error).context("remove registry session"),
            }
            revoked.context("local session removed; remote revocation failed")?;
            println!("Registry session revoked and removed.");
        }
        _ => {
            println!(
                "No registry session for {}.",
                safe_text(display_registry_origin(origin))
            );
        }
    }
    Ok(())
}

fn require_session(
    manager: &PackageManager,
    origin: &str,
) -> Result<ejm_harness_auth::SecretString> {
    match legacy_origin(manager.root())? {
        Some(saved) if saved == origin => load_registry_token(manager.root(), origin),
        _ => bail!(
            "no registry session for {}",
            display_registry_origin(origin)
        ),
    }
}

fn discard_retired_session(root: &std::path::Path) -> Result<()> {
    match legacy_origin(root)? {
        Some(origin) if origin_is_retired(&origin) => {
            std::fs::remove_file(root.join(LEGACY_SESSION))
                .context("remove retired registry session")?;
            println!(
                "Discarded the local session for the retired meetgerry.ai registry. No request was sent."
            );
        }
        Some(_) => {
            println!("Saved registry session is not for a meetgerry.ai host. Left it in place.");
        }
        None => println!("No leftover registry session file."),
    }
    Ok(())
}

/// True when a private leftover session file belongs to meetgerry.ai or a subdomain.
///
/// # Errors
///
/// Returns an error when the session file exists but cannot be read safely.
pub fn retired_session_present(packages_root: &std::path::Path) -> Result<bool> {
    Ok(legacy_origin(packages_root)?.is_some_and(|origin| origin_is_retired(&origin)))
}

fn origin_is_retired(origin: &str) -> bool {
    url::Url::parse(origin)
        .ok()
        .and_then(|url| url.host().map(|host| host_is_meetgerry(&host)))
        .unwrap_or(false)
}

fn legacy_origin(root: &std::path::Path) -> Result<Option<String>> {
    let path = root.join(LEGACY_SESSION);
    let metadata = match std::fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error).context("inspect registry session"),
    };
    ensure!(
        metadata.is_file(),
        "registry session must be a regular file"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        ensure!(
            metadata.permissions().mode().trailing_zeros() >= 6,
            "registry session file must be private"
        );
    }
    let bytes = std::fs::read(&path).context("read registry session")?;
    ensure!(
        bytes.len() <= MAX_LEGACY_SESSION,
        "registry session exceeds byte limit"
    );
    let value: serde_json::Value =
        serde_json::from_slice(&bytes).context("invalid registry session")?;
    let origin = value
        .get("origin")
        .and_then(serde_json::Value::as_str)
        .context("invalid registry session")?
        .to_owned();
    Ok(Some(origin))
}
